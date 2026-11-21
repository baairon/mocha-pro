#!/usr/bin/env python3
"""Computes the status of result files, from the result files only.

Usage: status.py result.json [...]   Prints one line per file and a totals row.
Exit codes: 0 the lines were printed; 2 usage; 3 a file that cannot be read or is not JSON.
Python 3 standard library only. Only the allowed status wording is printed: no claim that work is finished,
no bare "bit exact", no percentages and no merged counts.
"""
import json, os, sys
from datetime import datetime, timezone

CASE_FLOOR = 150  # the fewest compared cases a module may have and still match


def _g(d, *path, default=0):
    for p in path:
        if not isinstance(d, dict) or p not in d:
            return default
        d = d[p]
    return d


def survivor_ok(s):
    """A surviving mutant is accepted when an added case kills it, or when it has a written argument and a reviewer."""
    return bool(s.get("killed_by_added_case")) or bool(s.get("argument") and s.get("reviewed_by"))


def survivor_rule(r):
    m = r.get("mutants", {})
    total, killed = m.get("total", 0), m.get("killed", 0)  # killed counts outright kills only
    if total < 10 or killed * 10 < total * 9:
        return False
    return all(survivor_ok(s) for s in m.get("survivors", []))


def invariants(r):
    """Problems that make a result file invalid. An empty list means the file is valid."""
    c = r["cases"]
    bad = []
    if c["identical"] + c["differing"] != c["compared"]:
        bad.append("identical + differing != compared")
    if c["compared"] > c["in_contract"]:
        bad.append("compared > in_contract")
    return bad


def small_cases(r):
    """True only when contract.max_values_per_case is an integer of 16 or less."""
    m = _g(r, "contract", "max_values_per_case", default=None)
    return isinstance(m, int) and not isinstance(m, bool) and m <= 16


def k_floor_misses(r):
    """Unmet K-PARITY floor items (empty list means K-PARITY)."""
    miss = []
    c, v = r["cases"], r["values"]
    if small_cases(r):
        if c["compared"] < 500:
            miss.append("compared < 500 (small cases)")
    elif c["compared"] < 200 or v["compared"] < 10000:
        miss.append("compared < 200 or values compared < 10000")
    b = r.get("branches", {})
    if b.get("both_ways", 0) + len(b.get("unreached", [])) < b.get("total", 0):
        miss.append("branch not both ways and not unreached with reason")
    if any(not u.get("reason") for u in b.get("unreached", [])):
        miss.append("unreached branch without reason")
    allowed = r.get("value_classes_allowed")
    if not allowed:
        miss.append("value_classes_allowed missing or empty")
    elif any(x not in r.get("value_classes", []) for x in allowed):
        miss.append("allowed value class not present")
    if any(not s.get("killed_by_mutant") for s in r.get("sites", [])):
        miss.append("site not killed by a mutant")
    if not survivor_rule(r):
        miss.append("survivor rule unmet")
    if not _g(r, "mutants", "control_matches_release", default=False):
        miss.append("control build differs from release")
    if any("SUSPENDED" in s for s in r.get("stand_ins", [])):
        miss.append("SUSPENDED stand-in")
    return miss


def rung_label(r):
    """Returns (label or None, note)."""
    c = r["cases"]
    if _g(r, "comparator", "tolerance_ulp") != 0:
        return None, "tolerance not 0"
    if c["compared"] == 0:
        return None, "0 cases compared"
    rung = r.get("rung")
    cr = r["call_records"]
    if rung == "REC-PARITY":
        if cr["compared"] > 0 and cr["matching"] == cr["compared"]:
            return "REC-PARITY", ""
        return None, "call records not all matching"
    if rung == "K-PARITY":
        if c["differing"] or r["values"]["differing"]:
            return None, "differing cases or values"
        miss = k_floor_misses(r)
        return ("K-PARITY", "") if not miss else ("K-PARITY-THIN", "unmet: " + "; ".join(miss))
    return None, "rung %s is not computed: REC-PARITY and K-PARITY (with K-PARITY-THIN) are the rungs this tool computes, any other rung is not computed" % rung


