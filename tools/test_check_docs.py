"""Tests of check_docs.py: the checks that need a term list, and the generic checks hexaddr and secrets.

Run: python -I tools/test_check_docs.py. Every fixture is built in a temporary directory with invented
terms only. The one test that reads a real term list runs only when MOCHA_PRIVATE_TERMS is set in the
environment of the test, and otherwise is skipped with a message.
"""
import os
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

HERE = str(Path(__file__).resolve().parent)
CHECK = os.path.join(HERE, "check_docs.py")
GEN = os.path.join(HERE, "gen_docs.py")
sys.path.insert(0, HERE)

import check_docs  # noqa: E402

HEADER = "check\tname\tpattern\tflags\tscope\tprobe\n"
TERM = "test term"
WORD = "quuxterm"


def row(check, name, pattern, flags="", scope="all", probe=WORD):
    return "\t".join([check, name, pattern, flags, scope, probe]) + "\n"


def write(path, text):
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as h:
        h.write(text.encode("utf-8"))


class Scratch(unittest.TestCase):
    """A scratch repository and helpers to run the checker on it."""

    def setUp(self):
        self._tmp = tempfile.TemporaryDirectory()
        self.addCleanup(self._tmp.cleanup)
        self.dir = self._tmp.name
        self.root = os.path.join(self.dir, "repo")
        os.makedirs(self.root)
        self.put("README.md", "# Scratch\n\nA plain page.\n")
        self.put("docs/page.md", "# Page\n\nPlain text.\n")

    def put(self, rel, text):
        write(os.path.join(self.root, rel), text)

    def terms(self, *rows, name="terms.tsv"):
        path = os.path.join(self.dir, name)
        write(path, HEADER + "".join(rows))
        return path

    def run_check(self, *args, env=None):
        """Write the generated files of the scratch repository, then run the checker. Returns (code, out, err)."""
        subprocess.run([sys.executable, "-I", GEN, "--root", self.root], capture_output=True, check=True)
        e = {k: v for k, v in os.environ.items() if k != "MOCHA_PRIVATE_TERMS"}
        e.update(env or {})
        p = subprocess.run([sys.executable, "-I", CHECK, "--root", self.root] + list(args), capture_output=True,
                           text=True, env=e)
        return p.returncode, p.stdout, p.stderr


class NotRun(Scratch):
    def test_no_list_prints_not_run_and_exits_0(self):
        code, out, _ = self.run_check()
        self.assertEqual(code, 0, out)
        self.assertIn("check_docs process  NOT RUN (no term list: --private-terms PATH or MOCHA_PRIVATE_TERMS)", out)
        self.assertIn("check_docs scope    NOT RUN (no term list: --private-terms PATH or MOCHA_PRIVATE_TERMS)", out)
        self.assertIn("0 findings, 2 checks not run", out)

    def test_list_option_not_run(self):
        for name in ("process", "scope"):
            code, out, _ = self.run_check("--list", name)
            self.assertEqual(code, 0)
            self.assertIn("NOT RUN", out)

    def test_other_checks_still_decide_the_exit_code(self):
        self.put("docs/page.md", "# Page\n\nThis is " + "do" + "ne.\n")
        code, out, _ = self.run_check()
        self.assertEqual(code, 1)
        self.assertIn("wording", out)
        self.assertIn("2 checks not run", out)

    def test_empty_environment_variable_counts_as_no_list(self):
        code, out, _ = self.run_check(env={"MOCHA_PRIVATE_TERMS": ""})
        self.assertEqual(code, 0)
        self.assertIn("NOT RUN", out)


class RequirePrivate(Scratch):
    def test_no_list_exits_2_before_any_check(self):
        code, out, err = self.run_check("--require-private")
        self.assertEqual(code, 2)
        self.assertEqual(out, "")
        self.assertIn("--require-private", err)

    def test_valid_list_runs_every_check(self):
        path = self.terms(row("process", TERM, WORD), row("scope", TERM, WORD))
        code, out, _ = self.run_check("--require-private", "--private-terms", path)
        self.assertEqual(code, 0, out)
        self.assertNotIn("NOT RUN", out)
        self.assertNotIn("not run", out)


