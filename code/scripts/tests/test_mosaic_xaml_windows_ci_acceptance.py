from __future__ import annotations

import importlib.util
import json
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
TASK_XAML_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "task-mosaic-app"
    / "conformance"
    / "xaml"
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
        for package in ("mosaic/programs/task-app", "rust/task-mosaic-app"):
            with self.subTest(package=package):
                self.assertTrue(
                    MODULE.requires_mosaic_xaml_windows(
                        {"affected_packages": [package]}
                    )
                )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
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

    def test_taskapp_report_requires_native_complete_zero_degradations(self) -> None:
        report = {
            "backend": "xaml",
            "nativeComplete": True,
            "degradations": [],
        }
        MODULE.validate_taskapp_report(report)
        report["degradations"].append({"code": "runtime.sample-fallback"})
        with self.assertRaisesRegex(ValueError, "zero degradations"):
            MODULE.validate_taskapp_report(report)

    def test_cli_validates_taskapp_report(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            report = Path(directory) / "mosaic-degradations.json"
            report.write_text(
                json.dumps(
                    {
                        "backend": "xaml",
                        "nativeComplete": True,
                        "degradations": [],
                    }
                ),
                encoding="utf-8",
            )
            result = subprocess.run(
                [
                    sys.executable,
                    str(SCRIPT),
                    "--validate-taskapp-report",
                    str(report),
                ],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertIn("native-complete with zero degradations", result.stdout)

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
        self.assertIn("Build concrete Mosaic TaskApp WinUI shell", workflow)
        self.assertIn("timeout-minutes: 15", workflow)
        self.assertIn(
            "mosaic-compile/Cargo.toml -- pkg code/programs/mosaic/task-app",
            workflow,
        )
        self.assertIn("-p mosaic-app-conformance -p task-mosaic-app", workflow)
        self.assertIn("task_mosaic_app.dll", workflow)
        self.assertIn(
            "--backend xaml --output $output --emit-project --profile native-complete --runtime-library $taskLibrary.Path",
            workflow,
        )
        self.assertIn("--validate-taskapp-report", workflow)
        self.assertIn("TaskApp XAML degradation validation failed", workflow)
        self.assertIn("dotnet build (Split-Path -Leaf $project)", workflow)
        self.assertIn("selected by generated global.json", workflow)
        self.assertIn("TaskApp.binlog", workflow)
        self.assertIn("output.json", workflow)
        self.assertIn("TaskApp WinUI build did not produce TaskApp.exe", workflow)
        self.assertIn("mosaic_app.dll beside TaskApp.exe", workflow)
        self.assertIn("TaskApp WinUI runtime bytes differ from task-mosaic-app.dll", workflow)
        self.assertIn("TaskAppXamlRuntimeConformance.csproj", workflow)
        self.assertIn("TaskAppXamlRuntimeConformance.dll", workflow)
        self.assertIn("TaskApp XAML Rust runtime conformance failed", workflow)
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

    def test_windows_lane_checks_the_xaml_platform_library(self) -> None:
        # UI87 §7.6: the platform library TaskApp's generated project compiled
        # is installed by its window and driven headless in the same lane.
        workflow = WORKFLOW.read_text(encoding="utf-8")
        step = workflow.index("- name: Check the XAML platform library headless")
        body = workflow[step : workflow.index("\n      - name:", step + 1)]
        self.assertIn(
            "if: runner.os == 'Windows' && needs.detect.outputs.needs_mosaic_xaml_windows == 'true'",
            body,
        )
        self.assertIn("'mosaic-xaml-taskapp/xaml'", body)
        self.assertIn("MosaicPlatformEffects.Install(this, appKinds: null);", body)
        self.assertIn("conformance/xaml-platform-effects/*", body)
        self.assertIn("'MosaicPlatformEffects.cs', 'MosaicRuntimeHost.cs'", body)
        self.assertIn("XamlPlatformEffectsConformance.csproj", body)
        self.assertIn("Mosaic XAML platform library conformance failed", body)
        # After the TaskApp build that generates the files it copies.
        self.assertLess(
            workflow.index("Build concrete Mosaic TaskApp WinUI shell"), step
        )

        harness = XAML_CONFORMANCE.parent / "xaml-platform-effects"
        project = (harness / "XamlPlatformEffectsConformance.csproj").read_text(
            encoding="utf-8"
        )
        program = (harness / "Program.cs").read_text(encoding="utf-8")
        self.assertIn("<TargetFramework>net9.0</TargetFramework>", project)
        self.assertIn("MOSAIC_HEADLESS_TEST", project)
        self.assertNotIn("Microsoft.WindowsAppSDK", project)
        color_stub = (harness / "WindowsColorStub.cs").read_text(encoding="utf-8")
        self.assertIn("namespace Windows.UI;", color_stub)
        self.assertIn("Mosaic XAML platform effects conformance passed", program)
        self.assertIn("class FakeDialogs : IMosaicFileDialogs", program)
        self.assertIn("class FakeHost : IMosaicPlatformEffectHost", program)

    def test_layout_rules_require_acceptance(self) -> None:
        # UI48 §7.11: the rules the WinUI window switches layouts by.
        self.assertTrue(
            MODULE.requires_mosaic_xaml_windows(
                {"affected_packages": ["rust/mosaic-package-manifest"]}
            )
        )

    def test_windows_lane_resizes_the_layout_variants_fixture(self) -> None:
        # UI48 §7.11: the gate that changes the environment. The fixture is
        # built native-complete against the conformance runtime, its source
        # contract checked, its WinUI project built, and the real window
        # resized across 600 effective pixels both ways.
        workflow = WORKFLOW.read_text(encoding="utf-8")
        step = workflow.index("- name: Resize the WinUI layout-variants fixture (UI48 §7.11)")
        body = workflow[step : workflow.index("\n      - name:", step + 1)]
        self.assertIn(
            "if: runner.os == 'Windows' && needs.detect.outputs.needs_mosaic_xaml_windows == 'true'",
            body,
        )
        self.assertIn("shell: pwsh", body)
        self.assertIn(
            "pkg code/packages/rust/mosaic-emit-xaml/fixtures/layout-variants --backend xaml",
            body,
        )
        self.assertIn("--profile native-complete --runtime-library $library.Path", body)
        self.assertIn("mosaic_app_conformance.dll", body)
        for marker in (
            '("compact", new[] { ("sizeClass", "compact") }),',
            "var root = new LayoutProbeCompact();",
            "MosaicRuntimeHost.ApplyRequiredProps(next, RequiredProps);",
            "this.layoutSwitchQueued = this.DispatcherQueue.TryEnqueue(SwitchLayout);",
            "if (MosaicRuntimeHost.IsSettling)",
        ):
            self.assertIn(marker, body)
        self.assertIn("LayoutProbe.compact.Event.cs", body)
        self.assertIn("public event EventHandler<LayoutProbeEvent>? Dispatch;", body)
        self.assertIn("dotnet build (Split-Path -Leaf $project)", body)
        self.assertIn("code/scripts/mosaic-xaml-layout-variants-smoke.ps1", body)
        # Every external command's failure is a thrown error, never ignored.
        # cargo build, mosaic-compile and the smoke; dotnet build below.
        self.assertEqual(body.count("if ($LASTEXITCODE -ne 0)"), 3)
        self.assertIn("if ($buildExitCode -ne 0)", body)

        fixture = XAML_PACKAGE.parents[1] / "mosaic-emit-xaml" / "fixtures" / "layout-variants"
        self.assertNotIn(
            "[[app.layouts]]",
            (fixture / "mosaic-package.toml").read_text(encoding="utf-8").replace(
                "# No `[[app.layouts]]`", ""
            ),
            "the compact layout is selected by convention",
        )
        interface = (fixture / "src" / "LayoutProbe.mil").read_text(encoding="utf-8")
        # Two of the conformance runtime's props, so it drives the window.
        self.assertIn("slot platform : text ;", interface)
        self.assertIn("slot status : text ;", interface)
        counter = (XAML_PACKAGE / "src" / "Counter.mil").read_text(encoding="utf-8")
        self.assertIn("slot platform : text;", counter)
        self.assertIn("slot status : text;", counter)
        for layout, marker in (
            ("LayoutProbe.mll", "Layout: default"),
            ("LayoutProbe.compact.mll", "Layout: compact"),
        ):
            self.assertIn(marker, (fixture / "src" / layout).read_text(encoding="utf-8"))

        smoke = (
            WORKFLOW.parents[2] / "code" / "scripts" / "mosaic-xaml-layout-variants-smoke.ps1"
        ).read_text(encoding="utf-8")
        self.assertIn("$ErrorActionPreference = 'Stop'", smoke)
        self.assertIn("SetWindowPos", smoke)
        self.assertIn("'Layout: default'", smoke)
        self.assertIn("'Layout: compact'", smoke)
        # Wide, narrow, wide, narrow: both directions, twice.
        self.assertEqual(smoke.count("Resize-AndExpect $process $root 1300"), 2)
        self.assertEqual(smoke.count("Resize-AndExpect $process $root 420"), 2)

    def test_conformance_engine_has_a_real_mosaic_package(self) -> None:
        self.assertTrue((XAML_PACKAGE / "mosaic-package.toml").is_file())
        for suffix in ("mil", "mll", "msl"):
            self.assertTrue((XAML_PACKAGE / "src" / f"Counter.{suffix}").is_file())

    def test_task_app_conformance_drives_the_generated_binding(self) -> None:
        project = (
            TASK_XAML_CONFORMANCE / "TaskAppXamlRuntimeConformance.csproj"
        ).read_text(encoding="utf-8")
        program = (TASK_XAML_CONFORMANCE / "Program.cs").read_text(encoding="utf-8")
        color_stub = (TASK_XAML_CONFORMANCE / "WindowsColorStub.cs").read_text(
            encoding="utf-8"
        )
        self.assertIn("<TargetFramework>net9.0</TargetFramework>", project)
        self.assertNotIn("Microsoft.WindowsAppSDK", project)
        self.assertIn("MosaicRuntimeHost.LoadRequired()", program)
        self.assertIn('"task-rows", "ring-percent"', program)
        self.assertIn('"onNewTaskNameChange"', program)
        self.assertIn('"onNewTaskDueChange"', program)
        self.assertIn('"onToggleProjectComplexity"', program)
        self.assertIn('"onToggleTask"', program)
        self.assertIn('"onDeleteTask"', program)
        self.assertIn("Native acceptance task", program)
        self.assertIn("Persisted native task", program)
        self.assertIn("MOSAIC_EXPECT_RESTORED", program)
        self.assertIn("MosaicRuntimeHost.HandleRequiredEvent", program)
        self.assertIn("TaskApp XAML native lifecycle conformance passed", program)
        self.assertIn("TaskApp XAML persisted-restart conformance passed", program)
        self.assertIn("namespace Windows.UI;", color_stub)


if __name__ == "__main__":
    unittest.main()
