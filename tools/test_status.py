import contextlib
import copy
import json
import os
import re
import subprocess
import sys
import shutil
import tempfile
import unittest
from pathlib import Path

HERE = str(Path(__file__).resolve().parent)
sys.path.insert(0, HERE)


@contextlib.contextmanager
def scratch_dir():
    path = tempfile.mkdtemp()
    try:
        yield path
    finally:
        shutil.rmtree(path, ignore_errors=True)


import status  # noqa: E402

SCHEMA = os.path.join(os.path.dirname(HERE), "docs", "data", "result_schema.json")


def load_schema():
    with open(SCHEMA, "rb") as h:
        return json.loads(h.read().decode("utf-8"))


def base():
    return {"unit": "u1", "rung": "K-PARITY", "entry": "e", "route": "r",
            "judge": {"engine": "engine-a 1.0", "manual_follows": True, "cross_checked": False},
            "comparator": {"tolerance_ulp": 0, "nan_rule": "class", "signed_zero": "bits"},
            "original": {"artifact_sha256": "0" * 64, "data_sources": []},
            "port": {"source_sha256": "0" * 64, "build_profile": "release"},
            "seeds": ["1"],
            "cases": {"in_contract": 0, "compared": 0, "identical": 0, "differing": 0,
                      "out_of_contract": 0, "blocked": 0, "logic_parity": 0},
            "values": {"compared": 0, "differing": 0, "max_ulp": 0, "nan_class_equal_payload_differs": 0},
            "branches": {"total": 0, "both_ways": 0, "unreached": []},
            "value_classes": [], "sites": [],
            "mutants": {"total": 0, "killed": 0, "control_matches_release": True, "survivors": []},
            "call_records": {"compared": 0, "matching": 0, "not_recordable": []},
            "stand_ins": [], "harness_stand_ins": [], "qualifiers": [], "not_rebuilt": [],
            "commands": [], "measured_at": "2026-11-17T00:00:00Z", "blockers": []}


def counted(n=300, vals=20000):
    r = base()
    r["cases"].update(in_contract=n, compared=n, identical=n)
    r["values"]["compared"] = vals
    r["branches"] = {"total": 2, "both_ways": 2, "unreached": []}
    r["value_classes"] = ["finite"]
    r["value_classes_allowed"] = ["finite"]
    r["mutants"].update(total=10, killed=10)
    r["call_records"].update(compared=3, matching=3)
    return r


FORBIDDEN = [r"\bdone\b", r"\bcomplete\b", r"bit[ -]exact", r"%", r"percent"]


