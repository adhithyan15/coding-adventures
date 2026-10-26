"""NN48 pins the actual matrix planner and Windows training execution owner."""

import unittest

import test_c_bounded_json_windows_ci as bounded


class NativeCWindowsCITests(unittest.TestCase):
    plan = bounded.BoundedJsonWindowsCITests.plan

    def test_native_package_and_platform_dependency_closure_keep_windows(self):
        for plan in (
            {"affected_packages": ["c/neural-framework"], "packages": []},
            {
                "affected_packages": [],
                "packages": [],
                "platform_overrides": {
                    "windows": {"affected_packages": ["c/neural-framework"]}
                },
            },
        ):
            out = self.plan(plan)
            self.assertEqual(out["needs_c_neural_framework_windows"], "true")
            self.assertIn("windows-2025", out["build_matrix"])

    def test_unrelated_packages_do_not_broaden_windows(self):
        for package in ("cpp/matrix", "c/matrix", "python/neural-framework"):
            out = self.plan({"affected_packages": [package], "packages": []})
            self.assertEqual(out["needs_c_neural_framework_windows"], "false")
            self.assertNotIn("windows-2025", out["build_matrix"])

    def test_unknown_plan_fails_closed(self):
        for plan, missing in (
            (None, True),
            (None, False),
            ({}, False),
            ({"affected_packages": 7, "packages": []}, False),
        ):
            out = self.plan(plan, missing=missing)
            self.assertEqual(out["needs_c_neural_framework_windows"], "true")
            self.assertIn("windows-2025", out["build_matrix"])

    def test_existing_independent_flags_are_not_replaced(self):
        for requirement in ("NEEDS_OCAML", "NEEDS_RUST"):
            out = self.plan(
                {"affected_packages": [], "packages": []}, **{requirement: "true"}
            )
            self.assertEqual(out["needs_c_neural_framework_windows"], "false")
            self.assertIn("windows-2025", out["build_matrix"])

    def test_bootstrap_execution_and_required_rollup_wiring(self):
        ci = bounded.CI
        self.assertIn(
            "needs_c_neural_framework_windows: ${{ steps.build-matrix.outputs.needs_c_neural_framework_windows }}",
            ci,
        )
        bootstrap = ci.split("      - name: Set up MSVC (", 1)[1].split(
            "      - name:", 1
        )[0]
        self.assertIn(
            "needs.detect.outputs.needs_c_neural_framework_windows == 'true'", bootstrap
        )
        step = ci.split(
            "      - name: Validate native C dense training with MSVC\n", 1
        )[1].split("      - name:", 1)[0]
        for required in (
            "runner.os == 'Windows'",
            "ISO_REQUIRE: cl clang-cl",
            "timeout-minutes: 10",
            "Push-Location code/packages/c/neural-framework",
            "-File tools/selftest.ps1",
            "-File tools/run.ps1",
        ):
            self.assertIn(required, step)
        self.assertEqual(step.count("if ($LASTEXITCODE -ne 0)"), 2)
        self.assertNotIn("continue-on-error", step)
        rollup = ci.split("\n  ci-gate:\n", 1)[1]
        self.assertIn("build", rollup.split("runs-on:", 1)[0])
        self.assertIn("BUILD_RESULT: ${{ needs.build.result }}", rollup)
        self.assertIn('"$BUILD_RESULT"', rollup)
        self.assertIn("test_c_neural_framework_windows_ci.py", ci)


if __name__ == "__main__":
    unittest.main()
