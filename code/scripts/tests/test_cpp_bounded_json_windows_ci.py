"""NN50 exercises the production planner, not a duplicate scheduling model."""

import unittest

import test_c_bounded_json_windows_ci as bounded


class CppBoundedJsonWindowsTests(unittest.TestCase):
    plan = bounded.BoundedJsonWindowsCITests.plan

    def test_direct_and_platform_closure_select_windows(self):
        for plan in (
            {"affected_packages": ["cpp/bounded-json"], "packages": []},
            {
                "affected_packages": [],
                "packages": [],
                "platform_overrides": {
                    "windows": {"affected_packages": ["cpp/bounded-json"]}
                },
            },
        ):
            out = self.plan(plan)
            self.assertEqual(out["needs_cpp_bounded_json_windows"], "true")
            self.assertIn("windows-2025", out["build_matrix"])
            self.assertEqual(out["shard_count"], "0")

    def test_unrelated_cpp_does_not_broaden_selection(self):
        for package in ("cpp/matrix", "cpp/float-math", "python/neural-framework"):
            out = self.plan({"affected_packages": [package], "packages": []})
            self.assertEqual(out["needs_cpp_bounded_json_windows"], "false")
            self.assertNotIn("windows-2025", out["build_matrix"])

    def test_unknown_windows_closure_fails_closed(self):
        for plan, missing in (
            (None, True),
            (None, False),
            ({}, False),
            ({"affected_packages": 7, "packages": []}, False),
            ({"platform_overrides": {"windows": 7}}, False),
        ):
            out = self.plan(plan, missing=missing)
            self.assertEqual(out["needs_cpp_bounded_json_windows"], "true")
            self.assertIn("windows-2025", out["build_matrix"])

    def test_independent_ocaml_and_rust_requirements_survive(self):
        for name in ("NEEDS_OCAML", "NEEDS_RUST"):
            out = self.plan({"affected_packages": [], "packages": []}, **{name: "true"})
            self.assertEqual(out["needs_cpp_bounded_json_windows"], "false")
            self.assertIn("windows-2025", out["build_matrix"])

    def test_real_cpp_language_keeps_macos(self):
        out = self.plan(
            {
                "affected_packages": ["cpp/bounded-json"],
                "packages": [{"name": "cpp/bounded-json", "language": "cpp"}],
            }
        )
        self.assertIn("macos-latest", out["build_matrix"])

    def test_bootstrap_real_front_and_required_build_rollup(self):
        ci = bounded.CI
        self.assertIn(
            "needs_cpp_bounded_json_windows: ${{ steps.build-matrix.outputs.needs_cpp_bounded_json_windows }}",
            ci,
        )
        bootstrap = ci.split("      - name: Set up MSVC (", 1)[1].split(
            "      - name:", 1
        )[0]
        self.assertIn("needs.detect.outputs.needs_cpp_bounded_json_windows", bootstrap)
        step = ci.split("      - name: Validate bounded C++ JSON with MSVC\n", 1)[
            1
        ].split("      - name:", 1)[0]
        for required in (
            "runner.os == 'Windows'",
            "needs.detect.outputs.needs_cpp_bounded_json_windows == 'true'",
            "ISO_REQUIRE: cl clang-cl",
            "timeout-minutes: 10",
            "Push-Location code/packages/cpp/bounded-json",
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
        self.assertIn("test_cpp_bounded_json_windows_ci.py", ci)


if __name__ == "__main__":
    unittest.main()
