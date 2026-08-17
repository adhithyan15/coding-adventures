from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_swift_runtime_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
SWIFT_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "swiftui"
)
SPEC = importlib.util.spec_from_file_location("mosaic_swift_runtime_ci_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicSwiftRuntimeCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime({"affected_packages": None})
        )

    def test_swift_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["rust/mosaic-emit-swiftui"]}
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
                    MODULE.requires_mosaic_swift_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["mosaic/programs/task-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_swift_runtime(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_swift_runtime({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-swiftui"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_rust_and_swift_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        swift_runtime_step = workflow.split(
            "- name: Round-trip Rust engine through standard SwiftUI binding", 1
        )[1].split(
            "- name: Round-trip Rust engine through standard Qt binding", 1
        )[0]
        self.assertIn(
            "needs_mosaic_swift_runtime: "
            "${{ steps.mosaic-swift-runtime.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_swift_runtime_ci_acceptance.py",
            workflow,
        )
        self.assertIn(
            "needs_mosaic_swift_runtime == 'true'", workflow
        )
        self.assertIn(
            "Round-trip Rust engine through standard SwiftUI binding", workflow
        )
        self.assertIn("timeout-minutes: 15", swift_runtime_step)
        self.assertIn(
            "mosaic-compile/Cargo.toml -- pkg code/programs/mosaic/task-app",
            workflow,
        )
        self.assertIn(
            "--backend swiftui --output \"$taskapp_output\" --emit-project --profile native-complete --runtime-library \"$task_runtime_library\"",
            swift_runtime_step,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance",
            workflow,
        )
        self.assertIn("mosaic-app-conformance/package", workflow)
        self.assertIn('--runtime-library "$runtime_library"', workflow)
        self.assertIn(
            "find \"$swift_bin\" -type f -path '*/Runtime/libmosaic_app.dylib'",
            workflow,
        )
        self.assertIn('cmp "$runtime_library" "$installed_runtime"', workflow)
        self.assertIn("Sources/App/MosaicRuntimeHost.swift", workflow)
        self.assertIn("Sources/CMosaicRuntime/CMosaicRuntime.c", workflow)
        self.assertIn(
            'env -u MOSAIC_APP_LIBRARY swift run --package-path "$harness" '
            'Conformance --library "$installed_runtime"',
            workflow,
        )
        self.assertIn("mosaic-swift-bundled-conformance", workflow)
        self.assertIn(
            '--backend swiftui --output "$bundled_output" '
            '--emit-project --profile native-complete --runtime-library "$runtime_library"',
            workflow,
        )
        self.assertIn(
            ".nativeComplete == true and (.degradations | length == 0)",
            workflow,
        )
        self.assertIn(
            'swift build --package-path "$bundled_output/swiftui"', workflow
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app",
            swift_runtime_step,
        )
        self.assertIn("libtask_mosaic_app.dylib", swift_runtime_step)
        self.assertIn(
            'cmp "$task_runtime_library" "$installed_taskapp_runtime"',
            swift_runtime_step,
        )
        self.assertIn('installed_taskapp="$taskapp_bin/App"', swift_runtime_step)
        self.assertIn(
            'env -u MOSAIC_APP_LIBRARY "$installed_taskapp"', swift_runtime_step
        )
        self.assertIn('kill -0 "$taskapp_pid"', swift_runtime_step)
        self.assertIn("Mosaic Rust runtime unavailable", swift_runtime_step)
        self.assertIn("missing required MIL prop", swift_runtime_step)
        self.assertIn(
            '--backend swiftui --output "$ios_output" --emit-project',
            swift_runtime_step,
        )
        self.assertIn("MOSAIC_APP_LIBRARY", swift_runtime_step)

    def test_harness_does_not_duplicate_the_generated_binding(self) -> None:
        program = SWIFT_CONFORMANCE / "Sources" / "Conformance" / "Program.swift"
        package = (SWIFT_CONFORMANCE / "Package.swift").read_text(encoding="utf-8")
        self.assertTrue(program.is_file())
        self.assertFalse(
            (
                SWIFT_CONFORMANCE
                / "Sources"
                / "Conformance"
                / "MosaicRuntimeHost.swift"
            ).exists()
        )
        self.assertIn('name: "CMosaicRuntime"', package)
        self.assertIn('name: "Conformance"', package)


if __name__ == "__main__":
    unittest.main()
