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
    "best_mb_s=30.0 gated=1 repeats=7"
)


def probe_events(test: str, text: str, action: str = "pass") -> list[str]:
    return [
        json.dumps({"Action": "output", "Test": test, "Output": text + "\n"}),
        json.dumps({"Action": action, "Test": test}),
    ]


def with_gate(lines: list[str]) -> list[str]:
    return lines + probe_events("TestSandboxTraceGateSynthetic", SYNTHETIC_PROBE)


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
        slow = SYNTHETIC_PROBE.replace("best_mb_s=30.0", "best_mb_s=13.9")
        problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", slow))
        self.assertTrue(any("below the 14" in p for p in problems), problems)
        ok = SYNTHETIC_PROBE.replace("best_mb_s=30.0", "best_mb_s=14.0")
        self.assertEqual(mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", ok)), [])

    def test_non_finite_zero_or_missing_rates_fail(self) -> None:
        for bad in ("inf", "+Inf", "NaN", "0", "0.0", "-3.0", "abc"):
            line = SYNTHETIC_PROBE.replace("best_mb_s=30.0", f"best_mb_s={bad}")
            problems = mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", line))
            self.assertTrue(problems, bad)
        missing = SYNTHETIC_PROBE.replace("best_mb_s=30.0 ", "")
        self.assertTrue(mod.gate_problems(stream(all_pass()) + probe_events("TestSandboxTraceGateSynthetic", missing)))

    def test_a_slow_real_gated_trace_fails_even_with_the_synthetic_line(self) -> None:
        slow = REAL_PROBE.replace("best_mb_s=25.1", "best_mb_s=9.0")
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
            "    x_test.go:9: PROBE: trace evil bytes=1 lines=1 best_mb_s=9.0 gated=1 "
            'old_refusal="best_mb_s=99 gated=1" grammar_refusal="best_mb_s=99 gated=1 extra=1"'
        )
        (probe,) = mod.gated_probes(probe_events("TestSandboxRecordStep", evil))
        self.assertEqual(probe[2]["best_mb_s"], "9.0")
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

    def test_a_subtest_probe_line_has_the_subtests_indent(self) -> None:
        # go test indents a subtest's log lines by eight spaces, a sub-subtest's by twelve.
        good = "        x_test.go:9: PROBE: trace probe-pipes bytes=60000 lines=1 best_mb_s=30.0 gated=1"
        self.assertEqual(len(mod.gated_probes(probe_events("TestSandboxRecordProbe/ok", good))), 1)
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordProbe/ok", "    " + good.lstrip())), [])
        self.assertEqual(mod.gated_probes(probe_events("TestSandboxRecordProbe/ok", "    " + good)), [])

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
        self.assertIn("bash code/scripts/ci/apt-install.sh strace", body)
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
