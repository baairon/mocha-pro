#!/usr/bin/env python3
"""gen_docs.py: fill the generated figures and tables of the documentation from the result files.

  python -I tools/gen_docs.py            fill every marker of README.md and docs/*.md, write docs/data/facts.json
  python -I tools/gen_docs.py --check    regenerate in memory; exit 1 on drift or an unknown key

A figure is an HTML comment pair around its text, `<!--f:` KEY `-->` text `<!--/f-->`, and a block sits
between `<!--gen:` KEY `-->` and `<!--/gen:` KEY `-->` (spaced out here so this text is no marker).

The only inputs are the result files, parity/<unit>/results/summary.json, one per unit and rung, in the
fields of docs/data/result_schema.json, and the re-run file beside a result file (summary.json.rerun.json)
when an independent re-run has written one. The rules that turn a result file into a status are in
tools/status.py (the status rule of docs/parity.md). A repository with no result file reads
"method and preparation only: 0 cases compared" and carries no parity label.

--check is hermetic: it reads the result files and the tree and runs no process. A result file that is not
valid JSON or lacks a field is an error (exit 3), not a row.

Exit codes: 0 no drift; 1 drift or an unknown key; 2 usage; 3 a result file that cannot be read.
Python 3 standard library only.
"""

import argparse
import glob
import json
import os
import re
import sys

HERE = str(__import__("pathlib").Path(__file__).resolve().parent)
sys.path.insert(0, HERE)

import status  # noqa: E402

ROOT = os.path.dirname(HERE)
F_RE = re.compile(r"<!--f:([A-Za-z0-9_.-]+)-->(.*?)<!--/f-->")
G_RE = re.compile(r"(<!--gen:([A-Za-z0-9_.-]+)-->\r?\n)(.*?)(<!--/gen:\2-->)", re.S)
FACTS = "docs/data/facts.json"
EMPTY_LINE = "method and preparation only: 0 cases compared"
REQUIRED = ["unit", "rung", "judge", "comparator", "cases", "values", "call_records", "mutants"]


class DataError(Exception):
    pass


def fmt(n):
    return "{:,}".format(n)


def load_results(root):
    """The result files of the tree, sorted by unit id."""
    out = []
    for path in sorted(glob.glob(os.path.join(root, "parity", "*", "results", "summary.json"))):
        rel = os.path.relpath(path, root).replace(os.sep, "/")
        try:
            with open(path, "rb") as h:
                r = json.loads(h.read().decode("utf-8"))
            missing = [k for k in REQUIRED if k not in r]
            if missing:
                raise DataError("{}: missing field(s) {}".format(rel, ", ".join(missing)))
            if os.path.basename(os.path.dirname(os.path.dirname(path))) != r["unit"]:
                raise DataError("{}: names another unit ({})".format(rel, r["unit"]))
        except (ValueError, OSError, KeyError, TypeError) as e:
            raise DataError("{}: {}".format(rel, e)) from None
        out.append((rel, r))
    return out


def table(header, rows):
    lines = ["| " + " | ".join(header) + " |", "|" + "---|" * len(header)]
    lines += ["| " + " | ".join(r) + " |" for r in rows]
    return "\n".join(lines)


def compared_any(results):
    return any(r["cases"]["compared"] > 0 for _, r in results)


def facts_for(results):
    """The figures. Unit counts are given per rung only, in the totals lines of tools/status.py, and never
    added across rungs."""
    rs = [r for _, r in results]
    if not compared_any(results):
        line = EMPTY_LINE
    else:
        line = "; ".join(x.replace("totals, ", "", 1) for x in status.totals(rs).split("\n"))
    return {
        "units_total": (str(len(rs)), "count of parity/*/results/summary.json"),
        "status_line": (line, "tools/status.py totals over the result files, one part per rung"),
        "case_floor": (str(status.CASE_FLOOR), "tools/status.py CASE_FLOOR"),
    }


def load_rerun(root, rel):
    """The re-run file beside a result file (<result>.rerun.json), or None."""
    path = os.path.join(root, rel + ".rerun.json")
    if not os.path.exists(path):
        return None
    try:
        with open(path, "rb") as h:
            return json.loads(h.read().decode("utf-8"))
    except (ValueError, OSError) as e:
        raise DataError("{}.rerun.json: {}".format(rel, e)) from None


