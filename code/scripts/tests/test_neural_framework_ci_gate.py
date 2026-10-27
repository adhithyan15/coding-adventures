"""Pin NN37's scheduling and execute the required roll-up's actual shell.

An optional green workflow is evidence for a human, but cannot prevent a merge.
The caller must own a dependency and inspect the selected job's real result.
These tests exercise that distinction without credentials or network access.
"""

from __future__ import annotations

import json
import os
import re
import shutil
import subprocess
import textwrap
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CI = ROOT / ".github/workflows/ci.yml"
REUSABLE = ROOT / ".github/workflows/neural-framework.yml"
GATE_ID = "neural-framework-conformance"
FLAG = "run_neural_framework_conformance"


def job(source: str, name: str) -> str:
    """Take one literal job block, never a similarly named comment/step."""
    return (
        source.split(f"\n  {name}:\n", 1)[1]
        .split("\n  #", 1)[0]
        .split("\n  ci-gate:", 1)[0]
    )


class NeuralFrameworkCIGateTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.ci = CI.read_text(encoding="utf-8")
        cls.reusable = REUSABLE.read_text(encoding="utf-8")
        cls.gate = json.loads(
            (ROOT / "code/specs/data/ci-gates.json").read_text(encoding="utf-8")
        )["gates"][GATE_ID]
        cls.rollup = job(cls.ci, "ci-gate")

    def test_registry_covers_native_and_nonpackage_inputs(self) -> None:
        self.assertEqual(self.gate["scope"], "job")
        self.assertEqual(
            set(self.gate["packages"]),
            {
                "python/neural-framework",
                "typescript/neural-framework",
                "python/matrix",
                "typescript/matrix",
                "rust/neural-framework",
                "rust/matrix",
                "rust/bounded-json",
                "go/neural-framework",
                "go/matrix",
                "ruby/neural-framework",
                "ruby/matrix",
                "c/neural-framework",
                "c/matrix",
                "c/float-math",
                "c/bounded-json",
                "c/iso-harness",
            },
        )
        paths = set(self.gate["paths"])
        for pattern in (
            "code/specs/fixtures/neural-framework-v1/**",
            "code/scripts/validate_neural_framework_*.py",
            "code/scripts/tests/test_neural_framework_*.py",
            "code/specs/data/ci-gates.json",
            ".github/workflows/ci.yml",
            ".github/workflows/neural-framework.yml",
            "code/packages/typescript/matrix-rust-napi/**",
            "code/packages/rust/Cargo.toml",
        ):
            self.assertIn(pattern, paths)
        for package in self.gate["packages"]:
            self.assertIn(f"code/packages/{package}/**", paths)
        self.assertNotIn("**", paths)

    def test_caller_and_rollup_own_the_verdict(self) -> None:
        caller = job(self.ci, GATE_ID)
        self.assertIn("needs: detect", caller)
        self.assertIn(f"if: needs.detect.outputs.{FLAG} == 'true'", caller)
        self.assertIn("uses: ./.github/workflows/neural-framework.yml", caller)
        self.assertIn(f"{FLAG}: ${{{{ steps.detect.outputs.{FLAG} }}}}", self.ci)
        self.assertIn(GATE_ID, self.rollup.split("if: always()", 1)[0])
        self.assertIn(
            f"NEURAL_FRAMEWORK_REQUIRED: ${{{{ needs.detect.outputs.{FLAG} }}}}",
            self.rollup,
        )
        self.assertIn(
            "NEURAL_FRAMEWORK_RESULT: ${{ needs['neural-framework-conformance'].result }}",
            self.rollup,
        )
        self.assertEqual(self.rollup.count("if: always()"), 2)
        self.assertIn("current_head", self.rollup)
        self.assertIn("test_neural_framework_ci_gate.py", self.ci)

    def test_one_reusable_numerical_workflow(self) -> None:
        triggers = self.reusable.split("\npermissions:", 1)[0]
        self.assertIn("  workflow_call:", triggers)
        self.assertIn("  workflow_dispatch:", triggers)
        self.assertNotIn("  pull_request:", triggers)
        self.assertNotIn("  push:", triggers)
        for command in (
            "python -m pytest",
            "npm run build",
            "npm run test:coverage",
            "python code/scripts/validate_neural_framework_interchange.py",
            "cargo +stable test -p neural-framework",
            "cargo +stable build -p neural-framework --release --example xor",
            "test_neural_framework_interchange.py",
            "go mod tidy",
            "go vet ./...",
            "go test ./... -cover",
            "go test -race ./...",
            "go build ./...",
            "python ../../../scripts/validate_neural_framework_go.py",
            "test_neural_framework_go.py",
            "ruby scripts/build.rb",
            "python ../../../scripts/validate_neural_framework_ruby.py",
            "test_neural_framework_ruby.py",
            "sh tools/run.sh",
            "test_neural_framework_c.py",
            "--c-executable code/packages/c/neural-framework/_build/gcc-native-xor",
        ):
            self.assertIn(command, self.reusable)
        self.assertIn("working-directory: code/packages/go/matrix", self.reusable)
        self.assertIn(
            "working-directory: code/packages/go/neural-framework", self.reusable
        )
        self.assertIn("actions/setup-go@", self.reusable)
        self.assertIn(
            "ruby/setup-ruby@26e5b7d197ebf60545ee9667bcbace0be11ccaec", self.reusable
        )
        self.assertIn(
            "working-directory: code/packages/ruby/neural-framework", self.reusable
        )
        self.assertIn("persist-credentials: false", self.reusable)
        self.assertIn(
            "working-directory: code/packages/c/neural-framework", self.reusable
        )
        self.assertIn('ISO_REQUIRE: "gcc clang"', self.reusable)
        self.assertIn('CI: "true"', self.reusable)
        self.assertNotIn("sparse-checkout:", self.reusable)
        self.assertNotIn("secrets: inherit", self.ci)

    @unittest.skipUnless(shutil.which("bash"), "roll-up uses Bash, available on CI")
    def test_actual_rollup_fails_closed(self) -> None:
        script = textwrap.dedent(self.rollup.split("        run: |\n", 1)[1])
        # Use a non-PR event to avoid the authenticated head read. The complete
        # production shell still evaluates all dependency results and exits.
        env = os.environ.copy()
        env.update(
            {
                name: "success"
                for name in re.findall(
                    r"^          ([A-Z_0-9]+_RESULT):", self.rollup, re.MULTILINE
                )
            }
        )
        env["EVENT_NAME"] = "workflow_dispatch"
        cases = [
            ("true", "success", True),
            ("false", "skipped", True),
            ("false", "success", True),
            ("true", "skipped", False),
            ("true", "failure", False),
            ("true", "cancelled", False),
            ("true", "", False),
            ("true", "pending", False),
            ("false", "failure", False),
            ("", "success", False),
            ("garbage", "success", False),
        ]
        for required, result, accepted in cases:
            with self.subTest(required=required, result=result):
                env.update(
                    NEURAL_FRAMEWORK_REQUIRED=required, NEURAL_FRAMEWORK_RESULT=result
                )
                completed = subprocess.run(  # nosec B603: fixed shell, checked-in script
                    [shutil.which("bash"), "--noprofile", "--norc", "-c", script],
                    env=env,
                    capture_output=True,
                    text=True,
                    check=False,
                    timeout=10,
                )
                self.assertEqual(
                    completed.returncode == 0,
                    accepted,
                    completed.stdout + completed.stderr,
                )


if __name__ == "__main__":
    unittest.main()
