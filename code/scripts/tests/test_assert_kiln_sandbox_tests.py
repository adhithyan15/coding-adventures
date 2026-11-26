"""Tests for code/programs/go/kiln/ci/assert_sandbox_tests.py and the CI job wiring."""

from __future__ import annotations

import importlib.util
import json
import re
import sys
import tempfile
import unittest
from pathlib import Path

REPO = Path(__file__).resolve().parents[3]
SCRIPT = REPO / "code" / "programs" / "go" / "kiln" / "ci" / "assert_sandbox_tests.py"
CI = (REPO / ".github" / "workflows" / "ci.yml").read_text(encoding="utf-8")

spec = importlib.util.spec_from_file_location("assert_kiln_sandbox_tests", SCRIPT)
mod = importlib.util.module_from_spec(spec)
sys.modules["assert_kiln_sandbox_tests"] = mod
spec.loader.exec_module(mod)


def stream(outcomes: dict[str, str]) -> list[str]:
    lines = []
    for name, action in outcomes.items():
        lines.append(json.dumps({"Action": "run", "Test": name}))
        lines.append(json.dumps({"Action": action, "Test": name}))
    lines.append(json.dumps({"Action": "pass", "Package": "kiln"}))
    return lines


REAL_PROBE = (
    "    sandbox_tracesize_linux_amd64_test.go:80: PROBE: trace record-step bytes=51650 lines=327 old_mb_s=144.7 "
    "grammar_mb_s=22.8 best_mb_s=25.1 gated=1 old_allocs=1 grammar_allocs=2 step_wall_ms=900 parse_ms=2.26 "
    'parse_share=0.0025 old_refusal="" grammar_refusal=""'
)
SYNTHETIC_PROBE = (
    "    sandbox_tracesize_linux_amd64_test.go:130: PROBE: trace synthetic-slice bytes=65800 lines=9000 "
    "best_mb_s=30.0 ref_mb_s=100.0 norm_mb_s=30.0 calibrated=1 gated=1 repeats=7"
)


def probe_events(test: str, text: str, action: str = "pass") -> list[str]:
    return [
        json.dumps({"Action": "output", "Test": test, "Output": text + "\n"}),
        json.dumps({"Action": action, "Test": test}),
    ]


CROSSCHECK = "    sandbox_x_test.go:9: PROBE: crosscheck traces=7 accepted=7 bytes=51650 mismatches=0 allowed=0"


def with_gate(lines: list[str]) -> list[str]:
    return lines + probe_events("TestSandboxTraceGateSynthetic", SYNTHETIC_PROBE) + probe_events("TestSandboxRecordStep", CROSSCHECK)


def all_pass() -> dict[str, str]:
    return {name: "pass" for name in mod.REQUIRED}


