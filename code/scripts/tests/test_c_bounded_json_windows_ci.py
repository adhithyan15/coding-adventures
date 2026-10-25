"""Execute the production planner: a Unix green check is not MSVC evidence."""

from __future__ import annotations

import json
import os
import subprocess
import sys
import tempfile
import textwrap
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
CI = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
BLOCK = CI.split("      - name: Plan build matrix\n", 1)[1].split(
    "      - name: Upload build plan\n", 1
)[0]
SCRIPT = textwrap.dedent(BLOCK.split("        run: |\n", 1)[1])
SCRIPT = SCRIPT.split("python3 - <<'PY' >> \"$GITHUB_OUTPUT\"\n", 1)[1].rsplit("PY", 1)[
    0
]


class BoundedJsonWindowsCITests(unittest.TestCase):
    def plan(self, plan: object, *, missing: bool = False, **flags: str) -> dict:
        # Only trusted repository code is executed. Plan data stays JSON, not argv.
        env = {
            k: v
            for k, v in os.environ.items()
            if not k.startswith(("NEEDS_", "RUN_", "FULL_BUILD", "BUILD_SHARDS"))
        }
        env.update(FULL_BUILD="false", EVENT_NAME="pull_request", **flags)
        with tempfile.TemporaryDirectory() as directory:
            if not missing:
                Path(directory, "build-plan.json").write_text(
                    json.dumps(plan), encoding="utf-8"
                )
            result = subprocess.run(
                [sys.executable, "-c", SCRIPT],
                cwd=directory,
                env=env,
                capture_output=True,
                text=True,
                timeout=30,
                check=True,
            )
        return dict(line.split("=", 1) for line in result.stdout.splitlines())

    def test_selected_package_keeps_windows_without_other_toolchain_flags(self):
        out = self.plan({"affected_packages": ["c/bounded-json"], "packages": []})
        self.assertEqual(out["run_c_bounded_json_windows"], "true")
        self.assertIn("windows-2025", out["build_matrix"])
        self.assertEqual(out["shard_count"], "0")

    def test_unrelated_plan_does_not_broaden_cpp_windows_scheduling(self):
        out = self.plan({"affected_packages": ["cpp/matrix"], "packages": []})
        self.assertEqual(out["run_c_bounded_json_windows"], "false")
        self.assertNotIn("windows-2025", out["build_matrix"])

    def test_dependency_aware_windows_override_selects_package(self):
        out = self.plan(
            {
                "affected_packages": [],
                "packages": [],
                "platform_overrides": {
                    "windows": {"affected_packages": ["c/bounded-json"]}
                },
            }
        )
        self.assertIn("windows-2025", out["build_matrix"])

    def test_unreadable_or_unbounded_plan_fails_closed(self):
        for plan, missing in [(None, True), (None, False), ({}, False)]:
            with self.subTest(plan=plan, missing=missing):
                out = self.plan(plan, missing=missing)
                self.assertEqual(out["run_c_bounded_json_windows"], "true")
                self.assertIn("windows-2025", out["build_matrix"])

    def test_existing_windows_reasons_remain_independent(self):
        out = self.plan({"affected_packages": [], "packages": []}, NEEDS_RUST="true")
        self.assertEqual(out["run_c_bounded_json_windows"], "false")
        self.assertIn("windows-2025", out["build_matrix"])

    def test_requirement_wires_bootstrap_execution_and_required_rollup(self):
        self.assertIn(
            "run_c_bounded_json_windows: ${{ steps.build-matrix.outputs.run_c_bounded_json_windows }}",
            CI,
        )
        bootstrap = CI.split("      - name: Set up MSVC (", 1)[1].split(
            "      - name:", 1
        )[0]
        self.assertIn(
            "needs.detect.outputs.run_c_bounded_json_windows == 'true'", bootstrap
        )
        native = CI.split("      - name: Validate bounded C JSON with MSVC\n", 1)[
            1
        ].split("      - name:", 1)[0]
        self.assertIn("runner.os == 'Windows'", native)
        self.assertIn("ISO_REQUIRE: cl", native)
        self.assertIn("-File tools/selftest.ps1", native)
        self.assertIn("-File tools/run.ps1", native)
        self.assertEqual(native.count("if ($LASTEXITCODE -ne 0)"), 2)
        self.assertNotIn("continue-on-error", native)
        rollup = CI.split("\n  ci-gate:\n", 1)[1]
        self.assertIn("build", rollup.split("runs-on:", 1)[0])
        self.assertIn("BUILD_RESULT: ${{ needs.build.result }}", rollup)
        self.assertIn('"$BUILD_RESULT"', rollup)


if __name__ == "__main__":
    unittest.main()
