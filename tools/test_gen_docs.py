"""Tests of gen_docs.py: --check on the tree, drift in a temporary copy, unknown keys, exit codes.

Run: python -I tools/test_gen_docs.py. Every fixture is built in a temporary directory; nothing is
written inside the repository.
"""
import os
import re
import shutil
import subprocess
import sys
import tempfile
import json
import unittest
from pathlib import Path

HERE = str(Path(__file__).resolve().parent)
ROOT = os.path.dirname(HERE)
GEN = os.path.join(HERE, "gen_docs.py")
sys.path.insert(0, HERE)

import gen_docs  # noqa: E402


def run(*args):
    p = subprocess.run([sys.executable, "-I", GEN] + list(args), capture_output=True, text=True)
    return p.returncode, p.stdout, p.stderr


def read(path):
    with open(path, "rb") as h:
        return h.read().decode("utf-8")


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as h:
        h.write(text.encode("utf-8"))


class Scratch(unittest.TestCase):
    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = os.path.join(self._tmp.name, "copy")
        os.makedirs(self.root)
        shutil.copy(os.path.join(ROOT, "README.md"), self.root)
        shutil.copytree(os.path.join(ROOT, "docs"), os.path.join(self.root, "docs"))
        if os.path.isdir(os.path.join(ROOT, "parity")):
            shutil.copytree(os.path.join(ROOT, "parity"), os.path.join(self.root, "parity"))

    def check(self):
        return run("--check", "--root", self.root)

    def page_with_block(self):
        """(relative path, text) of a page that holds a generated block."""
        for rel in ["README.md"] + ["docs/" + f for f in sorted(os.listdir(os.path.join(self.root, "docs"))) if f.endswith(".md")]:
            text = read(os.path.join(self.root, rel))
            if re.search(r"<!--gen:[A-Za-z0-9_.-]+-->", text):
                return rel, text
        self.fail("no page holds a generated block")


class OnTheTree(unittest.TestCase):
    def test_check_passes_on_the_tree(self):
        code, out, err = run("--check")
        self.assertEqual(code, 0, out + err)
        self.assertRegex(out, r"0 drifted, 0 unknown keys")

    def test_usage_error_exits_2(self):
        code, _, err = run("--no-such-option")
        self.assertEqual(code, 2)
        self.assertIn("usage", err)


class Drift(Scratch):
    def test_copy_passes(self):
        code, out, err = self.check()
        self.assertEqual(code, 0, out + err)

    def test_hand_edited_block_fails(self):
        rel, text = self.page_with_block()
        m = re.search(r"(<!--gen:([A-Za-z0-9_.-]+)-->\r?\n)(.*?)(<!--/gen:\2-->)", text, re.S)
        edited = text[:m.start(3)] + "hand edited line\n" + text[m.start(3):]
        write(os.path.join(self.root, rel), edited)
        code, out, _ = self.check()
        self.assertEqual(code, 1)
        self.assertIn(rel, out)
        self.assertRegex(out, r"[1-9]\d* drifted")

    def test_hand_edited_figure_fails(self):
        rel = "README.md"
        text = read(os.path.join(self.root, rel))
        m = re.search(r"<!--f:([A-Za-z0-9_.-]+)-->(.*?)<!--/f-->", text)
        if m is None:
            self.skipTest("README.md holds no generated figure")
        edited = text[:m.start(2)] + m.group(2) + " plus a hand edit" + text[m.end(2):]
        write(os.path.join(self.root, rel), edited)
        code, out, _ = self.check()
        self.assertEqual(code, 1)
        self.assertIn(rel, out)

    def test_write_mode_repairs_drift(self):
        rel, text = self.page_with_block()
        m = re.search(r"(<!--gen:([A-Za-z0-9_.-]+)-->\r?\n)(.*?)(<!--/gen:\2-->)", text, re.S)
        write(os.path.join(self.root, rel), text[:m.start(3)] + "hand edited line\n" + text[m.start(3):])
        self.assertEqual(self.check()[0], 1)
        code, out, _ = run("--root", self.root)
        self.assertEqual(code, 0, out)
        self.assertEqual(self.check()[0], 0)
        self.assertEqual(read(os.path.join(self.root, rel)), text)


