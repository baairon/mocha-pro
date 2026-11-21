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
import unittest
from pathlib import Path

HERE = str(Path(__file__).resolve().parent)
ROOT = os.path.dirname(HERE)
GEN = os.path.join(HERE, "gen_docs.py")


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


if __name__ == "__main__":
    unittest.main()
