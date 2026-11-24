#!/usr/bin/env python3
"""Computes the status of result files, from the result files only.

Usage: status.py result.json [...]   Prints one line per file and a totals row.
Exit codes: 0 the lines were printed; 2 usage; 3 a file that cannot be read or is not JSON.
Python 3 standard library only. Only the allowed status wording is printed: no claim that work is finished,
no bare "bit exact", no percentages and no merged counts.
"""
import json, os, sys
from datetime import datetime, timezone

PARTIAL_LABEL = "partial observation, not a result of record"
OWNER_LABEL = "owner supplied frames, not the original, not a parity result"
REF_EMULATED = "emulated original"
REF_OWNER = "owner supplied frames"
DEPTHS = (8, 16, 32)
BYTES_PER_PIXEL = {8: 4, 16: 8, 32: 16}  # rgba, rgba64le, gbrapf32le (docs/handoff.md frame contract)
FRAME_MAX_BYTES = 4 * 1024 ** 3  # a frame is at most 4 GiB
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


def _int(x, lo=0):
    return isinstance(x, int) and not isinstance(x, bool) and x >= lo


def depth_rule(depth, comparator):
    """How the comparator applies at one frame depth (docs/parity.md). Depth 8 and 16 samples are unsigned
    integers: tolerance_ulp counts integer code steps (a whole number), and the NaN and signed zero rules
    do not apply because those samples cannot hold either. Depth 32 samples are floats: tolerance_ulp counts
    float ulps and the comparator's nan_rule and signed_zero apply as written. Returns a one line text."""
    if depth in (8, 16):
        return "depth %d: integer samples, tolerance %s code steps, NaN and signed zero rules do not apply" % (
            depth, _g(comparator, "tolerance_ulp"))
    if depth == 32:
        return "depth 32: float samples, tolerance %s ulp, NaN rule %s, signed zero %s" % (
            _g(comparator, "tolerance_ulp"), _g(comparator, "nan_rule", default="?"), _g(comparator, "signed_zero", default="?"))
    raise ValueError("depth %r is not 8, 16 or 32" % (depth,))


def frames_problems(r):
    """Problems of the optional frames block (empty list when absent or valid)."""
    f = r.get("frames")
    if f is None:
        return []
    if not isinstance(f, dict):
        return ["frames is not an object"]
    bad = []
    if f.get("reference_kind") not in (REF_EMULATED, REF_OWNER):
        bad.append("frames.reference_kind is not one of the two allowed values")
    g = f.get("geometries")
    tot_c = tot_i = 0
    if not isinstance(g, list):
        bad.append("frames.geometries is not a list")
    else:
        for e in g:
            ok = (isinstance(e, dict) and _int(e.get("width"), 1) and _int(e.get("height"), 1)
                  and e.get("depth") in DEPTHS and not isinstance(e.get("depth"), bool)
                  and _int(e.get("frames_compared")) and _int(e.get("frames_identical")))
            if not ok:
                bad.append("a frames geometry entry is malformed")
                continue
            if e["frames_identical"] > e["frames_compared"]:
                bad.append("a frames geometry entry has identical > compared")
            if e["width"] * e["height"] * BYTES_PER_PIXEL[e["depth"]] > FRAME_MAX_BYTES:
                bad.append("a frames geometry entry is above 4 GiB per frame")
            tot_c += e["frames_compared"]
            tot_i += e["frames_identical"]
        keys = [(e.get("width"), e.get("height"), e.get("depth")) for e in g if isinstance(e, dict)]
        if len(set(map(repr, keys))) != len(keys):
            bad.append("frames geometry listed twice")
    fc, fi = f.get("frames_compared"), f.get("frames_identical")
    if not (_int(fc) and _int(fi)):
        bad.append("frames counts are not non-negative integers")
        return bad
    if fi > fc:
        bad.append("frames_identical > frames_compared")
    if isinstance(g, list) and (fc != tot_c or fi != tot_i):
        bad.append("frames counts differ from the sum over the geometries")
    if isinstance(g, list) and fc > 0 and not g:
        bad.append("frames compared with no geometry listed")
    c = r.get("cases", {})
    if _int(c.get("compared")) and c["compared"] > fc:
        bad.append("cases compared > frames compared")
    if fi < fc and c.get("differing") == 0:
        bad.append("frames differ but no case differs")
    if fi == fc and c.get("differing", 0) > 0:
        bad.append("a case differs but all frames are identical")
    tol = _g(r, "comparator", "tolerance_ulp")
    ds = [e.get("depth") for e in g if isinstance(e, dict)] if isinstance(g, list) else []
    if ds and any(d in (8, 16) for d in ds) and not (isinstance(tol, (int, float)) and not isinstance(tol, bool) and float(tol).is_integer() and tol >= 0):
        bad.append("integer depths need a whole number tolerance")
    return bad