class UnknownKeys(Scratch):
    def test_unknown_figure_key_counted(self):
        path = os.path.join(self.root, "README.md")
        write(path, read(path) + "\nFigure: <!--f:no_such_key-->1<!--/f-->\n")
        code, out, _ = self.check()
        self.assertEqual(code, 1)
        self.assertIn("1 unknown keys", out)
        self.assertIn("figure no_such_key", out)

    def test_unknown_block_key_counted(self):
        path = os.path.join(self.root, "README.md")
        write(path, read(path) + "\n<!--gen:no_such_block-->\nx\n<!--/gen:no_such_block-->\n")
        code, out, _ = self.check()
        self.assertEqual(code, 1)
        self.assertIn("1 unknown keys", out)
        self.assertIn("block no_such_block", out)

    def test_two_unknown_keys_counted(self):
        path = os.path.join(self.root, "README.md")
        write(path, read(path) + "\n<!--f:a_key-->1<!--/f--> <!--f:b_key-->2<!--/f-->\n")
        code, out, _ = self.check()
        self.assertEqual(code, 1)
        self.assertIn("2 unknown keys", out)

    def test_write_mode_exits_1_on_unknown_key(self):
        path = os.path.join(self.root, "README.md")
        write(path, read(path) + "\n<!--f:a_key-->1<!--/f-->\n")
        code, out, _ = run("--root", self.root)
        self.assertEqual(code, 1)
        self.assertIn("1 unknown keys", out)


class BadResultFiles(Scratch):
    def put(self, text):
        write(os.path.join(self.root, "parity", "u1", "results", "summary.json"), text)

    def test_not_json_exits_3(self):
        self.put("{not json")
        code, out, err = self.check()
        self.assertEqual(code, 3)
        self.assertIn("gen_docs: error:", err)
        self.assertIn("summary.json", err)
        self.assertEqual(out, "")

    def test_missing_field_exits_3(self):
        self.put('{"unit": "u1"}')
        code, _, err = self.check()
        self.assertEqual(code, 3)
        self.assertIn("missing field", err)

    def test_unit_named_elsewhere_exits_3(self):
        fields = ["rung", "judge", "comparator", "cases", "values", "call_records", "mutants"]
        body = ", ".join('"%s": {}' % f for f in fields)
        self.put('{"unit": "other", %s}' % body)
        code, _, err = self.check()
        self.assertEqual(code, 3)
        self.assertIn("names another unit", err)


def survey(**over):
    """A survey capture comparison file with invented counts."""
    r = {"unit": "u-survey", "comparison": "survey capture comparison",
         "label": "never a result of record, never parity evidence", "every_row_has_a_counterpart": True,
         "members": {"blind": {"compared": 7, "equal": 7, "different": 0},
                     "word_selected": {"compared": 2, "equal": 1, "different": 1},
                     "informed": {"compared": 1234, "equal": 1234, "different": 0}},
         "members_not_compared": {"form": {"blind": 0, "word_selected": 0, "informed": 3}}}
    r.update(over)
    return r


SURVEY_REL = "parity/u-survey/results/survey_comparison.json"
SURVEY_TEXT = (
    "A survey capture comparison, never a result of record and never parity evidence, from "
    "`parity/u-survey/results/survey_comparison.json`. Every row on each side had its counterpart on the other side.\n"
    "\n"
    "| Members | Blind | Word selected | Informed |\n"
    "|---|---|---|---|\n"
    "| compared | 7 | 2 | 1,234 |\n"
    "| equal | 7 | 1 | 1,234 |\n"
    "| different | 0 | 1 | 0 |\n"
    "| not compared, reason form | 0 | 0 | 3 |")


