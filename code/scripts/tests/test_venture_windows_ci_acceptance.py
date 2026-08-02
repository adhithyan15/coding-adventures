from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path


SCRIPT = Path(__file__).resolve().parents[1] / "venture_windows_ci_acceptance.py"
WORKFLOW = Path(__file__).resolve().parents[3] / ".github" / "workflows" / "ci.yml"
SPEC = importlib.util.spec_from_file_location("venture_windows_ci_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class VentureWindowsCIAcceptanceTests(unittest.TestCase):
    def test_force_plan_requires_acceptance(self) -> None:
        self.assertTrue(MODULE.requires_venture_windows({"affected_packages": None}))

    def test_windows_bridge_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_venture_windows(
                {"affected_packages": ["rust/venture-browser-windows"]}
            )
        )

    def test_macos_bridge_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_venture_windows(
                {"affected_packages": ["rust/venture-browser-macos"]}
            )
        )

    def test_mosaic_package_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_venture_windows(
                {"affected_packages": ["unknown/programs/venture-browser"]}
            )
        )

    def test_unrelated_plan_skips_acceptance(self) -> None:
        self.assertFalse(
            MODULE.requires_venture_windows(
                {"affected_packages": ["rust/venture-browser-core"]}
            )
        )

    def test_workflow_change_self_tests_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_venture_windows(
                {"affected_packages": []}, workflow_changed=True
            )
        )

    def test_invalid_affected_packages_is_rejected(self) -> None:
        with self.assertRaisesRegex(ValueError, "array or null"):
            MODULE.requires_venture_windows({"affected_packages": "all"})

    def test_cli_emits_github_output_value(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "build-plan.json"
            plan.write_text(
                '{"affected_packages":["rust/venture-browser-windows"]}',
                encoding="utf-8",
            )
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
        self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_routes_required_toolchains_and_acceptance(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "needs_venture_windows: ${{ steps.venture-windows.outputs.required }}",
            workflow,
        )
        self.assertIn(
            "python3 code/scripts/venture_windows_ci_acceptance.py",
            workflow,
        )
        self.assertIn(
            '--diff-base "${{ steps.diff-base.outputs.base }}"',
            workflow,
        )
        self.assertIn(
            "needs.detect.outputs.needs_venture_windows == 'true') "
            "&& runner.os == 'Windows'",
            workflow,
        )
        self.assertIn(
            "needs.detect.outputs.needs_venture_windows == 'true' "
            "&& runner.os == 'Windows')",
            workflow,
        )
        self.assertIn(
            "needs.detect.outputs.needs_rust == 'true' || "
            "needs.detect.outputs.needs_venture_windows == 'true'",
            workflow,
        )
        self.assertIn("cargo test -p venture-browser-windows", workflow)
        self.assertIn("cargo test -p venture-browser-macos", workflow)


if __name__ == "__main__":
    unittest.main()
