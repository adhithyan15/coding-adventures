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

    def test_g2_is_reported_not_a_gate(self):
        # KILN04 D49: a census that misses G2 everywhere is not "not clean".
        s = summary()
        s["timing"] = {
            "g2_failing": 99,
            "share_over_10pct": 99,
            "parse_over_5s": 99,
            "g2_at_p99": {"traces": 3, "failing": 3},
            "g2_target": {"met": False, "share_p50": 0.107, "image_id": "ubuntu-24.04"},
        }
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
