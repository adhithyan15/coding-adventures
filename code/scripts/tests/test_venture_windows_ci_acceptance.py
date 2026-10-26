from __future__ import annotations

import importlib.util
import re
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

    def test_cairo_bridge_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_venture_windows(
                {"affected_packages": ["rust/venture-browser-cairo"]}
            )
        )

    def test_qt_bridge_requires_acceptance(self) -> None:
        self.assertTrue(
            MODULE.requires_venture_windows(
                {"affected_packages": ["rust/venture-browser-qt"]}
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
                {"affected_packages": ["mosaic/programs/venture-browser"]}
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
            "(needs.detect.outputs.needs_venture_windows == 'true' || "
            "needs.detect.outputs.needs_mosaic_xaml_windows == 'true') "
            "&& runner.os == 'Windows')",
            workflow,
        )
        self.assertIn(
            "needs.detect.outputs.needs_rust == 'true' || "
            "needs.detect.outputs.needs_venture_windows == 'true'",
            workflow,
        )
        self.assertIn(
            "(needs.detect.outputs.needs_rust == 'true' || "
            "needs.detect.outputs.needs_venture_windows == 'true' || "
            "needs.detect.outputs.needs_mosaic_qt_runtime == 'true') "
            "&& runner.os == 'Linux'",
            workflow,
        )
        self.assertEqual(
            workflow.count(
                "needs.detect.outputs.needs_java == 'true' || "
                "needs.detect.outputs.needs_kotlin == 'true' || "
                "needs.detect.outputs.needs_venture_windows == 'true'"
            ),
            3,
        )
        self.assertIn(
            "cargo test -p paint-vm-cairo -p venture-browser-cairo -p venture-browser-qt "
            "-p venture-browser-windows",
            workflow,
        )
        self.assertIn(
            "Set-Location ..\\..\\programs\\mosaic\\venture-browser",
            workflow,
        )
        self.assertIn(
            "powershell -NoProfile -ExecutionPolicy Bypass -File "
            "scripts\\build-all.ps1",
            workflow,
        )
        self.assertIn("cargo test -p venture-browser-macos", workflow)

    def test_required_gate_is_exclusive_to_pull_request_runs(self) -> None:
        workflow = WORKFLOW.read_text(encoding="utf-8")
        self.assertIn(
            "name: ${{ github.event_name == 'pull_request' && 'CI gate' "
            "|| 'CI push gate' }}",
            workflow,
        )
        self.assertNotIn("\n    name: CI gate\n", workflow)

    def test_venture_only_plan_admits_node_and_pinned_npm(self) -> None:
        # Scope executable step fields to the build job. A comment mentioning
        # Venture, or another job's unconditional setup, cannot satisfy this.
        workflow = WORKFLOW.read_text(encoding="utf-8")
        job = re.search(r"(?ms)^  build:\n(.*?)(?=^  [\w-]+:|\Z)", workflow)
        self.assertIsNotNone(job)
        assert job is not None
        steps = re.split(r"(?m)^      - ", job.group(1))[1:]
        flags = ("needs_typescript", "needs_venture_windows")
        for name in ("Set up Node.js", "Set up npm"):
            with self.subTest(step=name):
                matching = [s for s in steps if s.splitlines()[0] == f"name: {name}"]
                self.assertEqual(len(matching), 1)
                condition = re.search(r"(?m)^        if: (.+)$", matching[0])
                self.assertIsNotNone(condition)
                assert condition is not None
                # These two toolchains have a flat OR admission contract. Parse
                # only the allowed comparisons, never evaluate workflow text.
                terms = condition.group(1).split(" || ")
                admitted_flags = []
                for term in terms:
                    comparison = re.fullmatch(
                        r"needs\.detect\.outputs\.(needs_[a-z_]+) == 'true'", term
                    )
                    self.assertIsNotNone(comparison)
                    assert comparison is not None
                    admitted_flags.append(comparison.group(1))
                for typescript, venture, expected in (
                    (False, False, False),
                    (True, False, True),
                    (False, True, True),
                    (True, True, True),
                ):
                    values = dict(zip(flags, (typescript, venture)))
                    self.assertEqual(
                        any(values.get(flag, False) for flag in admitted_flags),
                        expected,
                        (name, values),
                    )
                if name == "Set up npm":
                    self.assertRegex(
                        matching[0],
                        r"(?m)^        run: npm install --global npm@11\.13\.0$",
                    )


if __name__ == "__main__":
    unittest.main()