class AssertScript(unittest.TestCase):
    def test_all_passing_is_clean(self) -> None:
        self.assertEqual(mod.check(stream(all_pass())), [])

    def test_a_skip_fails(self) -> None:
        outcomes = all_pass()
        outcomes["TestSandboxFilesystem"] = "skip"
        problems = mod.check(stream(outcomes))
        self.assertTrue(any("TestSandboxFilesystem: skip" in p for p in problems), problems)

    def test_a_skipped_subtest_fails(self) -> None:
        outcomes = all_pass()
        outcomes["TestSandboxFilesystem/inner"] = "skip"
        self.assertTrue(mod.check(stream(outcomes)))

    def test_a_failure_fails(self) -> None:
        outcomes = all_pass()
        outcomes["TestSandboxNoHostIPC"] = "fail"
        self.assertTrue(mod.check(stream(outcomes)))

    def test_a_missing_test_fails(self) -> None:
        outcomes = all_pass()
        del outcomes["TestSandboxRealStrace"]
        problems = mod.check(stream(outcomes))
        self.assertTrue(any("did not run" in p for p in problems), problems)

    def test_an_unnamed_sandbox_test_fails(self) -> None:
        outcomes = all_pass()
        outcomes["TestSandboxBrandNew"] = "pass"
        problems = mod.check(stream(outcomes))
        self.assertTrue(any("does not name" in p for p in problems), problems)

    def test_garbage_is_reported(self) -> None:
        self.assertTrue(mod.check(["not json"] + stream(all_pass())))

    def test_main_exit_codes(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            good = Path(tmp) / "good.json"
            good.write_text("\n".join(with_gate(stream(all_pass()))), encoding="utf-8")
            bad = Path(tmp) / "bad.json"
            bad.write_text("\n".join(stream({})), encoding="utf-8")
            self.assertEqual(mod.main(["x", str(good)]), 0)
            self.assertEqual(mod.main(["x", str(bad)]), 1)
            self.assertEqual(mod.main(["x", str(Path(tmp) / "missing")]), 2)
            self.assertEqual(mod.main(["x"]), 2)

    def test_every_required_name_exists_in_the_go_tests(self) -> None:
        # The sandbox tests are spread over sandbox*_linux_amd64_test.go in the
        # kiln package directory (the CI job runs them all with -run Sandbox).
        kiln = REPO / "code" / "programs" / "go" / "kiln"
        files = sorted(kiln.glob("sandbox*_linux_amd64_test.go"))
        self.assertTrue(files)
        source = "\n".join(f.read_text(encoding="utf-8") for f in files)
        declared = set(re.findall(r"^func (TestSandbox\w+)\(", source, re.M))
        self.assertEqual(declared, set(mod.REQUIRED))


class GateG1(unittest.TestCase):
    """The throughput gate must have been applied, not skipped by omission."""

    def test_synthetic_gated_line_satisfies_the_gate(self) -> None:
        self.assertEqual(mod.gate_problems(with_gate(stream(all_pass()))), [])

    def test_a_missing_gate_fails(self) -> None:
        problems = mod.gate_problems(stream(all_pass()))
        self.assertTrue(any("no passing test logged a gated PROBE" in p for p in problems), problems)

    def test_an_ungated_probe_does_not_count(self) -> None:
        ungated = REAL_PROBE.replace("gated=1", "gated=0")
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxRecordStep", ungated))
        self.assertTrue(problems, problems)

    def test_a_real_gated_line_without_the_synthetic_one_fails(self) -> None:
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxRecordStep", REAL_PROBE))
        self.assertTrue(any("synthetic-slice" in p for p in problems), problems)

    def test_a_gated_line_from_a_failing_test_does_not_count(self) -> None:
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", SYNTHETIC_PROBE, "fail"))
        self.assertTrue(problems, problems)

    def test_a_forged_substring_in_another_line_does_not_count(self) -> None:
        forged = "    x_test.go:9: opened /tmp/PROBE: trace synthetic-slice best_mb_s=30.0 gated=1"
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", forged))
        self.assertTrue(any("no passing test logged a gated" in p for p in problems), problems)
        bare = "PROBE: trace synthetic-slice best_mb_s=30.0 gated=1"  # no file:line prefix
        self.assertTrue(mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", bare)))

    def test_the_synthetic_line_from_the_wrong_test_does_not_count(self) -> None:
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxRecordStep", SYNTHETIC_PROBE))
        self.assertTrue(any("synthetic-slice" in p and "missing" in p for p in problems), problems)

    def test_a_gated_rate_below_the_floor_fails(self) -> None:
        slow = SYNTHETIC_PROBE.replace("best_mb_s=30.0", "best_mb_s=7.9")
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", slow))
        self.assertTrue(any("below the 8" in p for p in problems), problems)
        ok = SYNTHETIC_PROBE.replace("best_mb_s=30.0", "best_mb_s=8.0")
        self.assertEqual(mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", ok)), [])

    def synthetic(self, line: str) -> list[str]:
        return mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", line))

    def test_a_raw_rate_between_8_and_14_now_passes(self) -> None:
        for rate in ("8.0", "10.8", "12.3", "13.9"):
            line = SYNTHETIC_PROBE.replace("best_mb_s=30.0", f"best_mb_s={rate}")
            self.assertEqual(self.synthetic(line), [], rate)
        real = REAL_PROBE.replace("best_mb_s=25.1", "best_mb_s=11.0")
        self.assertEqual(mod.gate_problems(with_gate(stream(all_pass())) + probe_events("TestSandboxRecordStep", real)), [])

    def test_a_missing_reference_rate_fails(self) -> None:
        line = SYNTHETIC_PROBE.replace("ref_mb_s=100.0 ", "")
        self.assertTrue(any("ref_mb_s" in p for p in self.synthetic(line)))

    def test_a_non_finite_reference_rate_fails(self) -> None:
        for bad in ("inf", "NaN", "0", "0.0", "-5.0", "abc"):
            line = SYNTHETIC_PROBE.replace("ref_mb_s=100.0", f"ref_mb_s={bad}")
            self.assertTrue(any("ref_mb_s" in p for p in self.synthetic(line)), bad)

    def test_calibrated_must_be_exactly_0_or_1(self) -> None:
        for bad in ("2", "true", "01", "-1", "", "yes"):
            line = SYNTHETIC_PROBE.replace("calibrated=1", f"calibrated={bad}")
            self.assertTrue(any("calibrated=" in p and "exactly 0 or 1" in p for p in self.synthetic(line)), bad)
        missing = SYNTHETIC_PROBE.replace("calibrated=1 ", "")
        self.assertTrue(any("exactly 0 or 1" in p for p in self.synthetic(missing)))
        self.assertEqual(self.synthetic(SYNTHETIC_PROBE), [])

    def test_calibrated_0_fails_because_calibration_is_required(self) -> None:
        # The calibrating commit made calibrated=1 mandatory: Go cannot switch the
        # normalised gate off by logging calibrated=0.
        self.assertIs(mod.REQUIRE_CALIBRATED, True)
        line = SYNTHETIC_PROBE.replace("calibrated=1", "calibrated=0")
        self.assertTrue(any("REQUIRE_CALIBRATED" in p for p in self.synthetic(line)))
        # With a low norm and calibrated=0 the line still fails (never accepted).
        low = line.replace("norm_mb_s=30.0", "norm_mb_s=1.0")
        self.assertTrue(self.synthetic(low))

    def test_the_switch_can_be_relaxed_only_in_the_module(self) -> None:
        mod.REQUIRE_CALIBRATED = False
        try:
            line = SYNTHETIC_PROBE.replace("calibrated=1", "calibrated=0")
            self.assertEqual(self.synthetic(line), [])
        finally:
            mod.REQUIRE_CALIBRATED = True

    def test_a_missing_norm_when_calibrated_fails(self) -> None:
        line = SYNTHETIC_PROBE.replace("norm_mb_s=30.0 ", "")
        self.assertTrue(any("norm_mb_s" in p for p in self.synthetic(line)))

    def test_a_normalised_rate_below_the_floor_fails_when_calibrated(self) -> None:
        low = SYNTHETIC_PROBE.replace("norm_mb_s=30.0", "norm_mb_s=13.9")
        self.assertTrue(any("norm_mb_s" in p and "below the 14" in p for p in self.synthetic(low)))
        ok = SYNTHETIC_PROBE.replace("norm_mb_s=30.0", "norm_mb_s=14.0")
        self.assertEqual(self.synthetic(ok), [])
        for bad in ("inf", "NaN", "0", "abc"):
            line = SYNTHETIC_PROBE.replace("norm_mb_s=30.0", f"norm_mb_s={bad}")
            self.assertTrue(any("norm_mb_s" in p for p in self.synthetic(line)), bad)

    def test_a_refusal_cannot_claim_calibration_or_a_reference_rate(self) -> None:
        # The facts before the refusals win; text after them is never read.
        line = SYNTHETIC_PROBE.replace("ref_mb_s=100.0 ", "") + ' grammar_refusal="ref_mb_s=100.0 calibrated=0"'
        self.assertTrue(any("ref_mb_s" in p for p in self.synthetic(line)))

    def test_non_finite_zero_or_missing_rates_fail(self) -> None:
        for bad in ("inf", "+Inf", "NaN", "0", "0.0", "-3.0", "abc"):
            line = SYNTHETIC_PROBE.replace("best_mb_s=30.0", f"best_mb_s={bad}")
            problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", line))
            self.assertTrue(problems, bad)
        missing = SYNTHETIC_PROBE.replace("best_mb_s=30.0 ", "")
        self.assertTrue(mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", missing)))

    def test_a_slow_real_gated_trace_fails_even_with_the_synthetic_line(self) -> None:
        slow = REAL_PROBE.replace("best_mb_s=25.1", "best_mb_s=7.0")
        problems = mod.gate_problems(with_gate(stream(all_pass())) + probe_events("TestSandboxRecordStep", slow))
        self.assertTrue(any("record-step" in p and "below" in p for p in problems), problems)

    def test_a_job_without_the_gate_env_var_fails(self) -> None:
        # Without KILN_G1_GATE=1 the Go tests log the synthetic slice with gated=0,
        # so the kiln-sandbox job's stream has no gated line and must fail.
        ungated = SYNTHETIC_PROBE.replace("gated=1", "gated=0")
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", ungated))
        self.assertTrue(any("no passing test logged a gated" in p for p in problems), problems)

    def test_the_4mib_census_line_is_not_gated(self) -> None:
        census = "    x_test.go:5: PROBE: trace synthetic-4mib-census bytes=4195800 lines=1 best_mb_s=5.0 gated=0 repeats=7"
        self.assertEqual(mod.gate_problems(with_gate(stream(all_pass())) + probe_events("TestSandboxTraceGateSynthetic", census)), [])

    def test_a_refusal_quoting_fields_cannot_override_the_real_ones(self) -> None:
        # The slow real rate is followed by refusals that quote a fast, gated line.
        evil = (
            "    x_test.go:9: PROBE: trace evil bytes=1 lines=1 best_mb_s=7.0 gated=1 "
            'old_refusal="best_mb_s=99 gated=1" grammar_refusal="best_mb_s=99 gated=1 extra=1"'
        )
        (probe,) = mod.gated_probes(probe_events("TestSandboxRecordStep", evil))
        self.assertEqual(probe[2]["best_mb_s"], "7.0")
        self.assertNotIn("extra", probe[2])
        self.assertTrue(mod.gate_problems(with_gate(stream(all_pass())) + probe_events("TestSandboxRecordStep", evil)))
        # And a line whose REAL fields are ungated is not made gated by a refusal.
        sneaky = (
            "    x_test.go:9: PROBE: trace sneaky bytes=1 lines=1 best_mb_s=30.0 gated=0 "
            'old_refusal="" grammar_refusal="gated=1"'
        )
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordStep", sneaky)), [])

    def test_probe_fields_stop_at_the_first_refusal_and_keep_the_first_key(self) -> None:
        self.assertEqual(
            mod.probe_fields('PROBE: trace a best_mb_s=30.0 gated=1 best_mb_s=1.0 old_refusal="gated=0" grammar_refusal=""'),
            {"best_mb_s": "30.0", "gated": "1"},
        )
        self.assertEqual(mod.probe_fields("PROBE: trace a k=1 k=2 k=3"), {"k": "1"})

    def test_a_duplicate_key_keeps_the_first_value(self) -> None:
        dup = SYNTHETIC_PROBE.replace("repeats=7", "repeats=7 best_mb_s=1.0")
        self.assertEqual(mod.gate_problems(with_gate(stream(all_pass())) + probe_events("TestSandboxTraceGateSynthetic", dup)), [])
        dup_first_bad = SYNTHETIC_PROBE.replace("best_mb_s=30.0", "best_mb_s=1.0").replace("repeats=7", "repeats=7 best_mb_s=30.0")
        self.assertTrue(mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", dup_first_bad)))

    def test_a_continuation_line_is_not_a_probe_line(self) -> None:
        # t.Logf("first\nPROBE: ...") prints the second line indented eight spaces.
        cont = "        x_test.go:9: PROBE: trace evil bytes=1 lines=1 best_mb_s=30.0 gated=1"
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordStep", cont)), [])
        tab = "\t    x_test.go:9: PROBE: trace evil bytes=1 lines=1 best_mb_s=30.0 gated=1"
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordStep", tab)), [])
        three = "   x_test.go:9: PROBE: trace evil bytes=1 lines=1 best_mb_s=30.0 gated=1"
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordStep", three)), [])

    def test_a_subtest_probe_line_has_four_spaces_in_verbose_mode(self) -> None:
        # go test -v indents EVERY t.Logf line four spaces, at any subtest depth;
        # eight spaces is a continuation line.
        good = "    x_test.go:9: PROBE: trace probe-pipes bytes=60000 lines=1 best_mb_s=30.0 gated=1"
        for test in ("TestSandboxRecordProbe/ok", "TestSandboxRecordProbe/ok/deep"):
            self.assertEqual(len(mod.gated_probes(probe_events(test, good))), 1, test)
            self.assertEqual(mod.gated_probes(probe_events(test, "    " + good)), [], test)

    def test_a_real_verbose_json_subtest_event_parses(self) -> None:
        # Only the FIRST event is verbatim `go test -v -json` output (go1.26), from
        # a scratch test whose subtest "sub one" logged a PROBE line.  The second
        # is hand-written to look like the continuation line of a multi-line
        # message (eight spaces, no file:line), which go test does print.
        real = {"Time": "2026-11-17T19:30:15.369747-08:00", "Action": "output", "Package": "scratchfix", "Test": "TestTop/sub_one",
                "Output": "    a_test.go:7: PROBE: trace probe-pipes bytes=60000 lines=1 best_mb_s=30.0 gated=1\n"}
        forged = {"Time": "2026-11-17T19:30:15.369759-08:00", "Action": "output", "Package": "scratchfix", "Test": "TestTop/sub_one",
                  "Output": "        PROBE: trace evil bytes=1 best_mb_s=99 gated=1\n"}
        lines = [json.dumps(real), json.dumps(forged), json.dumps({"Action": "pass", "Test": "TestTop/sub_one"})]
        found = mod.gated_probes(lines)
        self.assertEqual([(t, label) for t, label, _ in found], [("TestTop/sub_one", "probe-pipes")])

    def test_a_probe_line_split_across_events_still_parses(self) -> None:
        text = SYNTHETIC_PROBE + "\n"
        cut = len(text) // 2
        events = [
            json.dumps({"Action": "output", "Test": "TestSandboxTraceGateSynthetic", "Output": text[:cut]}),
            json.dumps({"Action": "output", "Test": "TestSandboxTraceGateSynthetic", "Output": text[cut:]}),
            json.dumps({"Action": "pass", "Test": "TestSandboxTraceGateSynthetic"}),
        ]
        (probe,) = mod.gated_probes(events)
        self.assertEqual((probe[1], probe[2]["best_mb_s"]), ("synthetic-slice", "30.0"))

    def test_a_forged_line_after_a_1024_byte_split_does_not_match(self) -> None:
        # test2json cuts a long line at 1024 bytes with no newline; what follows the
        # cut is the middle of the SAME line, even when it looks like a PROBE line.
        head = '    x_test.go:5: PROBE: trace long bytes=1 lines=1 best_mb_s=9.0 gated=0 old_refusal="' + "a" * 900
        evil = '    evil.go:1: PROBE: trace evil bytes=1 lines=1 best_mb_s=99 gated=1 repeats=7'
        events = [
            json.dumps({"Action": "output", "Test": "TestSandboxRecordStep", "Output": head}),
            json.dumps({"Action": "output", "Test": "TestSandboxRecordStep", "Output": evil + '" grammar_refusal=""\n'}),
            json.dumps({"Action": "pass", "Test": "TestSandboxRecordStep"}),
        ]
        self.assertEqual(mod.gated_probes(events), [])
        # An unterminated tail at the end is not a line either.
        tail = [
            json.dumps({"Action": "output", "Test": "TestSandboxRecordStep", "Output": evil}),
            json.dumps({"Action": "pass", "Test": "TestSandboxRecordStep"}),
        ]
        self.assertEqual(mod.gated_probes(tail), [])
        # The facts printed for the log follow the same rule.
        self.assertEqual(mod.passing_facts(events), {"TestSandboxRecordStep": [mod.LOG_PREFIX + (head + evil + '" grammar_refusal=""').strip()[: mod.MAX_FACT_CHARS]]})

    def test_facts_print_no_control_characters_and_no_workflow_command(self) -> None:
        def out(text: str) -> str:
            return json.dumps({"Action": "output", "Test": "TestA", "Output": text})

        lines = [
            out("    x_test.go:1: F2: a\r::error::x\n"),
            out("    x_test.go:2: F2: b\x85::warning::y\n"),
            out("    x_test.go:3: F2: c\x1b[31m\u2028::error::z\n"),
            json.dumps({"Action": "pass", "Test": "TestA"}),
        ]
        facts = mod.passing_facts(lines)["TestA"]
        self.assertEqual(len(facts), 3)
        for fact in facts:
            self.assertEqual(fact, mod.printable(fact))
            self.assertFalse([c for c in fact if not 0x20 <= ord(c) <= 0x7E], fact)
            self.assertFalse(any(piece.startswith("::") for piece in fact.splitlines()), fact)
        self.assertIn("\\x0d::error::x", facts[0])
        # The failing test's output is escaped the same way.
        bad = [out("x\r::error::boom\n"), json.dumps({"Action": "fail", "Test": "TestA"})]
        printed = [line for text in mod.failing_output(bad).values() for line in text]
        self.assertTrue(printed)
        self.assertFalse([line for line in printed if line.startswith("::") or "\r" in line], printed)

    def test_an_over_long_line_is_dropped_and_the_next_line_still_parses(self) -> None:
        big = "    x_test.go:1: PROBE: trace huge bytes=1 lines=1 best_mb_s=99 gated=1 " + "a" * (mod.MAX_PENDING_CHARS + 10)
        test = "TestSandboxTraceGateSynthetic"
        events = [
            json.dumps({"Action": "output", "Test": test, "Output": big[:5000]}),
            json.dumps({"Action": "output", "Test": test, "Output": big[5000:] + "\n" + SYNTHETIC_PROBE + "\n"}),
            json.dumps({"Action": "pass", "Test": test}),
        ]
        found = mod.gated_probes(events)
        self.assertEqual([label for _, label, _ in found], ["synthetic-slice"])

    def test_a_label_ending_in_a_colon_is_not_a_measurement(self) -> None:
        err = "    x_test.go:5: PROBE: trace probe-pipes: cannot open: best_mb_s=99 gated=1 open x: no such file"
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordProbe", err)), [])

    def test_well_formed_lines_still_parse(self) -> None:
        (real,) = mod.gated_probes(probe_events("TestSandboxRecordStep", REAL_PROBE))
        self.assertEqual((real[1], real[2]["best_mb_s"], real[2]["step_wall_ms"], real[2]["parse_share"]), ("record-step", "25.1", "900", "0.0025"))
        (syn,) = mod.gated_probes(probe_events("TestSandboxTraceGateSynthetic", SYNTHETIC_PROBE))
        self.assertEqual(syn[1], "synthetic-slice")

    def test_main_fails_when_the_gate_is_missing(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "nogate.json"
            path.write_text("\n".join(stream(all_pass())), encoding="utf-8")
            self.assertEqual(mod.main(["x", str(path)]), 1)


def cc(traces: str = "7", accepted: str = "7", tail: str = "mismatches=0 allowed=0") -> str:
    return f"    x_test.go:9: PROBE: crosscheck traces={traces} accepted={accepted} bytes=10 {tail}"


class CrossCheckOfChildTraces(unittest.TestCase):
    """D34: the in-test cross-check of the traces the record children wrote."""

    def test_a_passing_line_with_accepted_traces_is_clean(self) -> None:
        self.assertEqual(mod.crosscheck_problems(probe_events("TestA", cc())), [])

    def test_no_line_is_a_problem(self) -> None:
        problems = mod.crosscheck_problems(probe_events("TestA", "    x_test.go:1: other"))
        self.assertEqual(len(problems), 1)
        self.assertIn("no passing test", problems[0])

    def test_zero_traces_everywhere_is_vacuous(self) -> None:
        problems = mod.crosscheck_problems(probe_events("TestA", cc("0", "0")))
        self.assertTrue(any("traces=0" in p for p in problems), problems)

    def test_traces_all_refused_is_vacuous(self) -> None:
        # Both readers refusing every trace alike is "agreement"; accepted=0 is
        # what shows the check's parameters drifted.
        problems = mod.crosscheck_problems(probe_events("TestA", cc("7", "0")))
        self.assertTrue(any("ACCEPTED" in p for p in problems), problems)

    def test_one_test_with_traces_is_enough(self) -> None:
        lines = probe_events("TestA", cc("0", "0")) + probe_events("TestB", cc())
        self.assertEqual(mod.crosscheck_problems(lines), [])

    def test_accepted_is_summed_across_tests(self) -> None:
        lines = probe_events("TestA", cc("3", "0")) + probe_events("TestB", cc("2", "1"))
        self.assertEqual(mod.crosscheck_problems(lines), [])

    def test_accepted_cannot_exceed_traces(self) -> None:
        problems = mod.crosscheck_problems(probe_events("TestA", cc("1", "5")))
        self.assertTrue(any("exceeds" in p for p in problems), problems)

    def test_a_failing_test_does_not_count(self) -> None:
        problems = mod.crosscheck_problems(probe_events("TestA", cc(), action="fail"))
        self.assertIn("no passing test", problems[0])

    def test_mismatch_or_allowed_fails(self) -> None:
        for tail in ("mismatches=1 allowed=0", "mismatches=0 allowed=1"):
            problems = mod.crosscheck_problems(probe_events("TestA", cc(tail=tail)))
            self.assertTrue(any("must be 0" in p for p in problems), tail)

    def test_missing_or_garbled_field_fails(self) -> None:
        problems = mod.crosscheck_problems(probe_events("TestA", cc(tail="mismatches=0")))
        self.assertTrue(any("allowed" in p for p in problems), problems)
        problems = mod.crosscheck_problems(probe_events("TestA", cc(traces="-3")))
        self.assertTrue(any("traces" in p for p in problems), problems)
        line = "    x_test.go:9: PROBE: crosscheck traces=3 bytes=10 mismatches=0 allowed=0"
        problems = mod.crosscheck_problems(probe_events("TestA", line))
        self.assertTrue(any("accepted" in p for p in problems), problems)

    def test_first_key_wins(self) -> None:
        # A later accepted=99 cannot raise the real (first) count.
        problems = mod.crosscheck_problems(probe_events("TestA", cc("7", "0", "mismatches=0 allowed=0 accepted=99")))
        self.assertTrue(any("ACCEPTED" in p for p in problems), problems)

    def test_the_anchor_rejects_forgeries(self) -> None:
        forged = [
            "  x_test.go:9: PROBE: crosscheck traces=5 accepted=5 bytes=1 mismatches=0 allowed=0",
            "    x_test.go:9: note PROBE: crosscheck traces=5 accepted=5 bytes=1 mismatches=0 allowed=0",
            "        PROBE: crosscheck traces=5 accepted=5 bytes=1 mismatches=0 allowed=0",
        ]
        lines = [json.dumps({"Action": "output", "Test": "TestA", "Output": f + "\n"}) for f in forged]
        lines.append(json.dumps({"Action": "pass", "Test": "TestA"}))
        self.assertIn("no passing test", mod.crosscheck_problems(lines)[0])

    def test_a_long_line_split_by_test2json_cannot_forge_one(self) -> None:
        first = "    x_test.go:9: " + "a" * 100
        second = "    y_test.go:1: PROBE: crosscheck traces=5 accepted=5 bytes=1 mismatches=0 allowed=0\n"
        lines = [json.dumps({"Action": "output", "Test": "TestA", "Output": o}) for o in (first, second)]
        lines.append(json.dumps({"Action": "pass", "Test": "TestA"}))
        self.assertIn("no passing test", mod.crosscheck_problems(lines)[0])

    def test_main_reports_the_check(self) -> None:
        with tempfile.TemporaryDirectory() as tmp:
            path = Path(tmp) / "out.json"
            path.write_text("\n".join(with_gate(stream(all_pass()))), encoding="utf-8")
            self.assertEqual(mod.main(["x", str(path)]), 0)
            without = stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", SYNTHETIC_PROBE)
            path.write_text("\n".join(without), encoding="utf-8")
            self.assertEqual(mod.main(["x", str(path)]), 1)


class CiWiring(unittest.TestCase):
    def job(self) -> str:
        return CI.split("\n  kiln-sandbox:\n", 1)[1].split("\n  d18f-message-conformance:\n", 1)[0]

    def test_job_requires_the_sandbox_and_asserts_the_stream(self) -> None:
        body = self.job()
        # D27: both images, never ubuntu-latest, one job id so ci-gate's
        # needs['kiln-sandbox'].result aggregates the legs.
        self.assertIn("runs-on: ${{ matrix.os }}", body)
        self.assertIn("os: [ubuntu-24.04, ubuntu-26.04]", body)
        self.assertIn("fail-fast: false", body)
        self.assertNotIn("ubuntu-latest", body)
        self.assertIn("sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0", body)
        self.assertNotRegex(body, r"if:.*apparmor")
        # strace is preinstalled on both images: apt only as a fallback, never
        # an unconditional `apt-get update` (mirror stalls fail the job).
        self.assertIn("if command -v strace >/dev/null 2>&1; then", body)
        self.assertIn("bash code/scripts/ci/apt-install.sh strace", body)
        self.assertIn("strace -V | head -1", body)
        self.assertIn("KILN_REQUIRE_SANDBOX: '1'", body)
        # D24: real traces are cross-checked between the old scanner and the grammar path.
        self.assertIn("KILN_TRACE_DIFF: '1'", body)
        self.assertIn("KILN_G1_GATE: '1'", body)
        self.assertIn("go test -count=1 -run Sandbox -v -json .", body)
        self.assertIn("assert_sandbox_tests.py", body)
        # The assertion is its own step and runs even when the tests failed, so
        # the failing output is printed rather than discarded.
        step = body.split("      - name: Assert every sandbox test ran and passed\n", 1)[1]
        self.assertIn("        if: always()\n", step.split("\n      - name:", 1)[0])
        self.assertNotIn("/dev/null", body.split("Assert every sandbox test", 1)[0].split("go test", 1)[1])
        self.assertIn("set -euo pipefail", body)
        self.assertNotIn("continue-on-error", body)
        self.assertIn("persist-credentials: false", body)

    def test_this_test_file_is_run_by_the_kiln_sandbox_job(self) -> None:
        # D34: nothing discovered this file before; the job that uses the script
        # now runs its tests (they need no sandbox, only python3).
        self.assertIn("python3 -m unittest discover -s code/scripts/tests -p 'test_assert_kiln_sandbox_tests.py'", self.job())

    def test_scheduled_and_dispatched_full_builds_force_every_gate(self) -> None:
        # The job's `if:` is the registry flag; detect makes every flag true on a
        # forced full build (schedule, workflow_dispatch, ci-full-build label).
        self.assertIn("if: needs.detect.outputs.run_kiln_sandbox == 'true'", self.job())
        self.assertIn('if [ "$EVENT_NAME" = "schedule" ] || [ "$MANUAL_FULL_BUILD" = "true" ]; then', CI)
        self.assertIn("force_flag=(-force)", CI)
        self.assertIn("schedule:", CI.split("jobs:", 1)[0])
        self.assertIn("workflow_dispatch:", CI.split("jobs:", 1)[0])

    def test_failing_output_is_reported(self) -> None:
        lines = [
            json.dumps({"Action": "run", "Test": "TestSandboxNoHostIPC"}),
            json.dumps({"Action": "output", "Test": "TestSandboxNoHostIPC", "Output": "boom: exit 125\n"}),
            json.dumps({"Action": "fail", "Test": "TestSandboxNoHostIPC"}),
            json.dumps({"Action": "output", "Test": "TestSandboxFilesystem", "Output": "fine\n"}),
            json.dumps({"Action": "pass", "Test": "TestSandboxFilesystem"}),
        ]
        self.assertEqual(mod.failing_output(lines), {"TestSandboxNoHostIPC": ["  boom: exit 125"]})

    def test_repeat_step_prints_every_failed_run_in_full(self) -> None:
        body = self.job().split("      - name: Repeat the restart and signal real-strace tests\n", 1)[1].split("\n      - name:", 1)[0]
        self.assertIn("go test -json -timeout 12m -count=20", body)
        self.assertIn("go test -json -timeout 10m -count=5", body)
        self.assertIn('--failures "$json1"', body)
        self.assertIn('--failures "$json2"', body)
        self.assertIn("STRACE_LISTEN_RETRIES", body)
        self.assertIn("::notice::", body)
        self.assertNotIn("tail -n 200", body)

    def test_failed_runs_keeps_each_failed_iteration_of_a_repeated_test(self) -> None:
        def ev(action: str, test: str | None = None, text: str | None = None) -> str:
            event = {"Action": action}
            if test:
                event["Test"] = test
            if text is not None:
                event["Output"] = text
            return json.dumps(event)

        lines = []
        for i, final in enumerate(["pass", "fail", "pass", "fail", "pass"]):
            lines += [ev("run", "TestD"), ev("output", "TestD", f"msg {i}\n"), ev(final, "TestD")]
        lines += [ev("output", None, "FAIL\n"), ev("fail")]
        self.assertEqual(
            mod.failed_runs(lines),
            [("TestD", ["  msg 1"]), ("TestD", ["  msg 3"]), ("(package)", ["  FAIL"])],
        )
        # All green: nothing to print.
        self.assertEqual(mod.failed_runs([ev("run", "T"), ev("pass", "T"), ev("pass")]), [])

    def test_failed_runs_is_not_truncated_and_cannot_inject(self) -> None:
        text = "".join(f"line {i}\n" for i in range(500)) + "::error::x\n"
        lines = [
            json.dumps({"Action": "run", "Test": "T"}),
            json.dumps({"Action": "output", "Test": "T", "Output": text}),
            json.dumps({"Action": "fail", "Test": "T"}),
        ]
        [(name, out), (package, _)] = mod.failed_runs(lines)
        self.assertEqual(package, "(package)")
        self.assertEqual(name, "T")
        self.assertEqual(len(out), 501)
        self.assertFalse([line for line in out if line.startswith("::")])

    def test_plain_text_rebuilds_the_verbose_log(self) -> None:
        lines = [
            json.dumps({"Action": "output", "Test": "T", "Output": "    f.go:1: STRACE_LISTEN_RETRIES=2\n"}),
            json.dumps({"Action": "pass", "Test": "T"}),
            "# build failure text",
            json.dumps({"Action": "output", "Output": "FAIL\tkiln\n"}),
        ]
        self.assertEqual(mod.plain_text(lines), "    f.go:1: STRACE_LISTEN_RETRIES=2\n# build failure text\nFAIL\tkiln\n")

    def test_passing_facts_are_surfaced_and_bounded(self) -> None:
        def out(test: str, text: str) -> str:
            return json.dumps({"Action": "output", "Test": test, "Output": text + "\n"})

        lines = [out("TestA/sub", "    x_test.go:1: F2: init order " + "y" * 400), out("TestA/sub", "plain line")]
        lines += [out("TestA/sub", f"PROBE: fact {i}") for i in range(80)]
        lines += [json.dumps({"Action": "pass", "Test": "TestA/sub"})]
        lines += [out("TestB", "F2: from a failing test"), json.dumps({"Action": "fail", "Test": "TestB"})]
        facts = mod.passing_facts(lines)
        self.assertEqual(list(facts), ["TestA/sub"])
        self.assertEqual(len(facts["TestA/sub"]), 60)
        self.assertTrue(all(len(f) <= 452 and ("F2:" in f or "PROBE:" in f) for f in facts["TestA/sub"]))

    def test_full_probe_line_with_step_wall_survives(self) -> None:
        # The D24 G2 census fields precede the two refusal fields, which come last
        # on the line; the numbers must survive the 450 character cut.
        probe = (
            "    sandbox_tracesize_linux_amd64_test.go:55: PROBE: trace probe-some-long-run-name bytes=51650 lines=327 "
            "old_mb_s=144.7 grammar_mb_s=22.8 best_mb_s=25.1 gated=0 old_allocs=12345 grammar_allocs=67890 "
            "step_wall_ms=123456 parse_ms=2.26 parse_share=0.0002 old_refusal=\"\" grammar_refusal=\"\""
        )
        lines = [
            json.dumps({"Action": "output", "Test": "TestA", "Output": probe + "\n"}),
            json.dumps({"Action": "pass", "Test": "TestA"}),
        ]
        (fact,) = mod.passing_facts(lines)["TestA"]
        self.assertIn("parse_share=0.0002", fact)
        self.assertTrue(fact.endswith('grammar_refusal=""'), fact)

    def test_output_cannot_inject_workflow_commands(self) -> None:
        def out(test: str, text: str) -> str:
            return json.dumps({"Action": "output", "Test": test, "Output": text})

        lines = [out("TestA", "F2: x\n::error::boom\n"), json.dumps({"Action": "pass", "Test": "TestA"})]
        lines += [out("TestB", "::error::from a failure\nplain\n"), json.dumps({"Action": "fail", "Test": "TestB"})]
        printed = [line for facts in mod.passing_facts(lines).values() for line in facts]
        printed += [line for text in mod.failing_output(lines).values() for line in text]
        self.assertTrue(printed)
        self.assertFalse([line for line in printed if line.startswith("::")], printed)
        self.assertEqual(mod.passing_facts(lines), {"TestA": ["  F2: x"]})

    def test_ci_gate_waits_for_the_job_and_rejects_nonsuccess(self) -> None:
        gate = CI.split("\n  ci-gate:\n", 1)[1]
        self.assertIn("kiln-sandbox", gate.split("runs-on:", 1)[0])
        self.assertIn("KILN_SANDBOX_RESULT: ${{ needs['kiln-sandbox'].result }}", gate)
        self.assertIn('"$KILN_SANDBOX_RESULT"', gate)


if __name__ == "__main__":
    unittest.main()