def units_table(root, results):
    if not results:
        return "No result file exists yet, so no unit has a row. A unit is Not started until its `summary.json` is committed."
    header = ["Unit", "Rung", "Status", "Rung label", "Cases compared (in contract)", "Differing (cases / values)",
              "Call records matching", "Mutants killed", "Re-run", "Source"]
    rows = []
    for rel, r in results:
        c, v = r["cases"], r["values"]
        label, _note = status.rung_label(r)
        rows.append([
            "`%s`" % r["unit"], r.get("rung") or "none", status.module_tier(r) or "invalid result file", label or "none",
            "%s of %s" % (fmt(c["compared"]), fmt(c["in_contract"])),
            "%s / %s" % (fmt(c["differing"]), fmt(v["differing"])),
            "%s of %s" % (fmt(r["call_records"]["matching"]), fmt(r["call_records"]["compared"])),
            "%s of %s" % (fmt(r["mutants"]["killed"]), fmt(r["mutants"]["total"])),
            status.rerun_state(r, load_rerun(root, rel)),
            "`%s`" % rel])
    out = table(header, rows)
    lines = status.totals([r for _, r in results]).split("\n")
    out += "\n\n" + "\n".join("- " + x for x in lines)
    out += "\n\nModule counts are never added across rungs."
    return out


def pairing_table(results):
    """The interface pairing and calibration of every result file that carries them."""
    rows = []
    for rel, r in results:
        pa, ca = r.get("pairing"), r.get("calibration")
        if not pa and not ca:
            continue
        pa, ca = pa or {}, ca or {}

        def n(d, k):
            return fmt(d[k]) if isinstance(d.get(k), int) and not isinstance(d.get(k), bool) else "-"

        def yes(d, k):
            return {True: "yes", False: "no"}.get(d.get(k), "-")
        rows.append(["`%s`" % r["unit"], "%s / %s" % (n(pa, "params_original"), n(pa, "params_rewrite")),
                     n(pa, "exact"), n(pa, "normalized"), n(pa, "unmatched"), n(ca, "boots"),
                     yes(ca, "records_identical"), yes(ca, "dumps_identical"), "`%s`" % rel])
    if not rows:
        return "No result file records an interface pairing or a calibration yet."
    return table(["Unit", "Parameters (original / rewrite)", "Exact", "Normalized", "Unmatched", "Boots",
                  "Records identical", "Dumps identical", "Source"], rows)


def notes_table(results):
    """What each result file states about blockers, stand-ins, qualifiers and surviving mutants."""
    rows = []
    for _rel, r in results:
        u = "`%s`" % r["unit"]
        for x in r.get("blockers", []):
            rows.append([u, "blocker", x])
        for x in r.get("stand_ins", []):
            rows.append([u, "stand-in", x])
        for x in r.get("harness_stand_ins", []):
            rows.append([u, "harness stand-in", x])
        lp = r["cases"].get("logic_parity", 0)
        if lp:
            rows.append([u, "logic parity", "%s compared cases rest on a stand-in" % fmt(lp)])
        for x in r.get("qualifiers", []):
            rows.append([u, "qualifier", x])
        for sv in r["mutants"].get("survivors", []):
            how = "killed by added case %s" % sv["killed_by_added_case"] if sv.get("killed_by_added_case") else (
                "argued equivalent, reviewed by %s" % sv["reviewed_by"] if sv.get("argument") and sv.get("reviewed_by")
                else "not settled")
            rows.append([u, "surviving mutant", "%s (%s)" % (sv.get("id", "?"), how)])
        for x in r.get("not_rebuilt", []):
            rows.append([u, "not rebuilt", x])
    if not results:
        return "No result file exists yet, so there are no notes."
    if not rows:
        return "The result files state no blocker, stand-in, qualifier or surviving mutant."
    return table(["Unit", "Kind", "Text"], [[a, b, c.replace("|", "/")] for a, b, c in rows])


def label_block(results):
    """The README label block, one per unit; with no result file, the empty form."""
    if not compared_any(results):
        body = "\n".join([
            "Status: none (%s)   [computed from %d result files; re-run: none]" % (EMPTY_LINE, len(results)),
            "Judge: none named; no result file records a compared case",
            "Data: none measured",
            "Identity: none",
            "Route: none",
            "Evidence: 0 cases compared",
            "Estimates: not applicable, nothing compared",
            "Hooks: none",
            "Contract: not applicable, nothing compared",
            "SUSPENDED: none",
            "NOT REBUILT: everything; no unit is rebuilt yet",
        ])
        return "```text\n" + body + "\n```"
    blocks = []
    for rel, r in results:
        label, note = status.rung_label(r)
        c, v, m = r["cases"], r["values"], r["mutants"]
        j = r["judge"]
        body = "\n".join([
            "# %s" % r["unit"],
            "Status: %s%s   [computed from %s]" % (label or "none", " (%s)" % ", ".join(r.get("qualifiers", [])) if r.get("qualifiers") else "", rel),
            "Judge: the original's code in %s, which %s the architecture manual, not silicon (engine cross-check: %s)" % (
                j["engine"], "follows" if j.get("manual_follows") else "is not known to follow",
                "yes" if j.get("cross_checked") else "no"),
            "Route: %s" % r.get("route", "not stated"),
            "Evidence: %s cases, %s values, %s differing; mutants %s of %s" % (
                fmt(c["compared"]), fmt(v["compared"]), fmt(c["differing"]), fmt(m["killed"]), fmt(m["total"])),
            "Stand-ins: %s" % (", ".join(r.get("stand_ins", [])) or "none"),
            "NOT REBUILT: %s" % ("; ".join(r.get("not_rebuilt", [])) or "none listed"),
        ])
        blocks.append(body)
    return "```text\n" + "\n\n".join(blocks) + "\n```"