class SurveyFile(unittest.TestCase):
    """A tree that holds only a survey capture comparison file, and the strict reading of that file."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.root = os.path.join(self._tmp.name, "copy")
        os.makedirs(self.root)

    def put(self, r, rel=SURVEY_REL):
        write(os.path.join(self.root, rel), r if isinstance(r, str) else json.dumps(r))

    def page(self):
        write(os.path.join(self.root, "docs", "p.md"),
              "".join("<!--gen:%s-->\n<!--/gen:%s-->\n" % (k, k) for k in ("units", "notes", "pairing", "label", "survey"))
              + "<!--f:status_line-->x<!--/f--> <!--f:units_total-->x<!--/f-->\n")

    def block(self, text, key):
        m = re.search(r"<!--gen:%s-->\n(.*?)\n<!--/gen:%s-->" % (key, key), text, re.S)
        return m.group(1)

    def test_only_the_survey_file(self):
        self.put(survey())
        self.page()
        code, out, err = run("--root", self.root)
        self.assertEqual(code, 0, out + err)
        facts = json.loads(read(os.path.join(self.root, "docs", "data", "facts.json")))
        self.assertEqual(facts["status_line"]["value"], "method and preparation only: 0 cases compared")
        self.assertEqual(facts["units_total"]["value"], "0")
        self.assertEqual(facts["result_files"]["value"], [])
        self.assertEqual(facts["survey_comparison_files"]["value"], [SURVEY_REL])
        text = read(os.path.join(self.root, "docs", "p.md"))
        self.assertEqual(self.block(text, "units"),
                         "No unit result file exists yet, so no unit has a row. A unit is Not started until its "
                         "`summary.json` is committed.")
        self.assertEqual(self.block(text, "notes"), "No unit result file exists yet, so there are no notes.")
        self.assertEqual(self.block(text, "pairing"), "No unit result file records an interface pairing or a calibration yet.")
        self.assertIn("[computed from 0 unit result files; re-run: none]", self.block(text, "label"))
        self.assertEqual(self.block(text, "survey"), SURVEY_TEXT)
        self.assertIn("<!--f:status_line-->method and preparation only: 0 cases compared<!--/f-->", text)
        self.assertIn("<!--f:units_total-->0<!--/f-->", text)
        code, out, err = run("--check", "--root", self.root)
        self.assertEqual(code, 0, out + err)
        self.assertIn("0 drifted", out)

    def test_no_survey_file(self):
        self.assertEqual(gen_docs.survey_block(gen_docs.load_surveys(self.root)),
                         "No survey capture comparison is recorded.")

    def test_counterpart_false_sentence(self):
        self.put(survey(every_row_has_a_counterpart=False))
        text = gen_docs.survey_block(gen_docs.load_surveys(self.root))
        self.assertIn("Not every row on each side had a counterpart on the other side, so the comparison fails as a whole.",
                      text)

    def test_empty_not_compared_has_no_row(self):
        self.put(survey(members_not_compared={}))
        text = gen_docs.survey_block(gen_docs.load_surveys(self.root))
        self.assertNotIn("not compared", text)

    def bad(self, r, why, rel=SURVEY_REL):
        self.put(r, rel)
        code, out, err = run("--check", "--root", self.root)
        self.assertEqual(code, 3, out + err)
        self.assertIn("gen_docs: error:", err)
        self.assertIn(why, err)

    def test_not_json_exits_3(self):
        self.bad("{not json", "survey_comparison.json")

    def test_extra_key_exits_3(self):
        self.bad(survey(rows=1), "keys are not exactly")

    def test_missing_key_exits_3(self):
        r = survey()
        del r["members_not_compared"]
        self.bad(r, "keys are not exactly")

    def test_wrong_label_exits_3(self):
        self.bad(survey(label="a result"), "label is not")

    def test_wrong_comparison_exits_3(self):
        self.bad(survey(comparison="other"), "comparison is not")

    def test_folder_and_unit_differ_exits_3(self):
        self.bad(survey(unit="other"), "names another unit")

    def test_sum_mismatch_exits_3(self):
        r = survey()
        r["members"]["blind"]["equal"] = 6
        self.bad(r, "equal + different is not compared")

    def test_bool_count_exits_3(self):
        r = survey()
        r["members"]["informed"] = {"compared": True, "equal": True, "different": 0}
        self.bad(r, "not an integer of 0 or more")

    def test_negative_count_exits_3(self):
        r = survey()
        r["members_not_compared"]["form"]["blind"] = -1
        self.bad(r, "not an integer of 0 or more")

    def test_unknown_reason_exits_3(self):
        r = survey()
        r["members_not_compared"]["address"] = {"blind": 0, "word_selected": 0, "informed": 1}
        self.bad(r, "reason outside")

    def test_member_class_keys_exit_3(self):
        r = survey()
        r["members"]["handle"] = {"compared": 0, "equal": 0, "different": 0}
        self.bad(r, "members keys are not exactly")

    def test_counterpart_not_a_boolean_exits_3(self):
        self.bad(survey(every_row_has_a_counterpart="false"), "every_row_has_a_counterpart is not a boolean")

    def test_not_compared_reason_keys_exit_3(self):
        r = survey()
        r["members_not_compared"]["form"] = {"blind": 0, "word_selected": 0}
        self.bad(r, "members_not_compared.form keys are not exactly")

    def test_verdict_block_extra_key_exits_3(self):
        r = survey()
        r["members"]["blind"]["total"] = 7
        self.bad(r, "members.blind keys are not exactly")

    def test_two_survey_files_exit_3(self):
        self.put(survey())
        self.bad(survey(unit="u-two"), "more than one survey comparison file",
                 rel="parity/u-two/results/survey_comparison.json")


if __name__ == "__main__":
    unittest.main()
