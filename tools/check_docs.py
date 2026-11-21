#!/usr/bin/env python3
"""check_docs.py: check the documentation and the text files of the mocha-pro repository.

  python -I tools/check_docs.py                           run every check, print counts and the first findings
  python -I tools/check_docs.py --list NAME               print every finding of one check
  python -I tools/check_docs.py --private-terms PATH      also run process and scope with the rows of a term list
  python -I tools/check_docs.py --private-terms PATH --require-private
                                                          exit 2 unless the list is named and valid

Checks:
  regen    the generated figures and blocks equal a fresh `gen_docs.py` generation, no unknown key
  links    relative links and #anchors in README.md and docs/*.md resolve (GitHub heading anchors)
  ascii    every text file holds ASCII only, and no control character but tab and CR
  dashes   no U+2010 to U+2015 or U+2212 in any text file of the repository
  wording  the status wording rules: no word that says the work is finished, no "bit exact" or bare "bit for
           bit" without a rung or a tolerance on the same line, no percentage, no merged count phrase
  hexaddr  no hexadecimal address (0x and five or more hex digits) in a page
  secrets  no private key block, token, assigned secret, email address outside the allowed domains, or home
           directory path in any text file
  process  terms that say how the repository was made, from a term list kept outside the repository
  scope    names that belong outside the repository, from a term list kept outside the repository

The checks process and scope need the list. It is named by --private-terms PATH or by the environment
variable MOCHA_PRIVATE_TERMS (the argument wins). Without a list they print NOT RUN and add no finding. A
finding of these two checks prints FILE:LINE: ROW-NAME and never the matched text.

The list is a tab separated file: check (process or scope), name, pattern (a Python regular expression),
flags (i or empty), scope (all: every text file; prose: page prose only, outside code spans, fenced blocks
and comments), probe (a literal the pattern matches).
Lines that are empty or start with # are ignored, and so is a first row that holds the column names.

Exit codes: 0 no findings; 1 findings; 2 usage, or a term list that is missing, unreadable, empty or badly
formed (or no list with --require-private). Python 3 standard library only.
"""

import argparse
import os
import re
import sys

TOOLS = str(__import__("pathlib").Path(__file__).resolve().parent)
sys.path.insert(0, TOOLS)

import gen_docs  # noqa: E402

ROOT = gen_docs.ROOT
CHECKS = ["regen", "links", "ascii", "dashes", "wording", "hexaddr", "secrets", "process", "scope"]
LISTED = ("process", "scope")
ENV_TERMS = "MOCHA_PRIVATE_TERMS"
TEXT_EXT = (".md", ".py", ".rs", ".toml", ".json", ".tsv", ".yml", ".yaml", ".txt", ".lock")
TEXT_NAMES = ("LICENSE", ".gitignore", ".gitattributes")
SKIP_DIRS = {".git", "target", "node_modules", "__pycache__"}
DASHES = tuple(chr(c) for c in range(0x2010, 0x2016)) + (chr(0x2212),)
LINK = re.compile(r"!?\[([^\]]*)\]\(([^)\s]+)(?:\s+\"[^\"]*\")?\)")
COMMENT = re.compile(r"<!--.*?-->", re.S)
SPAN = re.compile(r"``\s?(.+?)\s?``|`([^`\n]+)`")

# Wording: built from pieces so that this file does not trip a scan of itself.
FINISHED = re.compile(r"\b(" + "|".join(["do" + "ne", "comp" + "lete", "comp" + "leted", "fin" + "ished"]) + r")\b", re.I)
BITEXACT = re.compile(r"\bbit[- ]exact\b", re.I)
BITFORBIT = re.compile(r"\bbit for bit\b", re.I)
QUALIFIER = re.compile(r"\b(rung|tolerance|judge)\b", re.I)
PERCENT = re.compile(r"\d\s?%|\bpercent(age)?s?\b", re.I)
MERGED = re.compile(r"\b(cases|values) in total\b|\boverall (cases|count)\b|\bgrand total\b", re.I)

HEXADDR = re.compile(r"\b0x[0-9a-fA-F]{5,}\b")
SECRET_PATTERNS = [
    ("private key", re.compile(r"-----BEGIN [A-Z ]*PRIVATE KEY-----")),
    ("github token", re.compile(r"\b(?:gh[pousr]_[A-Za-z0-9]{30,}|github_pat_[A-Za-z0-9_]{30,})")),
    ("api key", re.compile(r"\b(?:sk-[A-Za-z0-9_-]{20,}|xox[baprs]-[A-Za-z0-9-]{10,}|AKIA[0-9A-Z]{16}|AIza[0-9A-Za-z_-]{35})")),
    ("assigned secret", re.compile(r"(?i)\b(?:password|passwd|secret|api[_-]?key|token)\s*[:=]\s*['\"][^'\"\s]{8,}['\"]")),
    ("home path", re.compile(r"(?i)\b[A-Z]:[\\/]+Users[\\/]+[^\\/\s'\x22]{2,}|/home/[a-z][\w.-]*/|/Users/[A-Za-z][\w.-]*/")),
]
EMAIL = re.compile(r"\b[A-Za-z0-9._%+-]+@[A-Za-z0-9-]+(?:\.[A-Za-z0-9-]+)*\.[A-Za-z]{2,}\b")
EMAIL_OK = re.compile(r"@(?:example\.(?:com|org|net)|users\.noreply\.github\.com|bairon\.dev)$")
LIST_HEADER = ["check", "name", "pattern", "flags", "scope", "probe"]