class T(unittest.TestCase):
    def test_empty_run_no_label(self):
        r = base()
        self.assertIsNone(status.rung_label(r)[0])
        self.assertIn("rung label: none", status.line(r))
        self.assertEqual(status.module_tier(r), "In progress")

    def test_tolerance_nonzero_no_label(self):
        r = counted(); r["comparator"]["tolerance_ulp"] = 1
        self.assertIsNone(status.rung_label(r)[0])
        self.assertNotEqual(status.module_tier(r), "Verified")

    def test_one_blocker_is_blocked(self):
        r = counted(); r["blockers"] = ["G1: licence state | record: a.json sha256: " + "a" * 64]
        self.assertEqual(status.module_tier(r), "Blocked")
        self.assertNotIn("matches the emulated original", status.line(r))

    def test_all_identical_is_verified_and_k_parity(self):
        r = counted()
        self.assertEqual(status.module_tier(r), "Verified")
        self.assertEqual(status.rung_label(r)[0], "K-PARITY")
        self.assertIn("matches the emulated original (rung K-PARITY, 300 of 300", status.line(r))

    def test_no_survivor_rule_is_matches(self):
        r = counted(); r["mutants"]["total"] = 0
        self.assertEqual(status.module_tier(r), "Matches")

    def test_thin(self):
        r = counted(); r["values"]["compared"] = 5000
        lab, note = status.rung_label(r)
        self.assertEqual(lab, "K-PARITY-THIN"); self.assertIn("unmet", note)

    def test_below_case_floor_in_progress(self):
        r = counted(100)
        self.assertEqual(status.module_tier(r), "In progress")

    def test_differing_not_matches(self):
        r = counted(); r["cases"]["differing"] = 1; r["cases"]["identical"] = 299
        self.assertEqual(status.module_tier(r), "In progress")
        self.assertIsNone(status.rung_label(r)[0])

    def test_rec_parity(self):
        r = counted(); r["rung"] = "REC-PARITY"
        self.assertEqual(status.rung_label(r)[0], "REC-PARITY")

    def test_totals_both_tiers(self):
        v, m, e = counted(), counted(), base(); m["mutants"]["total"] = 0
        self.assertEqual(status.totals([v, m, e]),
                         "totals, rung K-PARITY: 2 of 3 modules match the emulated original, 1 of 3 verified (judge: emulator)")

    def test_totals_one_line_per_rung_never_merged(self):
        a, b = counted(), counted(); b["rung"] = "REC-PARITY"
        out = status.totals([a, b, base()])
        self.assertEqual(out.splitlines(), [
            "totals, rung K-PARITY: 1 of 2 modules match the emulated original, 1 of 2 verified (judge: emulator)",
            "totals, rung REC-PARITY: 1 of 1 modules match the emulated original, 1 of 1 verified (judge: emulator)"])
        self.assertNotIn("of 3", out)

    def test_totals_none(self):
        self.assertEqual(status.totals([]), "totals: none (0 cases compared)")
        self.assertEqual(status.report([]), "totals: none (0 cases compared)")

    def test_forbidden_words_never_appear(self):
        b = counted(); b["blockers"] = ["G1: x | record: a sha256: " + "b" * 64]
        out = status.report([base(), counted(), b, counted(100)])
        for pat in FORBIDDEN:
            self.assertIsNone(re.search(pat, out, re.I), pat)


def thin_misses(r):
    return status.k_floor_misses(r)