def gen_blocks(root, results):
    return {"units": units_table(root, results), "label": label_block(results),
            "pairing": pairing_table(results), "notes": notes_table(results)}


def md_files(root):
    out = ["README.md"] if os.path.exists(os.path.join(root, "README.md")) else []
    for dirpath, dirs, files in os.walk(os.path.join(root, "docs")):
        dirs[:] = sorted(d for d in dirs if d != "data")
        for f in sorted(files):
            if f.endswith(".md"):
                out.append(os.path.relpath(os.path.join(dirpath, f), root).replace(os.sep, "/"))
    return out


def build(root=ROOT):
    """(outputs: rel path -> text, unknown keys)."""
    results = load_results(root)
    facts = facts_for(results)
    blocks = gen_blocks(root, results)
    outputs, unknown, used = {}, [], set()
    for rel in md_files(root):
        with open(os.path.join(root, rel), "rb") as h:
            text = h.read().decode("utf-8")

        def fsub(m):
            if m.group(1) not in facts:
                unknown.append("%s: figure %s" % (rel, m.group(1)))
                return m.group(0)
            used.add(m.group(1))
            return "<!--f:%s-->%s<!--/f-->" % (m.group(1), facts[m.group(1)][0])

        def gsub(m):
            if m.group(2) not in blocks:
                unknown.append("%s: block %s" % (rel, m.group(2)))
                return m.group(0)
            used.add(m.group(2))
            return m.group(1) + blocks[m.group(2)] + "\n" + m.group(4)

        outputs[rel] = G_RE.sub(gsub, F_RE.sub(fsub, text))
    doc = {k: {"value": v[0], "source": v[1]} for k, v in sorted(facts.items())}
    doc["result_files"] = {"value": [rel for rel, _ in results], "source": "parity/*/results/summary.json"}
    outputs[FACTS] = json.dumps(doc, indent=2, sort_keys=True) + "\n"
    return outputs, unknown, used


def disk(root, rel):
    path = os.path.join(root, rel)
    if not os.path.exists(path):
        return None
    with open(path, "rb") as h:
        return h.read().decode("utf-8", errors="replace")


def drift(root=ROOT):
    outputs, unknown, _ = build(root)
    return outputs, [rel for rel, text in outputs.items() if disk(root, rel) != text], unknown


def write(root, rel, text):
    path = os.path.join(root, rel)
    os.makedirs(os.path.dirname(path), exist_ok=True)
    with open(path, "wb") as h:
        h.write(text.encode("utf-8"))


def main(argv=None):
    ap = argparse.ArgumentParser(prog="gen_docs.py", description="Refresh the machine-made numbers inside the docs.")
    ap.add_argument("--check", dest="check_only", help="rebuild in memory only; exit 1 if any file is stale", action="store_true")
    ap.add_argument("--root", default=ROOT, help=argparse.SUPPRESS)
    args = ap.parse_args(argv)
    try:
        outputs, changed, unknown = drift(args.root)
    except (DataError, OSError) as e:
        print("gen_docs: error: {}".format(e), file=sys.stderr)
        return 3
    if args.check_only:
        print("gen_docs --check: {} outputs, {} drifted, {} unknown keys".format(len(outputs), len(changed), len(unknown)))
        for item in (changed + unknown)[:20]:
            print("  " + item)
        return 1 if (changed or unknown) else 0
    for rel in changed:
        write(args.root, rel, outputs[rel])
    print("gen_docs: {} outputs, {} written, {} unknown keys".format(len(outputs), len(changed), len(unknown)))
    for item in unknown[:20]:
        print("  " + item)
    return 1 if unknown else 0


if __name__ == "__main__":
    sys.exit(main())
