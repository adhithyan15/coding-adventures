from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_qt_runtime_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
QT_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "qt"
)
QT_PACKAGE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-conformance"
    / "package"
)
SPEC = importlib.util.spec_from_file_location("mosaic_qt_runtime_ci_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicQtRuntimeCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(MODULE.requires_mosaic_qt_runtime({"affected_packages": None}))

    def test_qt_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_qt_runtime(
                {"affected_packages": ["rust/mosaic-emit-qt"]}
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
                    MODULE.requires_mosaic_qt_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_qt_runtime(
                {"affected_packages": ["mosaic/programs/task-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_qt_runtime(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_qt_runtime(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_qt_runtime(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_qt_runtime({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-qt"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_rust_qt_and_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        qt_runtime_step = workflow.split(
            "- name: Round-trip Rust engine through standard Qt binding", 1
        )[1].split(
            "- name: Round-trip Rust engine through standard Flutter binding", 1
        )[0]
        self.assertIn(
            "needs_mosaic_qt_runtime: "
            "${{ steps.mosaic-qt-runtime.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_qt_runtime_ci_acceptance.py",
            workflow,
        )
        self.assertIn("Round-trip Rust engine through standard Qt binding", workflow)
        self.assertIn("needs_mosaic_qt_runtime == 'true'", workflow)
        self.assertIn("timeout-minutes: 15", workflow)
        self.assertIn(
            "--backend qt --output \"$bundled_output\" --emit-project --profile native-complete",
            workflow,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance",
            workflow,
        )
        self.assertIn("qt/MosaicHost.cpp", workflow)
        self.assertIn("cmake --build \"$harness/build\"", workflow)
        self.assertIn("--runtime-library \"$runtime_library\"", workflow)
        self.assertIn("cmake --install \"$bundled_output/qt/build\"", workflow)
        self.assertIn("find \"$bundled_output/install\"", workflow)
        self.assertIn("cmp \"$runtime_library\" \"$installed_runtime\"", workflow)
        self.assertIn("env -u MOSAIC_APP_LIBRARY", workflow)
        self.assertIn("installed_harness", workflow)
        self.assertIn("--expect-missing-prop-failure", workflow)
        self.assertIn("--expect-required-failure", workflow)
        self.assertIn("mosaic-qt-taskapp", workflow)
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app",
            workflow,
        )
        self.assertIn(
            "pkg code/programs/mosaic/task-app --backend qt", workflow
        )
        self.assertNotIn('"accessibility.table-semantics-missing"', qt_runtime_step)
        self.assertIn(
            "pkg code/programs/mosaic/task-app --backend qt --output \"$taskapp_output\" --emit-project --profile native-complete --runtime-library \"$task_runtime_library\"",
            workflow,
        )
        self.assertIn(
            "'.nativeComplete == true and (.degradations | length == 0)' \"$taskapp_output/qt/mosaic-degradations.json\"",
            workflow,
        )
        self.assertNotIn('"runtime.sample-fallback"', qt_runtime_step)
        self.assertIn('cmake --build "$taskapp_output/qt/build"', workflow)
        self.assertIn('QT_QPA_PLATFORM=offscreen timeout 5s "$installed_taskapp"', workflow)
        self.assertIn(
            '! grep -E "missing required MIL prop|ReferenceError|TypeError" "$taskapp_log"',
            workflow,
        )
        self.assertIn("mosaic-qt-toolkit", workflow)
        self.assertIn(
            "pkg code/packages/mosaic/mosaic-pkg-toolkit --backend qt",
            workflow,
        )
        self.assertIn(
            'cmake --build "$toolkit_output/qt/build"',
            workflow,
        )
        self.assertIn(
            "--backend qt --output \"$strict_output\" --emit-project --profile native-complete",
            workflow,
        )
        self.assertIn("mosaic-qt-native-complete-rating-controls", workflow)
        self.assertIn(
            "native-complete requires the Mosaic Rust application runtime", workflow
        )

    def test_harness_does_not_duplicate_the_generated_binding(self) -> None:
        self.assertTrue((QT_CONFORMANCE / "main.cpp").is_file())
        self.assertFalse((QT_CONFORMANCE / "MosaicHost.cpp").exists())
        self.assertFalse((QT_CONFORMANCE / "MosaicHost.h").exists())
        cmake = (QT_CONFORMANCE / "CMakeLists.txt").read_text(encoding="utf-8")
        self.assertIn("Qt6::Core", cmake)
        self.assertIn("CMAKE_AUTOMOC", cmake)

    def test_conformance_engine_has_a_real_mosaic_package(self) -> None:
        self.assertTrue((QT_PACKAGE / "mosaic-package.toml").is_file())
        for suffix in ("mil", "mll", "msl"):
            self.assertTrue((QT_PACKAGE / "src" / f"Counter.{suffix}").is_file())


if __name__ == "__main__":
    unittest.main()