class StatusRuleDetails(unittest.TestCase):
    # every allowed value class must be present
    def test_allowed_missing_is_unmet(self):
        r = counted(); del r["value_classes_allowed"]
        self.assertEqual(status.rung_label(r)[0], "K-PARITY-THIN")

    def test_allowed_member_absent_is_unmet(self):
        r = counted(); r["value_classes_allowed"] = ["finite", "nan+"]
        self.assertEqual(status.rung_label(r)[0], "K-PARITY-THIN")

    def test_allowed_all_present_is_k(self):
        r = counted(); r["value_classes_allowed"] = ["finite"]; r["value_classes"] = ["finite", "+0"]
        self.assertEqual(status.rung_label(r)[0], "K-PARITY")

    # the values floor, and the 500 case floor for a contract with small cases
    def test_values_floor_applies_without_a_small_case_contract(self):
        r = counted(500, 8000)  # 8000 values miss the 10000 floor when no contract declares small cases
        self.assertEqual(status.rung_label(r)[0], "K-PARITY-THIN")

    def test_small_cases_500_floor(self):
        r = counted(500, 8000); r["contract"] = {"max_values_per_case": 16}
        self.assertEqual(status.rung_label(r)[0], "K-PARITY")
        r2 = counted(300, 4800); r2["contract"] = {"max_values_per_case": 16}
        self.assertEqual(status.rung_label(r2)[0], "K-PARITY-THIN")

    def test_null_or_large_max_uses_200_10000(self):
        for m in (None, 17):
            r = counted(500, 8000); r["contract"] = {"max_values_per_case": m}
            self.assertEqual(status.rung_label(r)[0], "K-PARITY-THIN")

    # a result file that contradicts itself is invalid; out-of-contract cases do not count
    def test_invariant_sum_invalid(self):
        r = counted(); r["cases"]["identical"] = 290
        self.assertIn("invalid result file", status.line(r))
        self.assertIsNone(status.module_tier(r))

    def test_invariant_compared_over_in_contract_invalid(self):
        r = counted(); r["cases"]["in_contract"] = 299
        self.assertIn("invalid result file", status.line(r))
        self.assertNotIn("Verified", status.line(r))

    def test_verified_needs_all_in_contract_compared(self):
        r = counted(); r["cases"]["in_contract"] = 400
        self.assertEqual(status.module_tier(r), "In progress")

    def test_out_of_contract_does_not_count(self):
        r = counted(100); r["cases"]["out_of_contract"] = 500
        self.assertEqual(status.module_tier(r), "In progress")

    # the survivor rule
    def test_survivor_added_case(self):
        r = counted(); r["mutants"]["survivors"] = [{"id": "m1", "killed_by_added_case": "c7"}]
        self.assertEqual(status.module_tier(r), "Verified")

    def test_survivor_argument_without_reviewer_fails(self):
        r = counted(); r["mutants"]["survivors"] = [{"id": "m1", "argument": "equivalent", "reviewed_by": ""}]
        self.assertEqual(status.module_tier(r), "Matches")

    def test_survivor_reviewer_without_argument_fails(self):
        r = counted(); r["mutants"]["survivors"] = [{"id": "m1", "argument": "", "reviewed_by": "reviewer-a"}]
        self.assertEqual(status.module_tier(r), "Matches")

    def test_survivor_argument_and_reviewer(self):
        r = counted(); r["mutants"]["survivors"] = [{"id": "m1", "argument": "equivalent", "reviewed_by": "reviewer-a"}]
        self.assertEqual(status.module_tier(r), "Verified")

    def test_ratio_uses_outright_kills(self):
        r = counted(); r["mutants"].update(total=10, killed=8, survivors=[{"id": "a", "killed_by_added_case": "c"}] * 2)
        self.assertEqual(status.module_tier(r), "Matches")

    # independent re-runs
    def rr(self, **k):
        d = {"unit": "u1", "matches": True, "differences": [], "reran_at": "2026-11-18T00:00:00Z",
             "regenerated_oracle": True, "seeds_used": []}
        d.update(k); return d

    def test_rerun_yes(self):
        self.assertEqual(status.rerun_state(counted(), self.rr()), "yes")
        self.assertIn("re-run verified", status.line(counted(), self.rr()))

    def test_rerun_no_count(self):
        d = self.rr(matches=False, differences=["a", "b"])
        self.assertEqual(status.rerun_state(counted(), d), "no (2 differences)")
        self.assertNotIn("re-run verified", status.line(counted(), d))

    def test_rerun_stale(self):
        d = self.rr(reran_at="2026-11-16T00:00:00Z")
        self.assertEqual(status.rerun_state(counted(), d), "stale")
        self.assertNotIn("re-run verified", status.line(counted(), d))

    def test_rerun_same_date_or_unreadable_date_is_stale(self):
        same = self.rr(reran_at="2026-11-17T00:00:00Z")
        self.assertEqual(status.rerun_state(counted(), same), "stale")
        self.assertEqual(status.rerun_state(counted(), self.rr(reran_at="not a date")), "stale")
        r = counted(); r["measured_at"] = "not a date"
        self.assertEqual(status.rerun_state(r, self.rr()), "stale")

    def test_rerun_none(self):
        self.assertEqual(status.rerun_state(counted(), None), "none")
        self.assertNotIn("re-run verified", status.line(counted()))

    def test_rerun_file_reader(self):
        import tempfile, os, json, subprocess, sys
        with scratch_dir() as d:
            p = os.path.join(d, "u1.json")
            with open(p, "w") as h:
                json.dump(counted(), h)
            with open(p + ".rerun.json", "w") as h:
                json.dump(self.rr(), h)
            out = subprocess.run([sys.executable, status.__file__, p], capture_output=True, text=True).stdout
        self.assertIn("re-run verified", out)

    # a file with only the interface pairing and the calibration
    def test_pairing_only_file_has_no_label(self):
        r = base(); r["rung"] = None
        r["pairing"] = {"params_original": 10, "params_rewrite": 10, "exact": 8, "normalized": 2, "unmatched": 0}
        r["calibration"] = {"boots": 2, "records_identical": True, "dumps_identical": True,
                            "record_sha256": ["a" * 64, "a" * 64]}
        self.assertIsNone(status.rung_label(r)[0])
        self.assertIn("rung label: none", status.line(r))
        self.assertNotIn("Verified", status.line(r))

    def test_schema_accepts_the_optional_fields(self):
        import json, os
        sch = load_schema()
        for k in ("pairing", "calibration", "contract", "value_classes_allowed"):
            self.assertIn(k, sch["properties"])
        self.assertIn(None, sch["properties"]["rung"]["enum"])


