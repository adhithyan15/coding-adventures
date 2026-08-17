from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "mosaic_compose_runtime_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
COMPOSE_CONFORMANCE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-bindings"
    / "conformance"
    / "compose"
)
COMPOSE_PACKAGE = (
    Path(__file__).resolve().parents[2]
    / "packages"
    / "rust"
    / "mosaic-app-conformance"
    / "package"
)
SPEC = importlib.util.spec_from_file_location(
    "mosaic_compose_runtime_ci_acceptance", SCRIPT
)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class MosaicComposeRuntimeCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime({"affected_packages": None})
        )

    def test_compose_emitter_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["rust/mosaic-emit-compose"]}
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
                    MODULE.requires_mosaic_compose_runtime(
                        {"affected_packages": [package]}
                    )
                )

    def test_task_app_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["mosaic/programs/task-app"]}
            )
        )

    def test_standard_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["mosaic/mosaic-pkg-grid"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": ["rust/html-parser"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_mosaic_compose_runtime(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_mosaic_compose_runtime({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/mosaic-emit-compose"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_rust_jvm_and_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "needs_mosaic_compose_runtime: "
            "${{ steps.mosaic-compose-runtime.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/mosaic_compose_runtime_ci_acceptance.py",
            workflow,
        )
        self.assertIn("Round-trip Rust engine through standard Compose binding", workflow)
        self.assertIn("needs_mosaic_compose_runtime == 'true'", workflow)
        self.assertIn("timeout-minutes: 20", workflow)
        self.assertIn(
            "--backend compose --output \"$taskapp_output\" --emit-project",
            workflow,
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p mosaic-app-conformance",
            workflow,
        )
        self.assertIn(
            "$bundled_output/compose/src/main/kotlin/MosaicRuntimeHost.kt", workflow
        )
        self.assertIn(
            "cargo build --manifest-path code/packages/rust/Cargo.toml -p task-mosaic-app",
            workflow,
        )
        self.assertIn("libtask_mosaic_app.so", workflow)
        self.assertIn(
            'cmp "$task_runtime_library" "$installed_taskapp_runtime"', workflow
        )
        self.assertIn("*/bin/task_app", workflow)
        self.assertIn('xvfb-run -a timeout 8s "$installed_taskapp"', workflow)
        self.assertIn('test "$taskapp_status" -eq 124', workflow)
        self.assertIn("Mosaic Rust runtime unavailable", workflow)
        self.assertIn("--runtime-library \"$runtime_library\"", workflow)
        self.assertIn("compileKotlin createDistributable", workflow)
        self.assertIn("*/resources/libmosaic_app.so", workflow)
        self.assertIn("cmp \"$runtime_library\" \"$installed_runtime\"", workflow)
        self.assertIn("unset MOSAIC_APP_LIBRARY", workflow)
        self.assertIn("-Dcompose.application.resources.dir", workflow)
        self.assertIn("mosaic-pkg-rating-controls", workflow)
        self.assertIn("--profile native-complete", workflow)
        self.assertIn(
            "gradle --no-daemon --stacktrace -p \"$strict_output/compose\" compileKotlin",
            workflow,
        )

    def test_harness_does_not_duplicate_the_generated_binding(self) -> None:
        source = COMPOSE_CONFORMANCE / "src" / "main" / "kotlin"
        self.assertTrue((source / "Conformance.kt").is_file())
        self.assertFalse((source / "MosaicRuntimeHost.kt").exists())
        gradle = (COMPOSE_CONFORMANCE / "build.gradle.kts").read_text(encoding="utf-8")
        self.assertIn("net.java.dev.jna:jna:5.19.1", gradle)
        self.assertIn("kotlinx-serialization-json:1.11.0", gradle)

    def test_conformance_engine_has_a_real_mosaic_package(self) -> None:
        self.assertTrue((COMPOSE_PACKAGE / "mosaic-package.toml").is_file())
        for suffix in ("mil", "mll", "msl"):
            self.assertTrue((COMPOSE_PACKAGE / "src" / f"Counter.{suffix}").is_file())


if __name__ == "__main__":
    unittest.main()
