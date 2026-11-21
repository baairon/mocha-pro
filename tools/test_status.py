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
        self.assertEqual(optional, {"contract", "value_classes_allowed", "pairing", "calibration"})


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