class EmptyCallRecords(unittest.TestCase):
    """A call-record check with 0 records compared is unmet; a result file with no rung is listed on a line of its own."""
    def pairing_only_file(self):
        r = base(); r["rung"] = None; r["unit"] = "u2"
        r["pairing"] = {"params_original": 6, "params_rewrite": 6, "exact": 6, "normalized": 0, "unmatched": 0}
        r["calibration"] = {"boots": 2, "records_identical": True, "dumps_identical": True, "record_sha256": ["a" * 64] * 2}
        return r

    def test_zero_call_records_is_not_matches(self):
        r = counted(); r["call_records"].update(compared=0, matching=0)
        self.assertEqual(status.module_tier(r), "In progress")
        self.assertNotIn("matches the emulated original", status.line(r))
        self.assertIn("0 of 1 modules match", status.totals([r]))

    def test_zero_call_records_is_not_verified(self):
        r = counted(); r["call_records"].update(compared=0, matching=0)
        self.assertNotEqual(status.module_tier(r), "Verified")

    def test_one_call_record_still_verifies(self):
        r = counted(); r["call_records"].update(compared=1, matching=1)
        self.assertEqual(status.module_tier(r), "Verified")

    def test_null_rung_never_prints_rung_none(self):
        out = status.totals([self.pairing_only_file()])
        self.assertNotIn("rung None", out)
        self.assertEqual(out, "not a parity unit: u2 carries pairing and calibration only, has no rung and 0 cases compared")

    def test_null_rung_apart_from_rung_lines(self):
        out = status.totals([counted(), self.pairing_only_file()]).splitlines()
        self.assertEqual(out, [
            "totals, rung K-PARITY: 1 of 1 modules match the emulated original, 1 of 1 verified (judge: emulator)",
            "not a parity unit: u2 carries pairing and calibration only, has no rung and 0 cases compared"])

    def test_null_rung_wording_has_no_match_or_verified(self):
        r = self.pairing_only_file()
        for text in (status.totals([r]), status.line(r)):
            self.assertNotRegex(text, r"(?i)\bmatch|verified|rung None")

    def test_null_rung_with_cases_cannot_match(self):
        r = counted(); r["rung"] = None
        self.assertNotIn(status.module_tier(r), ("Matches", "Verified"))
        self.assertNotIn("matches the emulated original", status.line(r))
        self.assertNotIn("1 of 1 modules match", status.totals([r]))

    def test_schema_is_closed_and_has_only_the_known_optional_fields(self):
        sch = load_schema()
        self.assertIs(sch["additionalProperties"], False)
        optional = set(sch["properties"]) - set(sch["required"])
        self.assertEqual(optional, {"contract", "value_classes_allowed", "pairing", "calibration", "rows_basis", "frames"})


def mixed(read=3, obs=2, rung=None):
    r = counted() if rung else base()
    r["rung"] = rung
    r["rows_basis"] = {"rows_total": read + obs, "rows_read": read, "rows_observed": obs,
                       "observation_label": "partial observation, not a result of record"}
    return r


