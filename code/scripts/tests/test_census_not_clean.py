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
        "go_actions": over.get("go_actions", 2),
        "go_built_ok": 2,
        "tmpfs_execs": over.get("tmpfs_execs", 6),
        "seal_mismatch": over.get("mismatch", 0),
    }
    if "steps" in over:
        s["exec_head"]["supervised_steps"] = over["steps"]
    if "notifications" in over:
        s["exec_head"]["notifications"] = over["notifications"]
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


class StrictExecHeadTests(unittest.TestCase):
    """--strict-exec-head: the exec-head criteria fail a fresh exec-head run; the soft passes stay annotations."""

    def run_flag(self, data, flag="--strict-exec-head"):
        import contextlib
        import io

        with tempfile.TemporaryDirectory() as d:
            p = pathlib.Path(d, "s.json")
            p.write_text(data if isinstance(data, str) else json.dumps(data))
            with contextlib.redirect_stdout(io.StringIO()):
                return mod.main(["x", str(p), "0", flag])

    def test_an_exec_head_violation_fails(self):
        self.assertEqual(self.run_flag(exec_head_summary(kills=1)), 1)

    def test_a_clean_exec_head_shard_passes(self):
        self.assertEqual(self.run_flag(exec_head_summary()), 0)

    def test_a_soft_pass_alone_stays_an_annotation(self):
        s = exec_head_summary()
        s["cross_check"]["skipped_too_large"] = 3
        self.assertEqual(self.run_flag(s), 0)
        self.assertEqual(self.run_flag(s, "--strict"), 1)

    def test_an_unreadable_summary_fails(self):
        self.assertEqual(self.run_flag("{not json"), 1)

    def test_a_shard_without_exec_heads_is_unaffected_by_the_plain_flags(self):
        self.assertEqual(self.run_flag(exec_head_summary(sealed=False, kills=5), "--strict"), 0)
        self.assertEqual(self.run_flag(exec_head_summary(sealed=False, kills=5), ""), 0)

    def test_requested_but_not_sealed_fails(self):
        # exec_head: true was dispatched, but the summary says the shard was not sealed
        # (or has no block at all): the mechanism under test was not on.
        for s in (exec_head_summary(sealed=False), summary()):
            self.assertEqual(self.run_flag(s), 1)
            self.assertEqual(mod.reasons(s, requested=True), ["exec-head requested but summary not sealed"])
            self.assertEqual(mod.reasons(s), [])
        s = exec_head_summary()
        del s["exec_head"]["sealed"]
        self.assertEqual(self.run_flag(s), 1)

    def test_a_non_bool_sealed_fails_closed(self):
        for v in ("true", 1, "yes", [True]):
            s = exec_head_summary()
            s["exec_head"]["sealed"] = v
            self.assertEqual(self.run_flag(s), 1, v)

    def test_zero_supervised_steps_or_notifications_means_it_did_not_run(self):
        for over, needle in [({"steps": 0}, "supervised_steps 0"), ({"notifications": 0}, "notifications 0")]:
            with self.subTest(over):
                s = exec_head_summary(**over)
                why = mod.reasons(s)
                self.assertEqual(len(why), 1, why)
                self.assertIn(needle, why[0])
                self.assertEqual(self.run_flag(s), 1)

    def test_a_direct_seal_mismatch_count_fails_without_a_collect_block(self):
        s = exec_head_summary(mismatch=2)
        s["collect"] = None  # the census ran without --records
        why = mod.reasons(s)
        self.assertEqual(len(why), 1, why)
        self.assertIn("2 started steps disagree with the exec-head seal", why[0])
        self.assertEqual(self.run_flag(s), 1)

    def test_every_counter_must_be_a_real_count(self):
        bad_values = [True, False, -1, 1.0, "0", None, [0], {"a": 0}]
        scalars = ["kills", "kill_steps", "supervised_steps", "notifications", "go_emittable", "seal_mismatch"]
        for key in scalars:
            for v in bad_values:
                with self.subTest(key=key, value=v):
                    s = exec_head_summary()
                    s["exec_head"][key] = v
                    self.assertEqual(self.run_flag(s), 1)
        for table in ("unclean", "sub_codes", "parse_reasons"):
            for v in bad_values:
                with self.subTest(table=table, value=v):
                    s = exec_head_summary()
                    first = next(iter(s["exec_head"][table]))
                    s["exec_head"][table][first] = v
                    self.assertEqual(self.run_flag(s), 1)
            with self.subTest(table=table, value="not a table"):
                s = exec_head_summary()
                s["exec_head"][table] = 3
                self.assertEqual(self.run_flag(s), 1)

    def test_a_missing_counter_fails_closed(self):
        for key in ["kills", "kill_steps", "supervised_steps", "notifications", "go_emittable", "seal_mismatch", "go_actions", "go_built_ok", "tmpfs_execs"]:
            with self.subTest(key):
                s = exec_head_summary()
                del s["exec_head"][key]
                self.assertEqual(self.run_flag(s), 1)

    def test_a_non_int_collect_seal_count_fails_closed(self):
        s = exec_head_summary()
        s["collect"]["unknown_reasons"]["exechead-seal"] = True
        self.assertEqual(self.run_flag(s), 1)


