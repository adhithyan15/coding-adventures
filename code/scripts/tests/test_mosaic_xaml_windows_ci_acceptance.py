from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_xaml_windows_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
SPEC = importlib.util.spec_from_file_location("mosaic_xaml_windows_ci_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicXamlWindowsCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(MODULE.requires_mosaic_xaml_windows({"affected_packages": None}))

    def test_xaml_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": ["rust/mosaic-emit-xaml"]}
            )
        )

    def test_standard_runtime_binding_requires_acceptance(self) -> None:
        for package in (
            "rust/mosaic-app-bindings",
            "rust/mosaic-app-capi",
            "rust/mosaic-app-conformance",
            "rust/mosaic-app-runtime",
        ):
            with self.subTest(package=package):
                self.assertTrue(
                    MODULE.requires_mosaic_xaml_windows(
                        {"affected_packages": [package]}
                    )
                )

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": ["unknown/programs/task-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": ["unknown/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_xaml_windows({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-xaml"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_windows_toolchains_and_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "needs_mosaic_xaml_windows: "
            "${{ steps.mosaic-xaml-windows.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_xaml_windows_ci_acceptance.py",
            workflow,
        )
        self.assertIn("Build and launch complete Mosaic TaskApp WinUI shell", workflow)
        self.assertIn(
            "mosaic-compile/Cargo.toml -- pkg code/programs/mosaic/task-app",
            workflow,
        )
        self.assertIn("--backend xaml --output $output --emit-project", workflow)
        self.assertIn("dotnet build (Split-Path -Leaf $project)", workflow)
        self.assertIn("selected by generated global.json", workflow)
        self.assertIn("TaskApp.binlog", workflow)
        self.assertIn("output.json", workflow)
        self.assertIn("Start-Process -FilePath $executable", workflow)
        self.assertIn("Round-trip Rust engine through standard XAML binding", workflow)
        self.assertIn("cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance", workflow)
        self.assertIn("XamlRuntimeConformance.csproj", workflow)
        self.assertIn("MOSAIC_APP_LIBRARY", workflow)


if __name__ == "__main__":
    unittest.main()