class MixedRows(unittest.TestCase):
    def test_state_and_wording(self):
        r = mixed()
        self.assertEqual(status.module_tier(r), "Read and partly observed")
        out = status.line(r)
        self.assertIn("3 of 5 rows read, 2 observed in a partial observation, not a result of record", out)
        self.assertNotRegex(out, r"(?i)matches the emulated|verified|rung None|%")

    def test_never_parity_even_with_perfect_counts(self):
        r = mixed(rung="K-PARITY")
        self.assertEqual(status.module_tier(r), "Read and partly observed")
        self.assertIsNone(status.rung_label(r)[0])
        self.assertNotIn("matches the emulated original", status.line(r))
        self.assertNotIn("1 of 1 modules", status.totals([r]))

    def test_totals_line_apart(self):
        out = status.totals([counted(), mixed()]).splitlines()
        self.assertEqual(out[-1], "not a parity unit: u1 has 3 rows read and 2 rows observed in a partial observation, not a result of record")
        self.assertEqual(len(out), 2)

    def test_bad_counts_invalid(self):
        r = mixed(); r["rows_basis"]["rows_read"] = 4
        self.assertIn("invalid result file", status.line(r))
        r = mixed(); r["rows_basis"]["observation_label"] = "x"
        self.assertIn("invalid result file", status.line(r))
        r = mixed(); r["rows_basis"]["rows_read"] = True
        self.assertIn("invalid result file", status.line(r))

    def test_sum_below_total_invalid(self):
        r = mixed(); r["rows_basis"]["rows_total"] = 9
        self.assertIn("invalid result file", status.line(r))

    def test_total_under_one_invalid(self):
        r = mixed(read=0, obs=0)
        self.assertEqual(r["rows_basis"]["rows_total"], 0)
        self.assertIn("invalid result file", status.line(r))

    def test_boolean_counts_invalid(self):
        r = mixed(read=1, obs=0)
        r["rows_basis"]["rows_read"] = True
        self.assertIn("invalid result file", status.line(r))
        r = mixed(read=1, obs=0)
        r["rows_basis"]["rows_total"] = True
        self.assertIn("invalid result file", status.line(r))

    def test_mixed_unit_never_enters_a_rung_line(self):
        r = mixed(rung="K-PARITY")
        out = status.totals([r])
        self.assertNotIn("totals, rung", out)
        self.assertEqual(len(out.splitlines()), 1)
        self.assertTrue(out.startswith("not a parity unit: u1 has 3 rows read"))
        self.assertEqual(status.totals([counted(), r]).count("totals, rung"), 1)

    def test_schema_const_and_minimum(self):
        p = load_schema()["properties"]["rows_basis"]["properties"]
        self.assertEqual(p["observation_label"]["const"], status.PARTIAL_LABEL)
        self.assertEqual(p["rows_total"]["minimum"], 1)

    def test_blockers_still_win(self):
        r = mixed(); r["blockers"] = ["G1: c | record: p sha256: " + "0" * 64]
        self.assertEqual(status.module_tier(r), "Blocked")

    def test_schema_shape(self):
        p = load_schema()["properties"]["rows_basis"]
        self.assertEqual(set(p["required"]), set(mixed()["rows_basis"]))
        self.assertIs(p["additionalProperties"], False)


def pix(kind="emulated original", n=300, per=2, depth=8, ident=None):
    """A made up, generic pixel unit (self test of the tools, no claim about any plug-in)."""
    r = counted(n)
    tot = n * per
    ident = tot if ident is None else ident
    r["frames"] = {"reference_kind": kind, "frames_compared": tot, "frames_identical": ident,
                   "geometries": [{"width": 64, "height": 32, "depth": depth, "frames_compared": tot, "frames_identical": ident}]}
    if ident < tot:
        r["cases"].update(identical=n - 1, differing=1)
    return r


