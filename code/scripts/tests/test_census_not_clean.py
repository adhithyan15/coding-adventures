"""Tests for code/programs/go/kiln/ci/census_not_clean.py (the census soft-pass flags)."""

import importlib.util
import json
import pathlib
import tempfile
import unittest

SCRIPT = pathlib.Path(__file__).resolve().parents[2] / "programs/go/kiln/ci/census_not_clean.py"
spec = importlib.util.spec_from_file_location("census_not_clean", SCRIPT)
mod = importlib.util.module_from_spec(spec)
spec.loader.exec_module(mod)


def summary(traces=5, accepted=5, skipped=0, actions_skipped=0):
    return {
        "cross_check": {"traces": traces, "accepted": accepted, "skipped_too_large": skipped},
        "bounds": {"actions_skipped": actions_skipped},
    }


SUB_CODES = [
    "exechead-pair", "exechead-kernel", "exechead-outside", "exechead-gone", "exechead-mismatch",
    "exechead-timeout", "exechead-unpinned", "exechead-fexecve", "exechead-same-image",
    "exechead-failed", "exechead-overload", "exechead-tainted", "exechead-notelf",
]
UNCLEAN = ["exechead-handover", "exechead-stall", "exechead-stuck", "exechead-receiver", "exechead-overflow", "exechead-failed"]
PARSE = ["exechead-missing", "exechead-digest", "exechead-malformed", "exechead-unclaimed", "exechead-oversize"]


def exec_head_summary(sealed=True, kernel="7.0.0-1014-aws", trusted=None, **over):
    """A summary of a shard that ran with exec heads, every counter zero unless overridden.

    over may set: sub (dict), unclean (dict), parse (dict), kills, seal_mismatch."""
    s = summary()
    s["host"] = {"kernel": kernel}
    s["collect"] = {"unknown_reasons": {"serial-violated": 0, "exechead-seal": over.get("seal_mismatch", 0), "other": 0}}
    s["exec_head"] = {
        "sealed": sealed,
        "kernel_trusted": trusted if trusted is not None else kernel.startswith("7.0"),
        "supervised_steps": 4,
        "sub_codes": {**{c: 0 for c in SUB_CODES}, **over.get("sub", {})},
        "unclean": {**{c: 0 for c in UNCLEAN}, **over.get("unclean", {})},
        "parse_reasons": {**{c: 0 for c in PARSE}, **over.get("parse", {})},
        "kills": over.get("kills", 0),
        "kill_steps": 0,
        "notifications": 12,
        "go_emittable": 3,
    }
    return s


class ExecHeadCriteriaTests(unittest.TestCase):
    """KILN04 D48 step 6: the hard criteria apply only to a shard that ran with exec heads."""

    def test_a_clean_exec_head_shard_has_no_reasons(self):
        self.assertEqual(mod.reasons(exec_head_summary()), [])
        self.assertTrue(mod.exec_head_ran(exec_head_summary()))

    def test_a_shard_without_exec_heads_is_not_judged_by_them(self):
        # Even a block full of nonzero counters is ignored when the seal is off.
        s = exec_head_summary(
            sealed=False, kills=3, unclean={"exechead-stall": 2}, parse={"exechead-digest": 1}, sub={"exechead-kernel": 9}
        )
        self.assertEqual(mod.reasons(s), [])
        self.assertFalse(mod.exec_head_ran(s))
        # And a summary with no exec_head block at all (an older census) is the same.
        self.assertEqual(mod.reasons(summary()), [])
        self.assertFalse(mod.exec_head_ran(summary()))

    def test_each_hard_criterion_is_flagged(self):
        for name, over, needle in [
            ("unclean", {"unclean": {"exechead-stuck": 1}}, "1 exechead-* unclean reasons"),
            ("every unclean code", {"unclean": {c: 1 for c in UNCLEAN}}, "6 exechead-* unclean reasons"),
            ("kills", {"kills": 2}, "2 supervisor kills"),
            ("parse", {"parse": {"exechead-oversize": 1}}, "1 exechead-* parse reasons"),
            ("every parse code", {"parse": {c: 1 for c in PARSE}}, "5 exechead-* parse reasons"),
            ("seal mismatch", {"seal_mismatch": 3}, "3 exechead-* parse reasons"),
            ("kernel", {"sub": {"exechead-kernel": 4}}, "4 exechead-kernel on a trusted kernel series"),
        ]:
            with self.subTest(name):
                why = mod.reasons(exec_head_summary(**over))
                self.assertEqual(len(why), 1, why)
                self.assertIn(needle, why[0])

    def test_other_sub_codes_are_counted_but_not_hard_failures(self):
        # The thirteen codes are censused; only exechead-kernel on a trusted kernel gates.
        s = exec_head_summary(sub={c: 5 for c in SUB_CODES if c != "exechead-kernel"})
        self.assertEqual(mod.reasons(s), [])

    def test_exechead_kernel_is_allowed_off_the_trusted_table(self):
        s = exec_head_summary(kernel="6.8.0-1020-aws", sub={"exechead-kernel": 4})
        self.assertEqual(mod.reasons(s), [])

    def test_the_trusted_table_is_matched_exactly(self):
        for kernel, trusted in [
            ("7.0.0-1014-aws", True), ("7.0", True), ("7.0.1", True), ("7.0-rc2", True),
            ("7.00", False), ("7.01.0", False), ("7.10.0", False), ("7.0x", False), ("6.7.0", False), ("", False),
        ]:
            with self.subTest(kernel):
                # The summary does not say (kernel_trusted false): the gate reads the release itself.
                s = exec_head_summary(kernel=kernel, trusted=False, sub={"exechead-kernel": 1})
                self.assertEqual(bool(mod.reasons(s)), trusted)

    def test_a_summary_that_says_the_kernel_is_trusted_is_believed(self):
        s = exec_head_summary(kernel="odd", trusted=True, sub={"exechead-kernel": 1})
        self.assertEqual(len(mod.reasons(s)), 1)

    def test_every_failing_criterion_is_reported_together(self):
        s = exec_head_summary(kills=1, unclean={"exechead-stall": 1}, parse={"exechead-missing": 1}, sub={"exechead-kernel": 1})
        self.assertEqual(len(mod.reasons(s)), 4)

    def test_a_sealed_shard_with_a_truncated_block_fails_closed(self):
        import contextlib
        import io

        s = exec_head_summary()
        del s["exec_head"]["unclean"]
        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d, "s.json")
            p.write_text(json.dumps(s))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(mod.main(["x", str(p), "3", "--strict"]), 1)
            self.assertIn("unreadable (KeyError)", out.getvalue())

    def test_main_prints_the_exec_head_reasons_and_a_positive_line(self):
        import contextlib
        import io

        with tempfile.TemporaryDirectory() as d:
            clean, dirty = pathlib.Path(d, "a.json"), pathlib.Path(d, "b.json")
            clean.write_text(json.dumps(exec_head_summary()))
            dirty.write_text(json.dumps(exec_head_summary(kills=1)))
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(mod.main(["x", str(clean), "0", "--strict"]), 0)
                self.assertEqual(mod.main(["x", str(dirty), "1", "--strict"]), 1)
                self.assertEqual(mod.main(["x", str(dirty), "1"]), 0)
            text = out.getvalue()
            self.assertIn("- shard 0: clean (no skipped_too_large, no actions_skipped, not all refused; exec-head criteria clean)", text)
            self.assertIn("- shard 1: **not clean**: 1 supervisor kills", text)