class TermError(Exception):
    """A term list that cannot be used; the message names the file and a line number only."""


def lines_outside_fences(text):
    fenced = False
    for i, line in enumerate(text.split("\n"), 1):
        if line.lstrip().startswith("```"):
            fenced = not fenced
            continue
        yield i, line, fenced


def prose(text):
    """(line number, line) of prose: outside fences, HTML comments blanked."""
    text = COMMENT.sub(lambda m: re.sub(r"[^\n]", " ", m.group(0)), text)
    return [(i, ln) for i, ln, fenced in lines_outside_fences(text) if not fenced]


def anchor(heading):
    h = re.sub(r"[`*_]", "", heading.strip().lower())
    h = re.sub(r"[^\w\- ]", "", h)
    return h.replace(" ", "-")


def load_terms(path):
    """Rows of the term list as (check, name, compiled pattern, scope). Raises TermError."""
    try:
        with open(path, "rb") as h:
            raw = h.read().decode("utf-8")
    except (OSError, ValueError):
        raise TermError("cannot read term list %s" % path)
    rows = []
    for i, line in enumerate(raw.split("\n"), 1):
        line = line.rstrip("\r")
        if not line.strip() or line.startswith("#"):
            continue
        cells = line.split("\t")
        if not rows and cells[0] == LIST_HEADER[0]:
            if cells != LIST_HEADER:
                raise TermError("bad term list %s: line %d" % (path, i))
            rows.append(None)
            continue
        if len(cells) != len(LIST_HEADER):
            raise TermError("bad term list %s: line %d" % (path, i))
        check, name, pattern, flags, scope, _probe = cells
        if check not in LISTED or not name.strip() or not pattern or flags not in ("", "i") or scope not in ("all", "prose"):
            raise TermError("bad term list %s: line %d" % (path, i))
        try:
            rx = re.compile(pattern, re.I if flags == "i" else 0)
        except re.error:
            raise TermError("bad term list %s: line %d" % (path, i))
        rows.append((check, name, rx, scope))
    rows = [r for r in rows if r]
    if not rows:
        raise TermError("bad term list %s: no rows" % path)
    return rows


class Ctx:
    def __init__(self, root, terms=None):
        self.root = root
        self.terms = terms
        self.md = gen_docs.md_files(root)
        self._text = {}

    def text(self, rel):
        if rel not in self._text:
            with open(os.path.join(self.root, rel), "rb") as h:
                self._text[rel] = h.read().decode("utf-8", errors="replace")
        return self._text[rel]

    def text_files(self):
        out = []
        for dirpath, dirs, files in os.walk(self.root):
            dirs[:] = sorted(d for d in dirs if d not in SKIP_DIRS)
            for f in sorted(files):
                rel = os.path.relpath(os.path.join(dirpath, f), self.root).replace(os.sep, "/")
                if f.endswith(TEXT_EXT) or f in TEXT_NAMES:
                    out.append(rel)
        return out


def check_regen(ctx):
    try:
        _, changed, unknown = gen_docs.drift(ctx.root)
    except (gen_docs.DataError, OSError) as e:
        return ["cannot generate: %s" % e]
    return ["stale: " + c for c in changed] + ["unknown: " + u for u in unknown]


def check_links(ctx):
    out = []
    anchors = {}
    for rel in ctx.md:
        heads = set()
        for _, line, fenced in lines_outside_fences(ctx.text(rel)):
            m = re.match(r"^#{1,6}\s+(.*)$", line)
            if m and not fenced:
                heads.add(anchor(m.group(1)))
        anchors[rel] = heads
    for rel in ctx.md:
        for i, line in prose(ctx.text(rel)):
            for m in LINK.finditer(line):
                target = m.group(2)
                if re.match(r"^[a-z]+:", target):
                    continue
                path, _, frag = target.partition("#")
                dest = rel if not path else os.path.normpath(os.path.join(os.path.dirname(rel), path)).replace(os.sep, "/")
                if path and not os.path.exists(os.path.join(ctx.root, dest)):
                    out.append("%s:%d: broken link %s" % (rel, i, target))
                elif frag and dest in anchors and frag not in anchors[dest]:
                    out.append("%s:%d: no anchor %s" % (rel, i, target))
    return out


