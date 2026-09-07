"""Prevent affected Windows runtime tests from becoming silent CI skips."""

from __future__ import annotations

import importlib.util
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SCRIPT = ROOT / "code/scripts/lang_windows_ci_acceptance.py"
SPEC = importlib.util.spec_from_file_location("lang_windows_ci_acceptance", SCRIPT)
assert SPEC is not None and SPEC.loader is not None
MODULE = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(MODULE)


class LangWindowsCIAcceptanceTests(unittest.TestCase):
    def test_full_empty_and_affected_plans(self):
        for plan, expected in (
            ({}, True),
            ({"affected_packages": None}, True),
            ({"affected_packages": []}, False),
            ({"affected_packages": ["rust/html-parser"]}, False),
            ({"affected_packages": ["rust/twig-aot"]}, True),
            ({"affected_packages": ["rust/lang-aot"]}, True),
        ):
            with self.subTest(plan=plan):
                self.assertEqual(MODULE.requires_lang_windows(plan), expected)

    def test_windows_override_is_authoritative(self):
        for windows, expected in (([], False), (["rust/twig-aot"], True), (None, True)):
            with self.subTest(windows=windows):
                self.assertEqual(
                    MODULE.requires_lang_windows(
                        {
                            "affected_packages": ["rust/lang-aot"],
                            "platform_overrides": {
                                "windows": {"affected_packages": windows}
                            },
                        }
                    ),
                    expected,
                )
        self.assertTrue(
            MODULE.requires_lang_windows(
                {
                    "affected_packages": [],
                    "platform_overrides": {"windows": {}},
                }
            )
        )

    def test_malformed_plans_fail_even_for_gate_changes(self):
        for plan in (
            {"platform_overrides": []},
            {"platform_overrides": {"windows": []}},
            {"affected_packages": "rust/twig-aot"},
            {"affected_packages": [42]},
        ):
            with self.subTest(plan=plan), self.assertRaises(ValueError):
                MODULE.requires_lang_windows(plan, gate_changed=True)

    def test_gate_edits_select_an_empty_plan(self):
        self.assertTrue(
            MODULE.requires_lang_windows({"affected_packages": []}, gate_changed=True)
        )
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)

            def git(*args):
                return subprocess.run(
                    ["git", *args], cwd=root, check=True, capture_output=True
                )

            git("init")
            git(
                "-c",
                "user.name=Test",
                "-c",
                "user.email=test@example.invalid",
                "commit",
                "--allow-empty",
                "-m",
                "base",
            )
            base = git("rev-parse", "HEAD").stdout.decode().strip()
            self.assertFalse(MODULE.gate_changed(root, base))
            for name in MODULE.GATE_PATHS:
                path = root / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("gate edit\n", encoding="utf-8")
                git("add", name)
                git(
                    "-c",
                    "user.name=Test",
                    "-c",
                    "user.email=test@example.invalid",
                    "commit",
                    "-m",
                    "gate edit",
                )
                self.assertTrue(MODULE.gate_changed(root, "HEAD^"))
            with self.assertRaises(RuntimeError):
                MODULE.gate_changed(root, "missing-ref")

    def test_cli_output(self):
        with tempfile.TemporaryDirectory() as directory:
            plan = Path(directory) / "plan.json"
            plan.write_text('{"affected_packages":["rust/twig-aot"]}', encoding="utf-8")
            result = subprocess.run(
                [sys.executable, str(SCRIPT), str(plan)],
                check=True,
                capture_output=True,
                text=True,
            )
            self.assertEqual(result.stdout, "required=true\n")

    def test_workflow_wires_required_execution_and_toolchains(self):
        workflow = (ROOT / ".github/workflows/ci.yml").read_text(encoding="utf-8")
        self.assertIn(
            "needs_lang_windows: ${{ steps.lang-windows.outputs.required }}", workflow
        )
        self.assertIn(
            "NEEDS_LANG_WINDOWS: ${{ steps.lang-windows.outputs.required }}", workflow
        )
        self.assertIn('or flag("NEEDS_LANG_WINDOWS")', workflow)
        build = workflow.split("\n  build:\n", 1)[1]
        for name in (
            "Set up MSVC",
            "Set up Rust",
            "Test LANG Windows native execution",
        ):
            step = next(
                section
                for section in build.split("      - name: ")
                if section.startswith(name)
            )
            self.assertIn("needs.detect.outputs.needs_lang_windows == 'true'", step)
        step = workflow.split("      - name: Test LANG Windows native execution\n")[
            1
        ].split("      - name:")[0]
        self.assertIn("runner.os == 'Windows'", step)
        self.assertIn('LANG_REQUIRE_WINDOWS_AOT: "1"', step)
        self.assertIn(
            "cargo test -p twig-aot --test windows_x86_64_smoke -- --nocapture", step
        )
        self.assertIn("$LASTEXITCODE", step)


if __name__ == "__main__":
    unittest.main()