class BadList(Scratch):
    def exit2(self, path):
        code, out, err = self.run_check("--private-terms", path)
        self.assertEqual(code, 2, out + err)
        self.assertEqual(out, "")
        return err

    def test_missing_file(self):
        err = self.exit2(os.path.join(self.dir, "absent.tsv"))
        self.assertIn("absent.tsv", err)

    def test_empty_file(self):
        path = os.path.join(self.dir, "empty.tsv")
        write(path, "")
        self.assertIn("empty.tsv", self.exit2(path))

    def test_short_row_names_the_line(self):
        path = self.terms(row("process", TERM, WORD), "process\tshort\trow\n")
        err = self.exit2(path)
        self.assertIn("terms.tsv", err)
        self.assertIn("line 3", err)

    def test_pattern_that_does_not_compile(self):
        path = self.terms(row("scope", TERM, "(" + WORD))
        err = self.exit2(path)
        self.assertIn("line 2", err)
        self.assertNotIn(WORD, err)

    def test_unknown_check_flags_and_scope(self):
        for bad in (row("other", TERM, WORD), row("process", TERM, WORD, flags="x"), row("process", TERM, WORD, scope="some"),
                    row("process", TERM, "")):
            path = self.terms(bad)
            self.assertIn("line 2", self.exit2(path))

    def test_loader_rejects_with_term_error(self):
        path = self.terms(row("process", TERM, "["))
        with self.assertRaises(check_docs.TermError):
            check_docs.load_terms(path)

    def test_comments_blank_lines_and_header_are_ignored(self):
        path = os.path.join(self.dir, "c.tsv")
        write(path, "# a comment\n\n" + HEADER + row("process", TERM, WORD))
        self.assertEqual(len(check_docs.load_terms(path)), 1)


class Findings(Scratch):
    def test_finding_in_a_page(self):
        self.put("docs/page.md", "# Page\n\nA line with " + WORD + " in it.\n")
        path = self.terms(row("process", TERM, WORD))
        code, out, _ = self.run_check("--private-terms", path)
        self.assertEqual(code, 1)
        self.assertIn("docs/page.md:3: " + TERM, out)

    def test_finding_in_a_py_file(self):
        self.put("tools/helper.py", "x = 1\n# note: " + WORD + "\n")
        path = self.terms(row("scope", TERM, WORD))
        code, out, _ = self.run_check("--list", "scope", "--private-terms", path)
        self.assertEqual(code, 1)
        self.assertEqual(out.strip(), "tools/helper.py:2: " + TERM)

    def test_finding_inside_a_code_span_and_a_fence_for_scope_all(self):
        self.put("docs/page.md", "# Page\n\nUse `" + WORD + "` here.\n\n```\n" + WORD + "\n```\n")
        path = self.terms(row("process", TERM, WORD))
        code, out, _ = self.run_check("--list", "process", "--private-terms", path)
        self.assertEqual(code, 1)
        self.assertIn("docs/page.md:3: " + TERM, out)
        self.assertIn("docs/page.md:6: " + TERM, out)

    def test_no_finding_in_a_code_span_for_scope_prose(self):
        self.put("docs/page.md", "# Page\n\nUse `" + WORD + "` here.\n\n```\n" + WORD + "\n```\n")
        path = self.terms(row("process", TERM, WORD, scope="prose"))
        code, out, _ = self.run_check("--list", "process", "--private-terms", path)
        self.assertEqual(code, 0, out)

    def test_prose_scope_still_finds_plain_text_and_skips_code_files(self):
        self.put("docs/page.md", "# Page\n\nPlain " + WORD + " text.\n")
        self.put("tools/helper.py", "# " + WORD + "\n")
        path = self.terms(row("process", TERM, WORD, scope="prose"))
        code, out, _ = self.run_check("--list", "process", "--private-terms", path)
        self.assertEqual(code, 1)
        self.assertEqual(out.strip(), "docs/page.md:3: " + TERM)

    def test_flags_i(self):
        self.put("docs/page.md", "# Page\n\nA " + WORD.upper() + " line.\n")
        for flags, expect in (("i", 1), ("", 0)):
            path = self.terms(row("process", TERM, WORD, flags=flags))
            self.assertEqual(self.run_check("--private-terms", path)[0], expect)

    def test_findings_never_print_the_matched_text(self):
        self.put("docs/page.md", "# Page\n\nA line with " + WORD + " in it.\n")
        path = self.terms(row("process", TERM, WORD), row("scope", TERM, "line with"))
        code, out, err = self.run_check("--private-terms", path)
        self.assertEqual(code, 1)
        self.assertNotIn(WORD, out + err)
        self.assertNotIn("line with", out + err)

    def test_argument_overrides_the_environment_variable(self):
        self.put("docs/page.md", "# Page\n\nA line with " + WORD + " in it.\n")
        hit = self.terms(row("process", TERM, WORD), name="hit.tsv")
        miss = self.terms(row("process", TERM, "zzzzzzzz"), name="miss.tsv")
        self.assertEqual(self.run_check("--private-terms", miss, env={"MOCHA_PRIVATE_TERMS": hit})[0], 0)
        self.assertEqual(self.run_check("--private-terms", hit, env={"MOCHA_PRIVATE_TERMS": miss})[0], 1)
        self.assertEqual(self.run_check(env={"MOCHA_PRIVATE_TERMS": hit})[0], 1)
        # a broken environment list does not matter when the argument names a good one
        self.assertEqual(self.run_check("--private-terms", miss, env={"MOCHA_PRIVATE_TERMS": "/absent"})[0], 0)

    def test_the_environment_variable_alone_is_used(self):
        miss = self.terms(row("process", TERM, "zzzzzzzz"))
        code, out, _ = self.run_check(env={"MOCHA_PRIVATE_TERMS": miss})
        self.assertEqual(code, 0)
        self.assertNotIn("NOT RUN", out)