def owner_supplied(r):
    return isinstance(r.get("frames"), dict) and r["frames"].get("reference_kind") == REF_OWNER


def invariants(r):
    """Problems that make a result file invalid. An empty list means the file is valid."""
    c = r["cases"]
    bad = []
    rb = r.get("rows_basis")
    if rb is not None:
        ints = isinstance(rb, dict) and all(isinstance(rb.get(k), int) and not isinstance(rb.get(k), bool) and rb.get(k) >= 0
                                            for k in ("rows_total", "rows_read", "rows_observed"))
        if not ints:
            bad.append("rows_basis counts are not non-negative integers")
        elif rb["rows_total"] < 1 or rb["rows_read"] + rb["rows_observed"] != rb["rows_total"]:
            bad.append("rows_read + rows_observed != rows_total")
        if isinstance(rb, dict) and rb.get("observation_label") != PARTIAL_LABEL:
            bad.append("rows_basis observation_label missing")
    bad += frames_problems(r)
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
    if r.get("rows_basis") is not None:
        return None, "rows partly read and partly observed, never parity"
    if owner_supplied(r):
        return None, "owner supplied frames, never parity"
    if _g(r, "comparator", "tolerance_ulp") != 0:
        return None, "tolerance not 0"
    if c["compared"] == 0:
        return None, "0 cases compared"
    f = r.get("frames")
    if f is not None:
        if _g(f, "frames_compared") == 0:
            return None, "0 frames compared"
        if _g(f, "frames_identical") != _g(f, "frames_compared"):
            return None, "differing frames"
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
    if r.get("rows_basis") is not None:
        return "Read and partly observed"  # never a parity tier, whatever the rung and the counts
    if owner_supplied(r):
        return "Owner supplied frames"  # never Matches or Verified, whatever the counts
    c, v, cr = r["cases"], r["values"], r["call_records"]
    f = r.get("frames")
    clean = (c["compared"] >= CASE_FLOOR and c["compared"] == c["in_contract"] and r["comparator"]["tolerance_ulp"] == 0
             and c["differing"] == 0 and v["differing"] == 0
             and cr["compared"] >= 1 and cr["matching"] == cr["compared"]  # a check with nothing in it is unmet
             and r.get("rung") is not None  # a file with no rung is not a parity unit
             and (f is None or (f["frames_compared"] >= 1 and f["frames_identical"] == f["frames_compared"])))
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
        fr = r.get("frames")
        fs = ", %d of %d compared frames identical" % (fr["frames_identical"], fr["frames_compared"]) if fr else ""
        out += " | matches the emulated original (rung %s, %d of %d compared cases identical%s, judge %s)" % (
            r.get("rung"), c["identical"], c["compared"], fs, j)
    if owner_supplied(r):
        fr = r["frames"]
        out += " | %s: %d of %d frames equal to the supplied frames" % (OWNER_LABEL, fr["frames_identical"], fr["frames_compared"])
    if tier == "Read and partly observed":
        rb = r["rows_basis"]
        out += " | %d of %d rows read, %d observed in a %s" % (rb["rows_read"], rb["rows_total"], rb["rows_observed"], PARTIAL_LABEL)
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
    owner = [r for r in results if owner_supplied(r) and not frames_problems(r) and r.get("rows_basis") is None]
    mixed = [r for r in results if r.get("rows_basis") is not None]
    units = [r for r in results if r.get("rung") is not None and r.get("rows_basis") is None and r not in owner]
    apart = [r for r in results if r.get("rung") is None and r.get("rows_basis") is None and r not in owner]
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
    for r in mixed:
        rb = r["rows_basis"]
        out.append("not a parity unit: %s has %d rows read and %d rows observed in a %s" % (
            r["unit"], _g(rb, "rows_read"), _g(rb, "rows_observed"), PARTIAL_LABEL))
    for r in owner:
        out.append("not a parity unit: %s has %d frames compared against %s" % (
            r["unit"], r["frames"]["frames_compared"], OWNER_LABEL))
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