def module_tier(r):
    """Returns None (no tier) for an invalid result file."""
    if invariants(r):
        return None
    if r.get("blockers"):
        return "Blocked"
    c, v, cr = r["cases"], r["values"], r["call_records"]
    clean = (c["compared"] >= CASE_FLOOR and c["compared"] == c["in_contract"] and r["comparator"]["tolerance_ulp"] == 0
             and c["differing"] == 0 and v["differing"] == 0
             and cr["compared"] >= 1 and cr["matching"] == cr["compared"]  # a check with nothing in it is unmet
             and r.get("rung") is not None)  # a file with no rung is not a parity unit
    if clean and survivor_rule(r):  # clean already needs compared == in_contract
        return "Verified"
    if clean:
        return "Matches"
    return "In progress"


def _ts(x):
    try:
        d = datetime.fromisoformat(str(x).replace("Z", "+00:00"))
    except ValueError:
        return None
    return d if d.tzinfo else d.replace(tzinfo=timezone.utc)


def rerun_state(r, rerun):
    """Re-run state from the result and its re-run file (None when absent).
    Returns 'yes', 'no (N differences)', 'stale' or 'none'. Only a re-run dated after
    measured_at counts; an unreadable date is stale (never yes)."""
    if not isinstance(rerun, dict):
        return "none"
    t_res, t_run = _ts(r.get("measured_at")), _ts(rerun.get("reran_at"))
    if t_res is None or t_run is None or t_run <= t_res:
        return "stale"
    if rerun.get("matches") is True and not rerun.get("differences"):
        return "yes"
    return "no (%d differences)" % len(rerun.get("differences", []))


def line(r, rerun=None):
    bad = invariants(r)
    if bad:
        return "%s | invalid result file (%s)" % (r["unit"], "; ".join(bad))
    tier = module_tier(r)
    label, note = rung_label(r)
    c = r["cases"]
    j = r["judge"]["engine"]
    out = "%s | %s | rung label: %s" % (r["unit"], tier, label or "none")
    if note:
        out += " (%s)" % note
    if tier in ("Verified", "Matches"):
        out += " | matches the emulated original (rung %s, %d of %d compared cases identical, judge %s)" % (
            r.get("rung"), c["identical"], c["compared"], j)
    if tier == "Blocked":
        out += " | blockers: " + "; ".join(r["blockers"])
    st = rerun_state(r, rerun)
    out += " | re-run verified" if st == "yes" else " | re-run: " + st
    return out


def totals(results):
    """One line per rung present; module counts are never summed across rungs. A result file with no rung
    (pairing and calibration only) is not a parity unit: it gets its own line and never enters a rung line.
    No result file: a none line."""
    if not results:
        return "totals: none (0 cases compared)"
    units = [r for r in results if r.get("rung") is not None]
    apart = [r for r in results if r.get("rung") is None]
    rungs = []
    for r in units:
        if r["rung"] not in rungs:
            rungs.append(r["rung"])
    out = []
    for rung in sorted(rungs):
        rs = [r for r in units if r["rung"] == rung]
        m = sum(1 for r in rs if module_tier(r) in ("Verified", "Matches"))
        v = sum(1 for r in rs if module_tier(r) == "Verified")
        out.append("totals, rung %s: %d of %d modules match the emulated original, %d of %d verified (judge: emulator)" % (
            rung, m, len(rs), v, len(rs)))
    for r in apart:
        out.append("not a parity unit: %s carries pairing and calibration only, has no rung and %d cases compared" % (
            r["unit"], _g(r, "cases", "compared")))
    return "\n".join(out)


def report(results, reruns=None):
    reruns = reruns or [None] * len(results)
    return "\n".join([line(r, x) for r, x in zip(results, reruns)] + [totals(results)])


def read_json(path):
    with open(path, "rb") as h:
        return json.loads(h.read().decode("utf-8"))


def main(argv):
    if any(a in ("-h", "--help") for a in argv):
        print("usage: status.py result.json [...]", file=sys.stderr)
        return 2
    rs, rr = [], []
    for p in argv:
        q = p + ".rerun.json"
        try:
            rs.append(read_json(p))
            rr.append(read_json(q) if os.path.exists(q) else None)
        except (OSError, ValueError) as e:  # ValueError covers JSON and UTF-8 decode errors
            bad = q if rs and len(rr) < len(rs) else p
            print("status: cannot read %s: %s" % (bad, e), file=sys.stderr)
            return 3
    print(report(rs, rr) if rs else "Not started (no result file)")
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