class VacuityTests(unittest.TestCase):
    """The vacuity rule: a shard that exercised nothing is a NOTE; a run of them is a failure."""

    def run_main(self, argv):
        import contextlib
        import io

        buf = io.StringIO()
        with contextlib.redirect_stdout(buf):
            rc = mod.main(argv)
        return rc, buf.getvalue()

    def shard(self, d, name, data):
        p = pathlib.Path(d, name)
        p.write_text(data if isinstance(data, str) else json.dumps(data))
        return str(p)

    def test_a_shard_with_no_tmpfs_exec_or_no_go_action_is_a_note_not_a_failure(self):
        for over, text in [({"tmpfs_execs": 0}, "tmpfs_execs 0"), ({"go_actions": 0}, "go_actions 0")]:
            with self.subTest(over), tempfile.TemporaryDirectory() as d:
                p = self.shard(d, "s.json", exec_head_summary(**over))
                rc, out = self.run_main(["x", p, "3", "--strict-exec-head"])
                self.assertEqual(rc, 0)
                self.assertIn(f"- shard 3: note: {text}", out)
                self.assertIn("exec-head criteria clean", out)
                self.assertEqual(mod.reasons(exec_head_summary(**over), True), [])

    def test_a_shard_that_exercised_the_mechanism_has_no_note(self):
        self.assertEqual(mod.exec_head_notes(exec_head_summary()), [])
        # And a shard that did not run with exec heads is not noted at all.
        self.assertEqual(mod.exec_head_notes(exec_head_summary(sealed=False, tmpfs_execs=0)), [])
        self.assertEqual(mod.exec_head_notes(summary()), [])

    def test_the_note_does_not_hide_a_real_failure(self):
        with tempfile.TemporaryDirectory() as d:
            p = self.shard(d, "s.json", exec_head_summary(tmpfs_execs=0, kills=1))
            rc, out = self.run_main(["x", p, "0", "--strict-exec-head"])
            self.assertEqual(rc, 1)
            self.assertIn("note: tmpfs_execs 0", out)
            self.assertIn("supervisor kills", out)

    def test_the_run_fails_when_every_sampled_shard_has_zero_tmpfs_execs(self):
        with tempfile.TemporaryDirectory() as d:
            paths = [self.shard(d, f"{i}.json", exec_head_summary(tmpfs_execs=0)) for i in range(3)]
            rc, out = self.run_main(["x", "--exec-head-run", *paths])
            self.assertEqual(rc, 1)
            self.assertIn("all 3 sampled shards on this image have tmpfs_execs 0", out)
            self.assertIn("go_emittable is not a result", out)

    def test_the_run_passes_when_one_shard_exercised_the_mechanism(self):
        with tempfile.TemporaryDirectory() as d:
            paths = [
                self.shard(d, "0.json", exec_head_summary(tmpfs_execs=0, go_actions=0)),
                self.shard(d, "1.json", exec_head_summary(tmpfs_execs=1)),
            ]
            rc, out = self.run_main(["x", "--exec-head-run", *paths])
            self.assertEqual(rc, 0)
            self.assertIn("exercised", out)

    def test_an_unreadable_shard_fails_the_run_and_is_named(self):
        with tempfile.TemporaryDirectory() as d:
            good, bad = self.shard(d, "g.json", exec_head_summary()), self.shard(d, "b.json", "{not json")
            rc, out = self.run_main(["x", "--exec-head-run", good, bad])
            self.assertEqual(rc, 1)
            self.assertIn("summary unreadable: b.json", out)
            self.assertEqual(self.run_main(["x", "--exec-head-run", bad])[0], 1)
            self.assertEqual(self.run_main(["x", "--exec-head-run"])[0], 1)

    def test_expected_shards_fails_when_fewer_summaries_were_read(self):
        with tempfile.TemporaryDirectory() as d:
            paths = [self.shard(d, f"{i}.json", exec_head_summary()) for i in range(2)]
            self.assertEqual(self.run_main(["x", "--exec-head-run", "--expected-shards", "2", *paths])[0], 0)
            rc, out = self.run_main(["x", "--exec-head-run", "--expected-shards", "3", *paths])
            self.assertEqual(rc, 1)
            self.assertIn("only 2 readable summaries, expected 3", out)
            # No summary at all with a count expected (the shards left no JSON).
            self.assertEqual(self.run_main(["x", "--exec-head-run", "--expected-shards", "1"])[0], 1)
            for bad in (["--expected-shards"], ["--expected-shards", "x"], ["--expected-shards", "-1"]):
                self.assertEqual(self.run_main(["x", "--exec-head-run", *bad, *paths])[0], 1)

    def test_unsealed_shards_do_not_count_and_a_run_with_none_sealed_fails(self):
        with tempfile.TemporaryDirectory() as d:
            p = self.shard(d, "u.json", exec_head_summary(sealed=False))
            self.assertEqual(self.run_main(["x", "--exec-head-run", p])[0], 1)
            # An unsealed shard's tmpfs_execs is not evidence for the run.
            q = self.shard(d, "z.json", exec_head_summary(tmpfs_execs=0))
            r = self.shard(d, "w.json", exec_head_summary(sealed=False, tmpfs_execs=9))
            self.assertEqual(self.run_main(["x", "--exec-head-run", q, r])[0], 1)

    def test_a_malformed_tmpfs_count_fails_closed_at_run_level(self):
        for v in (True, -1, "3", 1.5, None):
            with self.subTest(v), tempfile.TemporaryDirectory() as d:
                s = exec_head_summary()
                s["exec_head"]["tmpfs_execs"] = v
                self.assertEqual(self.run_main(["x", "--exec-head-run", self.shard(d, "s.json", s)])[0], 1)


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