class PixelFrames(unittest.TestCase):
    """The optional frames block: generic tools and a pass through style self test."""

    def test_absent_block_changes_nothing(self):
        r = counted()
        self.assertEqual(status.module_tier(r), "Verified")
        self.assertNotIn("frames identical", status.line(r))

    def test_emulated_all_identical_verifies_and_names_frames(self):
        r = pix()
        self.assertEqual(status.module_tier(r), "Verified")
        self.assertIn("300 of 300 compared cases identical, 600 of 600 compared frames identical, judge", status.line(r))

    def test_owner_supplied_never_verified_or_matches(self):
        r = pix("owner supplied frames")
        self.assertEqual(status.module_tier(r), "Owner supplied frames")
        self.assertIsNone(status.rung_label(r)[0])
        self.assertNotIn("matches the emulated original", status.line(r))

    def test_owner_supplied_prints_fixed_label(self):
        out = status.line(pix("owner supplied frames"))
        self.assertIn(status.OWNER_LABEL, out)
        self.assertIn("600 of 600 frames equal to the supplied frames", out)
        self.assertNotRegex(out, r"(?i)\bmatch|verified|%")

    def test_owner_supplied_totals_apart(self):
        out = status.totals([counted(), pix("owner supplied frames")]).splitlines()
        self.assertEqual(out[0], "totals, rung K-PARITY: 1 of 1 modules match the emulated original, 1 of 1 verified (judge: emulator)")
        self.assertEqual(out[1], "not a parity unit: u1 has 600 frames compared against " + status.OWNER_LABEL)
        self.assertEqual(len(out), 2)

    def test_owner_supplied_with_perfect_everything_still_below(self):
        r = pix("owner supplied frames"); r["rung"] = "REC-PARITY"
        self.assertEqual(status.module_tier(r), "Owner supplied frames")
        self.assertNotIn("0 of 1", status.totals([r]))
        self.assertNotIn("modules match", status.totals([r]))

    def test_owner_supplied_with_blocker_still_labelled(self):
        r = pix("owner supplied frames"); r["blockers"] = ["G1: c | record: p sha256: " + "0" * 64]
        self.assertEqual(status.module_tier(r), "Blocked")
        self.assertIn(status.OWNER_LABEL, status.line(r))

    def test_reference_kind_only_two_values(self):
        r = pix(); r["frames"]["reference_kind"] = "reference"
        self.assertIn("invalid result file", status.line(r))
        r = pix(); del r["frames"]["reference_kind"]
        self.assertIn("invalid result file", status.line(r))
        enum = load_schema()["properties"]["frames"]["properties"]["reference_kind"]["enum"]
        self.assertEqual(enum, ["emulated original", "owner supplied frames"])

    def test_identical_above_compared_invalid(self):
        r = pix(); r["frames"]["frames_identical"] = 601
        self.assertIn("frames_identical > frames_compared", status.line(r))

    def test_entry_identical_above_compared_invalid(self):
        r = pix(); r["frames"]["geometries"][0]["frames_identical"] = 601
        self.assertIn("identical > compared", status.line(r))

    def test_sum_over_geometries_must_equal(self):
        r = pix(); r["frames"]["frames_compared"] = 700
        self.assertIn("differ from the sum", status.line(r))
        r = pix(); r["frames"]["geometries"].append(
            {"width": 8, "height": 8, "depth": 16, "frames_compared": 3, "frames_identical": 3})
        self.assertIn("differ from the sum", status.line(r))

    def test_two_geometries_sum_and_verify(self):
        r = pix()
        r["frames"]["geometries"][0].update(frames_compared=503, frames_identical=503)
        r["frames"]["geometries"].append({"width": 8, "height": 8, "depth": 32, "frames_compared": 97, "frames_identical": 97})
        self.assertEqual(status.invariants(r), [])
        self.assertEqual(status.module_tier(r), "Verified")

    def test_repeated_geometry_invalid(self):
        r = pix(); r["frames"]["geometries"] = r["frames"]["geometries"] * 2
        self.assertTrue(status.invariants(r))

    def test_bad_geometry_entries_invalid(self):
        for k, v in (("depth", 12), ("width", 0), ("height", 1.5), ("frames_compared", -1), ("depth", True)):
            r = pix(); r["frames"]["geometries"][0][k] = v
            self.assertTrue(status.invariants(r), (k, v))

    def test_counts_must_be_integers(self):
        r = pix(); r["frames"]["frames_compared"] = True
        self.assertTrue(status.invariants(r))
        r = pix(); r["frames"]["frames_identical"] = "5"
        self.assertTrue(status.invariants(r))

    def test_frames_without_geometry_invalid(self):
        r = pix(); r["frames"]["geometries"] = []
        self.assertTrue(status.invariants(r))

    def test_cases_cannot_exceed_frames(self):
        r = pix(per=1); r["frames"]["frames_compared"] = r["frames"]["frames_identical"] = 299
        r["frames"]["geometries"][0].update(frames_compared=299, frames_identical=299)
        self.assertIn("cases compared > frames compared", status.line(r))

    def test_differing_frames_need_a_differing_case(self):
        r = pix(ident=599); r["cases"].update(identical=300, differing=0)
        self.assertIn("frames differ but no case differs", status.line(r))

    def test_differing_case_with_identical_frames_invalid(self):
        r = pix(); r["cases"].update(identical=299, differing=1)
        self.assertIn("a case differs but all frames are identical", status.line(r))

    def test_differing_frames_below_matches(self):
        r = pix(ident=599)
        self.assertEqual(status.module_tier(r), "In progress")
        self.assertIsNone(status.rung_label(r)[0])

    def test_zero_frames_not_matches(self):
        r = pix(); r["frames"].update(frames_compared=0, frames_identical=0)
        r["frames"]["geometries"][0].update(frames_compared=0, frames_identical=0)
        r["cases"].update(in_contract=0, compared=0, identical=0)
        self.assertNotIn(status.module_tier(r), ("Matches", "Verified"))

    def test_depth_rules_per_depth(self):
        cmp_ = {"tolerance_ulp": 0, "nan_rule": "class", "signed_zero": "bits"}
        for d in (8, 16):
            t = status.depth_rule(d, cmp_)
            self.assertIn("NaN and signed zero rules do not apply", t)
            self.assertIn("integer samples", t)
        t = status.depth_rule(32, cmp_)
        self.assertIn("NaN rule class", t); self.assertIn("signed zero bits", t)
        with self.assertRaises(ValueError):
            status.depth_rule(10, cmp_)

    def test_integer_depth_needs_whole_tolerance(self):
        r = pix(depth=8); r["comparator"]["tolerance_ulp"] = 0.5
        self.assertIn("whole number tolerance", status.line(r))
        r = pix(depth=32); r["comparator"]["tolerance_ulp"] = 0.5
        self.assertEqual(status.invariants(r), [])

    def test_frame_above_4_gib_invalid(self):
        r = pix(depth=32); r["frames"]["geometries"][0].update(width=16384, height=16385)  # 16 B per pixel, just above 2**32
        self.assertIn("above 4 GiB", " ".join(status.invariants(r)))
        r = pix(depth=32); r["frames"]["geometries"][0].update(width=16384, height=16384)  # exactly 4 GiB
        self.assertEqual(status.invariants(r), [])
        r = pix(depth=8); r["frames"]["geometries"][0].update(width=16384, height=16385)  # 4 B per pixel, fine
        self.assertEqual(status.invariants(r), [])

    def test_mixed_depths_need_whole_tolerance(self):
        r = pix(depth=8); r["comparator"]["tolerance_ulp"] = 0.5
        r["frames"]["geometries"][0].update(frames_compared=300, frames_identical=300)
        r["frames"]["geometries"].append({"width": 4, "height": 4, "depth": 32, "frames_compared": 300, "frames_identical": 300})
        self.assertIn("whole number tolerance", " ".join(status.invariants(r)))

    def test_negative_tolerance_invalid_with_frames(self):
        r = pix(depth=8); r["comparator"]["tolerance_ulp"] = -1
        self.assertIn("whole number tolerance", " ".join(status.invariants(r)))

    def test_owner_label_text_is_fixed(self):
        self.assertEqual(status.OWNER_LABEL, "owner supplied frames, not the original, not a parity result")

    def test_identical_sum_must_equal(self):
        r = pix(); r["frames"]["geometries"][0]["frames_identical"] = 599
        self.assertIn("differ from the sum", " ".join(status.invariants(r)))

    def test_rows_basis_wins_over_owner_supplied(self):
        r = pix("owner supplied frames"); r["rows_basis"] = mixed()["rows_basis"]
        self.assertEqual(status.module_tier(r), "Read and partly observed")

    def test_nonzero_tolerance_still_no_label_with_frames(self):
        r = pix(depth=16); r["comparator"]["tolerance_ulp"] = 1
        self.assertIsNone(status.rung_label(r)[0])

    def test_schema_block_is_closed_and_complete(self):
        p = load_schema()["properties"]["frames"]
        self.assertIs(p["additionalProperties"], False)
        self.assertEqual(set(p["required"]), set(pix()["frames"]))
        g = p["properties"]["geometries"]["items"]
        self.assertIs(g["additionalProperties"], False)
        self.assertEqual(set(g["required"]), set(pix()["frames"]["geometries"][0]))
        self.assertEqual(g["properties"]["depth"]["enum"], [8, 16, 32])
        self.assertNotIn("frames", load_schema()["required"])

    def test_forbidden_words_with_frames(self):
        out = status.report([pix(), pix("owner supplied frames"), pix(ident=5)])
        for pat in FORBIDDEN:
            self.assertIsNone(re.search(pat, out, re.I), pat)

    def test_cli_prints_owner_label(self):
        with scratch_dir() as d:
            p = os.path.join(d, "u.json")
            with open(p, "w") as h:
                json.dump(pix("owner supplied frames"), h)
            q = subprocess.run([sys.executable, "-I", status.__file__, p], capture_output=True, text=True)
        self.assertEqual(q.returncode, 0)
        self.assertIn(status.OWNER_LABEL, q.stdout)


