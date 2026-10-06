from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_flutter_runtime_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
FLUTTER_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "flutter"
)
SPEC = importlib.util.spec_from_file_location(
    "mosaic_flutter_runtime_ci_acceptance", SCRIPT
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicFlutterRuntimeCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_flutter_runtime({"affected_packages": None})
        )

    def test_flutter_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_flutter_runtime(
                {"affected_packages": ["rust/mosaic-emit-flutter"]}
            )
        )

    def test_standard_runtime_binding_requires_acceptance(self) -> None:
        for package in (
            "rust/mosaic-app-bindings",
            "rust/mosaic-app-capi",
            "rust/mosaic-app-conformance",
            "rust/mosaic-app-runtime",
            "rust/task-mosaic-app",
        ):
            with self.subTest(package=package):
                self.assertTrue(
                    MODULE.requires_mosaic_flutter_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_flutter_runtime(
                {"affected_packages": ["mosaic/programs/task-app"]}
            )
        )

    def test_engram_app_requires_acceptance(self) -> None:
        """A change touching only Engram's Flutter host must fire this lane.

        It did not until the `[host_effects]` migration, and the consequence
        was not theoretical: nothing had ever emitted Engram on Flutter with
        `--profile native-complete`, and when something finally did, the
        project failed to compile because the hand-written host overrode the
        generated one without defining `loadRequired`.
        """

        self.assertTrue(
            MODULE.requires_mosaic_flutter_runtime(
                {"affected_packages": ["mosaic/programs/engram-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_flutter_runtime(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_flutter_runtime(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_flutter_runtime(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_flutter_runtime({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-flutter"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_rust_dart_and_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "needs_mosaic_flutter_runtime: "
            "${{ steps.mosaic-flutter-runtime.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_flutter_runtime_ci_acceptance.py",
            workflow,
        )
        self.assertIn("Round-trip Rust engine through standard Flutter binding", workflow)
        self.assertIn("needs_mosaic_flutter_runtime == 'true'", workflow)
        self.assertIn("timeout-minutes: 25", workflow)
        # Asserts the ACTION, not how it is pinned. The literal
        # `@v2` was asserted here, which made this test a tripwire against
        # SHA-pinning that action -- a test that fails when the workflow is
        # made more secure is a test arguing for the insecure form.
        self.assertRegex(workflow, r"uses: subosito/flutter-action@\S+")
        self.assertIn("flutter-version: '3.44.0'", workflow)
        self.assertIn("flutter config --enable-native-assets", workflow)
        self.assertIn("sudo apt-get install -y libgtk-3-dev", workflow)
        self.assertIn(
            "--backend flutter --output \"$taskapp_output\" --emit-project",
            workflow,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance",
            workflow,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app",
            workflow,
        )
        self.assertIn('libtask_mosaic_app.so', workflow)
        self.assertIn('cmp "$task_runtime_library" "$bundled_taskapp_runtime"', workflow)
        self.assertIn("-path '*/bundle/mosaic_taskapp_acceptance'", workflow)
        self.assertIn('run-under-xvfb.sh" timeout 8s "$installed_taskapp"', workflow)
        self.assertIn('test "$taskapp_status" -eq 124', workflow)
        self.assertIn('Mosaic Rust runtime unavailable', workflow)
        self.assertIn("--runtime-library \"$runtime_library\"", workflow)
        self.assertIn("flutter analyze", workflow)
        self.assertIn("-path '*/bundle/mosaic_flutter_runtime_conformance'", workflow)
        self.assertIn("cmp \"$runtime_library\" \"$bundled_runtime\"", workflow)
        self.assertIn("unset MOSAIC_APP_LIBRARY", workflow)
        self.assertIn("dart run bin/mosaic_runtime_conformance.dart", workflow)
        self.assertIn("dart analyze", workflow)
        self.assertIn("mosaic-flutter-native-complete-rating-controls", workflow)
        self.assertIn("--profile native-complete", workflow)
        self.assertIn(".nativeComplete == true", workflow)
        self.assertNotIn("dart analyze lib", workflow)
        self.assertIn("mosaic_taskapp_acceptance", workflow)
        self.assertIn("mosaic-flutter-data-grid", workflow)
        self.assertIn("mosaic_data_grid_acceptance", workflow)
        self.assertIn("mosaic-flutter-toolkit", workflow)
        self.assertIn("code/packages/mosaic/mosaic-pkg-toolkit", workflow)
        self.assertIn(
            "flutter create --platforms=linux "
            "--project-name mosaic_toolkit_acceptance .",
            workflow,
        )
        self.assertIn("flutter test", workflow)
        self.assertIn("flutter build linux --debug", workflow)
        self.assertIn("MOSAIC_APP_LIBRARY", workflow)
        self.assertIn('assign = text.index("_mosaicHost = host;")', workflow)
        self.assertIn(
            'install = text.index("installEngramEffects(host);")', workflow
        )
        self.assertNotIn(
            'assign = text.index("_mosaicHost = widget.mosaicHost")', workflow
        )

    def test_engram_layout_variant_is_compiled_beside_the_default(self) -> None:
        """UI48 §7.9: the Flutter lane checks Engram's touch variant and its
        selector are in lib/ before analyzing and building it."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index(
            "# UI48 §7.9 (ENV2/ENV3): Engram is the one package with a layout"
        )
        block = workflow[start : workflow.index("\n\n", start)]
        self.assertIn("^class EngramAppTouch extends StatelessWidget {$", block)
        self.assertIn("then exit 1; fi", block)
        self.assertIn("switch (mosaicLayoutVariant(environment))", block)
        self.assertIn("return EngramAppTouch($", block)
        # The checks run before the project is analyzed and built.
        self.assertLess(
            start, workflow.index("--project-name mosaic_engram_app .")
        )
        self.assertIn("rust/mosaic-package-manifest", MODULE.ACCEPTANCE_PACKAGES)

    def test_layout_variants_fixture_is_resized_in_the_lane(self) -> None:
        """UI48 §7 wants a gate that changes the environment: the lane runs
        the layout-variants fixture's widget test, which resizes the window
        across the compact threshold and asserts the root swaps."""

        workflow = WORKFLOW.read_text(encoding="utf-8")
        start = workflow.index("# UI48 §7.9 (ENV2/ENV3): the resize gate")
        block = workflow[start : workflow.index("\n\n", start)]
        self.assertIn("fixtures/layout-variants --backend flutter", block)
        self.assertIn("fixtures/layout-variants-test/widget_test.dart", block)
        self.assertIn("flutter analyze", block)
        self.assertIn("flutter test test/widget_test.dart", block)
        test = (
            Path(__file__).resolve().parents[2]
            / "packages"
            / "rust"
            / "mosaic-emit-flutter"
            / "fixtures"
            / "layout-variants-test"
            / "widget_test.dart"
        ).read_text(encoding="utf-8")
        self.assertIn("tester.view.physicalSize = const Size(400, 800);", test)
        self.assertIn("find.byType(LayoutProbeCompact), findsOneWidget", test)

    def test_workflow_runs_the_platform_library_harness(self) -> None:
        """UI87 §7.7: the lane checks TaskApp installs the platform library on
        its started host, then runs the headless harness against TaskApp's
        generated core and host -- after the TaskApp project was built, so the
        files it copies exist."""
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            'install = text.index("installMosaicPlatformEffects(host, appKinds: null);")',
            workflow,
        )
        self.assertIn(
            "cp -R code/packages/rust/mosaic-app-bindings/conformance/flutter-platform-effects",
            workflow,
        )
        self.assertIn(
            '"$taskapp_output/flutter/lib/mosaic_platform_effects_core.dart"', workflow
        )
        self.assertIn("dart analyze --fatal-infos", workflow)
        self.assertIn(
            'grep -F "Mosaic Flutter platform effects conformance passed"', workflow
        )
        taskapp_build = workflow.index(
            "--project-name mosaic_taskapp_acceptance ."
        )
        harness = workflow.index("conformance/flutter-platform-effects")
        self.assertLess(taskapp_build, harness)

    def test_platform_harness_does_not_duplicate_the_generated_library(self) -> None:
        harness = FLUTTER_CONFORMANCE.parent / "flutter-platform-effects"
        self.assertTrue((harness / "bin" / "conformance.dart").is_file())
        self.assertFalse((harness / "lib").exists())
        pubspec = (harness / "pubspec.yaml").read_text(encoding="utf-8")
        self.assertIn("ffi:", pubspec)
        self.assertNotIn("sdk: flutter", pubspec)
        self.assertNotIn("file_selector", pubspec)

    def test_harness_does_not_duplicate_the_generated_binding(self) -> None:
        self.assertTrue(
            (FLUTTER_CONFORMANCE / "bin" / "conformance.dart").is_file()
        )
        self.assertFalse((FLUTTER_CONFORMANCE / "lib" / "mosaic_host.dart").exists())
        pubspec = (FLUTTER_CONFORMANCE / "pubspec.yaml").read_text(encoding="utf-8")
        self.assertIn("ffi:", pubspec)
        self.assertNotIn("sdk: flutter", pubspec)


if __name__ == "__main__":
    unittest.main()