def check_ascii(ctx):
    out = []
    for rel in ctx.text_files():
        for i, line in enumerate(ctx.text(rel).split("\n"), 1):
            for ch in line:
                if ord(ch) > 126 or (ord(ch) < 32 and ch not in "\t\r"):
                    out.append("%s:%d: U+%04X" % (rel, i, ord(ch)))
                    break
    return out


def check_dashes(ctx):
    out = []
    for rel in ctx.text_files():
        for i, line in enumerate(ctx.text(rel).split("\n"), 1):
            if any(d in line for d in DASHES):
                out.append("%s:%d: dash character" % (rel, i))
    return out


def check_wording(ctx):
    out = []
    for rel in ctx.md:
        for i, line in prose(ctx.text(rel)):
            if FINISHED.search(line):
                out.append("%s:%d: word that claims a finished state" % (rel, i))
            if BITEXACT.search(line) and "rung" not in line.lower():
                out.append("%s:%d: bit exact without a rung" % (rel, i))
            if BITFORBIT.search(line) and not QUALIFIER.search(line):
                out.append("%s:%d: bit for bit without a rung, tolerance or judge" % (rel, i))
            if PERCENT.search(line):
                out.append("%s:%d: percentage" % (rel, i))
            if MERGED.search(line):
                out.append("%s:%d: merged count phrase" % (rel, i))
    return out


def check_hexaddr(ctx):
    out = []
    for rel in ctx.md:
        for i, line in prose(ctx.text(rel)):
            if HEXADDR.search(line):
                out.append("%s:%d: hexadecimal address" % (rel, i))
    return out


def check_secrets(ctx):
    out = []
    for rel in ctx.text_files():
        for i, line in enumerate(ctx.text(rel).split("\n"), 1):
            for name, rx in SECRET_PATTERNS:
                if rx.search(line):
                    out.append("%s:%d: %s" % (rel, i, name))
            if any(not EMAIL_OK.search(m.group(0)) for m in EMAIL.finditer(line)):
                out.append("%s:%d: email address" % (rel, i))
    return out


def scan_terms(ctx, check):
    """Findings of the listed rows of one check: FILE:LINE: ROW-NAME, never the matched text."""
    rows = [r for r in ctx.terms if r[0] == check]
    out = []
    for rel in ctx.text_files():
        text = ctx.text(rel)
        prose_lines = {i: SPAN.sub(lambda m: " " * len(m.group(0)), ln) for i, ln in prose(text)} if rel in ctx.md else {}
        for i, line in enumerate(text.split("\n"), 1):
            for _, name, rx, scope in rows:
                hit = rx.search(line) if scope == "all" else rx.search(prose_lines.get(i, ""))
                if hit:
                    out.append("%s:%d: %s" % (rel, i, name))
    return out


def check_process(ctx):
    return scan_terms(ctx, "process")


def check_scope(ctx):
    return scan_terms(ctx, "scope")


FUNCS = {"regen": check_regen, "links": check_links, "ascii": check_ascii, "dashes": check_dashes,
         "wording": check_wording, "hexaddr": check_hexaddr, "secrets": check_secrets,
         "process": check_process, "scope": check_scope}
NOT_RUN = "check_docs %-8s NOT RUN (no term list: --private-terms PATH or " + ENV_TERMS + ")"


def main(argv=None):
    ap = argparse.ArgumentParser(prog="check_docs.py", description="Check the documentation of mocha-pro.")
    ap.add_argument("--list", metavar="NAME", choices=CHECKS, help="print every finding of one check")
    ap.add_argument("--private-terms", metavar="PATH", help="term list for the checks process and scope")
    ap.add_argument("--require-private", action="store_true", help="exit 2 unless a valid term list is named")
    ap.add_argument("--root", default=ROOT, help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    path = args.private_terms or os.environ.get(ENV_TERMS) or None
    if path is None and args.require_private:
        print("check_docs: --require-private needs a term list: --private-terms PATH or " + ENV_TERMS, file=sys.stderr)
        return 2
    terms = None
    if path is not None:
        try:
            terms = load_terms(path)
        except TermError as e:
            print("check_docs: %s" % e, file=sys.stderr)
            return 2
    ctx = Ctx(args.root, terms)
    active = [c for c in CHECKS if terms is not None or c not in LISTED]
    skipped = [c for c in CHECKS if c not in active]
    results = {name: FUNCS[name](ctx) for name in active}
    if args.list:
        if args.list in skipped:
            print(NOT_RUN % args.list)
            return 0
        for f in results[args.list]:
            print(f)
        return 1 if results[args.list] else 0
    total = 0
    for name in CHECKS:
        if name in skipped:
            print(NOT_RUN % name)
            continue
        n = len(results[name])
        total += n
        print("check_docs %-8s %d findings" % (name, n))
        for f in results[name][:10]:
            print("  " + f)
    note = ", %d checks not run" % len(skipped) if skipped else ""
    print("check_docs: %d pages, %d findings%s" % (len(ctx.md), total, note))
    return 1 if total else 0


if __name__ == "__main__":
    sys.exit(main())