class Messages(unittest.TestCase):
    def test_unsupported_rung_message(self):
        r = counted()
        r["rung"] = "OTHER-RUNG"
        label, note = status.rung_label(r)
        self.assertIsNone(label)
        for word in ("REC-PARITY", "K-PARITY", "K-PARITY-THIN", "any other rung is not computed"):
            self.assertIn(word, note)
        self.assertNotIn("README", note)


class ExitCodes(unittest.TestCase):
    def run_status(self, *args):
        p = subprocess.run([sys.executable, "-I", status.__file__] + list(args), capture_output=True, text=True)
        return p.returncode, p.stdout, p.stderr

    def test_success_exit_0(self):
        with scratch_dir() as d:
            p = os.path.join(d, "u1.json")
            with open(p, "w") as h:
                json.dump(counted(), h)
            code, out, err = self.run_status(p)
        self.assertEqual(code, 0)
        self.assertIn("totals, rung K-PARITY", out)
        self.assertEqual(err, "")

    def test_no_file_exit_0(self):
        code, out, _ = self.run_status()
        self.assertEqual(code, 0)
        self.assertIn("Not started", out)

    def test_usage_exit_2(self):
        code, _, err = self.run_status("--help")
        self.assertEqual(code, 2)
        self.assertIn("usage", err)

    def test_missing_file_exit_3(self):
        with scratch_dir() as d:
            p = os.path.join(d, "absent.json")
            code, out, err = self.run_status(p)
        self.assertEqual(code, 3)
        self.assertEqual(out, "")
        self.assertTrue(err.startswith("status: cannot read " + p + ": "))
        self.assertNotIn("Traceback", err)

    def test_not_json_exit_3(self):
        with scratch_dir() as d:
            p = os.path.join(d, "bad.json")
            with open(p, "w") as h:
                h.write("{not json")
            code, _, err = self.run_status(p)
        self.assertEqual(code, 3)
        self.assertIn("status: cannot read " + p + ": ", err)
        self.assertNotIn("Traceback", err)

    def test_bad_rerun_file_exit_3(self):
        with scratch_dir() as d:
            p = os.path.join(d, "u1.json")
            with open(p, "w") as h:
                json.dump(counted(), h)
            with open(p + ".rerun.json", "w") as h:
                h.write("[")
            code, _, err = self.run_status(p)
        self.assertEqual(code, 3)
        self.assertIn("status: cannot read " + p + ".rerun.json: ", err)

    def test_not_utf8_exit_3(self):
        with scratch_dir() as d:
            p = os.path.join(d, "bin.json")
            with open(p, "wb") as h:
                h.write(b"\xff\xfe\x00")
            code, _, err = self.run_status(p)
        self.assertEqual(code, 3)
        self.assertIn("status: cannot read", err)


if __name__ == "__main__":
    unittest.main()