class NotCleanTests(unittest.TestCase):
    def test_clean_has_no_reasons(self):
        self.assertEqual(mod.reasons(summary()), [])

    def test_each_soft_pass_is_flagged(self):
        self.assertEqual(len(mod.reasons(summary(skipped=2))), 1)
        self.assertEqual(len(mod.reasons(summary(actions_skipped=1))), 1)
        self.assertEqual(len(mod.reasons(summary(traces=3, accepted=0))), 1)
        self.assertEqual(len(mod.reasons(summary(skipped=1, actions_skipped=1, accepted=0))), 3)

    def test_additive_census_reporting_fields_do_not_change_the_verdict(self):
        # The census-reporting fields (shapes, shard_void, allowed_rows, ...) are
        # additive: the soft-pass flags read only the fields they always read.
        s = summary()
        s["shapes"] = [{"origin": "refusal:size", "count": 1}]
        s["actions"] = {"shard_void": True, "writes_unknown": 3, "strict_merge_voids_shard": True}
        s["cross_check"]["allowed_rows"] = {"signed pid": 1}
        s["bounds"]["shapes_dropped"] = 9
        self.assertEqual(mod.reasons(s), [])

    def test_no_traces_is_not_all_refused(self):
        self.assertEqual(mod.reasons(summary(traces=0, accepted=0)), [])

    def test_main_prints_and_exits_zero(self):
        with tempfile.TemporaryDirectory() as d:
            good, bad = pathlib.Path(d, "a.json"), pathlib.Path(d, "b.json")
            good.write_text(json.dumps(summary(skipped=1)))
            bad.write_text("{not json")
            self.assertEqual(mod.main(["x", str(good), "0"]), 0)
            self.assertEqual(mod.main(["x", str(bad), "1"]), 0)
            self.assertEqual(mod.main(["x", str(pathlib.Path(d, "missing.json")), "2"]), 0)

    def run_main(self, content: str) -> str:
        import contextlib
        import io

        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d, "c.json")
            p.write_text(content)
            out = io.StringIO()
            with contextlib.redirect_stdout(out):
                self.assertEqual(mod.main(["x", str(p), "7"]), 0)
            return out.getvalue()

    def test_a_clean_shard_prints_a_positive_line(self):
        out = self.run_main(json.dumps(summary()))
        self.assertIn("- shard 7: clean (no skipped_too_large, no actions_skipped, not all refused)", out)
        self.assertNotIn("not clean", out)

    def test_any_exception_prints_only_its_type_name(self):
        # Deep nesting raises RecursionError, which the old tuple did not catch.
        out = self.run_main("[" * 200000)
        self.assertIn("**not clean**: summary JSON unreadable (RecursionError)", out)
        out = self.run_main('{"cross_check": 1, "bounds": {}}')
        self.assertIn("unreadable (TypeError)", out)
        self.assertNotIn("cross_check", out.split("unreadable")[1])

    def test_strict_exits_one_for_not_clean_and_unreadable_only(self):
        import contextlib
        import io

        with tempfile.TemporaryDirectory() as d:
            clean, dirty, bad = (pathlib.Path(d, n) for n in ("a.json", "b.json", "c.json"))
            clean.write_text(json.dumps(summary()))
            dirty.write_text(json.dumps(summary(skipped=1)))
            bad.write_text("{not json")
            with contextlib.redirect_stdout(io.StringIO()):
                self.assertEqual(mod.main(["x", str(clean), "0", "--strict"]), 0)
                self.assertEqual(mod.main(["x", str(dirty), "0", "--strict"]), 1)
                self.assertEqual(mod.main(["x", str(bad), "0", "--strict"]), 1)
                self.assertEqual(mod.main(["x", str(pathlib.Path(d, "gone.json")), "0", "--strict"]), 1)
                # Without --strict the same inputs still exit 0.
                self.assertEqual(mod.main(["x", str(dirty), "0"]), 0)


if __name__ == "__main__":
    unittest.main()
