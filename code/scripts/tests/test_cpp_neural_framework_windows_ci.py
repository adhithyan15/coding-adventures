"""NN51's production planner and native Windows execution ownership."""

import unittest

import test_c_bounded_json_windows_ci as bounded


class CppNeuralWindowsTests(unittest.TestCase):
    plan = bounded.BoundedJsonWindowsCITests.plan

    def test_direct_and_windows_closure_keep_training_lane(self):
        for plan in (
            {"affected_packages": ["cpp/neural-framework"], "packages": []},
            {
                "affected_packages": [],
                "packages": [],
                "platform_overrides": {
                    "windows": {"affected_packages": ["cpp/neural-framework"]}
                },
            },
        ):
            out = self.plan(plan)
            self.assertEqual(out["needs_cpp_neural_framework_windows"], "true")
            self.assertIn("windows-2025", out["build_matrix"])
            self.assertEqual(out["shard_count"], "0")

    def test_unrelated_cpp_is_not_broadened(self):
        for package in ("cpp/matrix", "cpp/float-math", "python/neural-framework"):
            out = self.plan({"affected_packages": [package], "packages": []})
            self.assertEqual(out["needs_cpp_neural_framework_windows"], "false")
            self.assertNotIn("windows-2025", out["build_matrix"])

    def test_unknown_closure_fails_closed(self):
        for plan in (
            None,
            {},
            {"affected_packages": 7},
            {"platform_overrides": {"windows": 7}},
        ):
            out = self.plan(plan)
            self.assertEqual(out["needs_cpp_neural_framework_windows"], "true")
            self.assertIn("windows-2025", out["build_matrix"])

    def test_other_native_owners_remain_independent(self):
        for package, output in (
            ("cpp/bounded-json", "needs_cpp_bounded_json_windows"),
            ("c/neural-framework", "needs_c_neural_framework_windows"),
            ("c/bounded-json", "needs_c_bounded_json_windows"),
        ):
            out = self.plan({"affected_packages": [package], "packages": []})
            self.assertEqual(out[output], "true")
            self.assertEqual(out["needs_cpp_neural_framework_windows"], "false")

    def test_ocaml_and_rust_requirements_survive(self):
        for flag in ("NEEDS_OCAML", "NEEDS_RUST"):
            out = self.plan({"affected_packages": [], "packages": []}, **{flag: "true"})
            self.assertIn("windows-2025", out["build_matrix"])
            self.assertEqual(out["needs_cpp_neural_framework_windows"], "false")

    def test_actual_language_plan_keeps_three_platforms(self):
        out = self.plan(
            {
                "affected_packages": ["cpp/neural-framework"],
                "packages": [{"name": "cpp/neural-framework", "language": "cpp"}],
            },
            NEEDS_CPP="true",
        )
        for os in ("ubuntu-latest", "macos-latest", "windows-2025"):
            self.assertIn(os, out["build_matrix"])

    def test_real_front_bootstrap_and_required_rollup(self):
        ci = bounded.CI
        self.assertIn(
            "needs_cpp_neural_framework_windows: ${{ steps.build-matrix.outputs.needs_cpp_neural_framework_windows }}",
            ci,
        )
        bootstrap = ci.split("      - name: Set up MSVC (", 1)[1].split(
            "      - name:", 1
        )[0]
        self.assertIn(
            "needs.detect.outputs.needs_cpp_neural_framework_windows", bootstrap
        )
        step = ci.split(
            "      - name: Validate native C++ dense training with MSVC\n", 1
        )[1].split("      - name:", 1)[0]
        for required in (
            "runner.os == 'Windows'",
            "needs.detect.outputs.needs_cpp_neural_framework_windows == 'true'",
            "ISO_REQUIRE: cl clang-cl",
            "timeout-minutes: 10",
            "-File tools/selftest.ps1",
            "Push-Location code/packages/cpp/neural-framework",
            "-File tools/run.ps1",
        ):
            self.assertIn(required, step)
        self.assertEqual(step.count("if ($LASTEXITCODE -ne 0)"), 2)
        self.assertNotIn("continue-on-error", step)
        rollup = ci.split("\n  ci-gate:\n", 1)[1]
        self.assertIn("BUILD_RESULT: ${{ needs.build.result }}", rollup)
        self.assertIn("test_cpp_neural_framework_windows_ci.py", ci)


if __name__ == "__main__":
    unittest.main()
