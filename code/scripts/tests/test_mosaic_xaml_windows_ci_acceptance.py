from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_xaml_windows_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
XAML_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "xaml"
)
XAML_PACKAGE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-conformance"
    / "package"
)
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
        self.assertIn("Build complete Mosaic TaskApp WinUI shell", workflow)
        self.assertIn("timeout-minutes: 15", workflow)
        self.assertIn(
            "mosaic-compile/Cargo.toml -- pkg code/programs/mosaic/task-app",
            workflow,
        )
        self.assertIn("--backend xaml --output $output --emit-project", workflow)
        self.assertIn("dotnet build (Split-Path -Leaf $project)", workflow)
        self.assertIn("selected by generated global.json", workflow)
        self.assertIn("TaskApp.binlog", workflow)
        self.assertIn("output.json", workflow)
        self.assertIn("TaskApp WinUI build did not produce TaskApp.exe", workflow)
        self.assertIn("mosaic-pkg-rating-controls --backend xaml", workflow)
        self.assertIn("--emit-project --profile native-complete", workflow)
        self.assertIn("--runtime-library $library.Path", workflow)
        self.assertIn("mosaic-xaml-native-complete-rating-controls", workflow)
        self.assertIn("Native-complete XAML generation reported degradations", workflow)
        self.assertIn("dotnet build (Split-Path -Leaf $strictProject)", workflow)
        self.assertIn("mosaic-xaml-bundled-conformance", workflow)
        self.assertIn(
            "pkg code/packages/rust/mosaic-app-conformance/package --backend xaml",
            workflow,
        )
        self.assertIn("dotnet build (Split-Path -Leaf $bundledProject)", workflow)
        self.assertIn("Get-FileHash $library.Path -Algorithm SHA256", workflow)
        self.assertIn("mosaic_app.dll beside the executable", workflow)
        self.assertIn("mosaic-xaml-toolkit", workflow)
        self.assertIn("mosaic-pkg-toolkit --backend xaml", workflow)
        self.assertIn("dotnet build (Split-Path -Leaf $toolkitProject)", workflow)
        self.assertIn("Complete Mosaic toolkit XAML build failed", workflow)
        self.assertIn("MOSAIC_SKIP_INTERACTIVE_WINDOWS_ACCEPTANCE: '1'", workflow)
        self.assertNotIn("Start-Process -FilePath $executable", workflow)
        self.assertIn("Round-trip Rust engine through standard XAML binding", workflow)
        self.assertIn("timeout-minutes: 10", workflow)
        self.assertIn("cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance", workflow)
        self.assertIn("XamlRuntimeConformance.csproj", workflow)
        self.assertIn("MOSAIC_APP_LIBRARY", workflow)
        self.assertIn("Remove-Item Env:MOSAIC_APP_LIBRARY", workflow)
        self.assertIn("-Filter 'XamlRuntimeConformance.dll'", workflow)
        self.assertIn("did not produce XamlRuntimeConformance.dll", workflow)
        self.assertIn("dotnet $harnessBinary.FullName", workflow)
        self.assertNotIn("bin/Release/net9.0/XamlRuntimeConformance.dll", workflow)
        self.assertIn("--expect-missing-prop-failure", workflow)
        self.assertIn("--expect-required-failure", workflow)

    def test_console_conformance_does_not_bootstrap_winui(self) -> None:
        project = (XAML_CONFORMANCE / "XamlRuntimeConformance.csproj").read_text(
            encoding="utf-8"
        )
        color_stub = (XAML_CONFORMANCE / "WindowsColorStub.cs").read_text(
            encoding="utf-8"
        )
        self.assertIn("<TargetFramework>net9.0</TargetFramework>", project)
        self.assertNotIn("Microsoft.WindowsAppSDK", project)
        self.assertIn("namespace Windows.UI;", color_stub)
        self.assertIn("Color FromArgb", color_stub)

    def test_conformance_engine_has_a_real_mosaic_package(self) -> None:
        self.assertTrue((XAML_PACKAGE / "mosaic-package.toml").is_file())
        for suffix in ("mil", "mll", "msl"):
            self.assertTrue((XAML_PACKAGE / "src" / f"Counter.{suffix}").is_file())


if __name__ == "__main__":
    unittest.main()
