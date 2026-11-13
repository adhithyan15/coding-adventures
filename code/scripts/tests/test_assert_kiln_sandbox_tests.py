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
            good.write_text("\n".join(stream(all_pass())), encoding="utf-8")
            bad = Path(tmp) / "bad.json"
            bad.write_text("\n".join(stream({})), encoding="utf-8")
            self.assertEqual(mod.main(["x", str(good)]), 0)
            self.assertEqual(mod.main(["x", str(bad)]), 1)
            self.assertEqual(mod.main(["x", str(Path(tmp) / "missing")]), 2)
            self.assertEqual(mod.main(["x"]), 2)

    def test_every_required_name_exists_in_the_go_tests(self) -> None:
        source = (REPO / "code" / "programs" / "go" / "kiln" / "sandbox_linux_amd64_test.go").read_text(encoding="utf-8")
        declared = set(re.findall(r"^func (TestSandbox\w+)\(", source, re.M))
        self.assertEqual(declared, set(mod.REQUIRED))


class CiWiring(unittest.TestCase):
    def job(self) -> str:
        return CI.split("\n  kiln-sandbox:\n", 1)[1].split("\n  d18f-message-conformance:\n", 1)[0]

    def test_job_requires_the_sandbox_and_asserts_the_stream(self) -> None:
        body = self.job()
        self.assertIn("runs-on: ubuntu-24.04", body)
        self.assertIn("sudo sysctl -w kernel.apparmor_restrict_unprivileged_userns=0", body)
        self.assertNotRegex(body, r"if:.*apparmor")
        self.assertIn("bash code/scripts/ci/apt-install.sh strace", body)
        self.assertIn("KILN_REQUIRE_SANDBOX: '1'", body)
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
        self.assertEqual(mod.failing_output(lines), {"TestSandboxNoHostIPC": ["boom: exit 125"]})

    def test_ci_gate_waits_for_the_job_and_rejects_nonsuccess(self) -> None:
        gate = CI.split("\n  ci-gate:\n", 1)[1]
        self.assertIn("kiln-sandbox", gate.split("runs-on:", 1)[0])
        self.assertIn("KILN_SANDBOX_RESULT: ${{ needs['kiln-sandbox'].result }}", gate)
        self.assertIn('"$KILN_SANDBOX_RESULT"', gate)


if __name__ == "__main__":
    unittest.main()