# The address, secret, email and home path fixtures below, and the finished-state word above, are joined from
# pieces so that this file holds no whole string of those shapes for a secret scanner or a scan of the repository
# to flag. Every joined value is an invented example: no real secret, address, path or term.
class Hexaddr(Scratch):
    def test_hex_address_in_a_page(self):
        self.put("docs/page.md", "# Page\n\nAt 0x" + "1a2b3c" + " there.\n")
        code, out, _ = self.run_check("--list", "hexaddr")
        self.assertEqual(code, 1)
        self.assertEqual(out.strip(), "docs/page.md:3: hexadecimal address")

    def test_short_hex_and_fenced_hex_are_allowed(self):
        self.put("docs/page.md", "# Page\n\nValue 0x1F and 0xABCD.\n\n```\n0x" + "1a2b3c4d\n```\n")
        self.assertEqual(self.run_check("--list", "hexaddr")[0], 0)


class Secrets(Scratch):
    def hits(self, text, rel="docs/page.md"):
        self.put(rel, text)
        code, out, _ = self.run_check("--list", "secrets")
        return code, out

    def test_clean_page(self):
        self.assertEqual(self.hits("# Page\n\nNothing here.\n")[0], 0)

    def test_allowed_email_domains(self):
        text = "# Page\n\n" + "\n\n".join(u + "@" + d for u, d in [
            ("a", "example.com"), ("b", "example.org"), ("c", "users.noreply.github.com"), ("d", "bairon.dev")]) + "\n"
        self.assertEqual(self.hits(text)[0], 0)

    def test_other_email_domain_is_a_finding(self):
        code, out = self.hits("# Page\n\n" + "someone" + "@" + "elsewhere.net" + "\n")
        self.assertEqual(code, 1)
        self.assertIn("docs/page.md:3: email address", out)
        self.assertNotIn("elsewhere", out)

    def test_token_and_private_key(self):
        code, out = self.hits("t = '" + "gh" + "p_" + "A" * 36 + "'\n# -----BEGIN " + "RSA PRIVATE KEY-----\n", "tools/x.py")
        self.assertEqual(code, 1)
        self.assertIn("tools/x.py:1: github token", out)
        self.assertIn("tools/x.py:2: private key", out)

    def test_assigned_secret(self):
        code, out = self.hits("pass" + "word = 'abcdefgh12'\n", "tools/x.py")
        self.assertEqual(code, 1)
        self.assertIn("assigned secret", out)

    def test_home_paths(self):
        for p in ("/ho" + "me/someone/file", "/Us" + "ers/someone/file", "C:" + "\\Users\\" + "someone"):
            code, out = self.hits("path = " + p + "\n", "tools/x.py")
            self.assertEqual(code, 1, p)
            self.assertIn("home path", out)

    def test_secrets_also_cover_dotfiles(self):
        code, out = self.hits("# " + "someone" + "@" + "elsewhere.net\n", ".gitattributes")
        self.assertEqual(code, 1)
        self.assertIn(".gitattributes:1: email address", out)


class ShippedFiles(unittest.TestCase):
    def test_the_checker_and_its_tests_hold_no_term_of_the_real_list(self):
        path = os.environ.get("MOCHA_PRIVATE_TERMS")
        if not path:
            self.skipTest("MOCHA_PRIVATE_TERMS is not set: the real term list is not read, this test is skipped")
        rows = check_docs.load_terms(path)
        for name in ("check_docs.py", "test_check_docs.py", "test_gen_docs.py", "test_status.py", "status.py", "gen_docs.py"):
            with open(os.path.join(HERE, name), "rb") as h:
                lines = h.read().decode("utf-8").split("\n")
            for i, line in enumerate(lines, 1):
                for _check, label, rx, scope in rows:
                    if scope != "all":
                        continue
                    self.assertIsNone(rx.search(line), "%s:%d: %s" % (name, i, label))


if __name__ == "__main__":
    unittest.main()
