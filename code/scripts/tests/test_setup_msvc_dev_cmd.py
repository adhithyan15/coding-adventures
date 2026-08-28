from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from subprocess import CompletedProcess
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import setup_msvc_dev_cmd as msvc  # noqa: E402


class EnvironmentParsingTests(unittest.TestCase):
    def test_parses_values_with_equals_and_ignores_cmd_fluff(self) -> None:
        self.assertEqual(
            {
                "INCLUDE": r"C:\SDK\include;C:\VC\include",
                "VSCMD_ARG_TGT_ARCH": "x64",
                "VALUE": "left=right",
            },
            msvc.parse_environment(
                [
                    "not an environment line",
                    r"=C:=C:\work",
                    r"INCLUDE=C:\SDK\include;C:\VC\include",
                    "VSCMD_ARG_TGT_ARCH=x64",
                    "VALUE=left=right",
                    "bad name=value",
                ]
            ),
        )

    def test_exports_only_changes_and_deduplicates_path_variables(self) -> None:
        changes = msvc.changed_environment(
            {"Path": r"C:\Windows;C:\Tools", "UNCHANGED": "same"},
            {
                "PATH": r"C:\VC\bin;C:\Windows;c:\vc\BIN;C:\Tools",
                "UNCHANGED": "same",
                "VCINSTALLDIR": r"C:\Visual Studio\VC",
            },
        )

        self.assertEqual(
            {
                "PATH": r"C:\VC\bin;C:\Windows;C:\Tools",
                "VCINSTALLDIR": r"C:\Visual Studio\VC",
            },
            changes,
        )

    def test_builds_batch_wrapper_that_returns_after_vcvarsall(self) -> None:
        self.assertEqual(
            "@echo off\r\n"
            'call "C:\\Program Files\\Microsoft Visual Studio\\18\\VC\\vcvarsall.bat" x64\r\n'
            "if errorlevel 1 exit /b %errorlevel%\r\n"
            "set\r\n",
            msvc.developer_command_script(
                Path(r"C:\Program Files\Microsoft Visual Studio\18\VC\vcvarsall.bat"),
                "x64",
            ),
        )

    @patch("setup_msvc_dev_cmd.subprocess.run")
    def test_invokes_temporary_wrapper_without_cmd_quote_ambiguity(self, run) -> None:
        wrapper_paths: list[Path] = []

        def execute(command, **kwargs):
            wrapper_path = Path(kwargs["cwd"]) / command[-1]
            wrapper_paths.append(wrapper_path)
            self.assertEqual(
                "@echo off\r\n"
                'call "C:\\Program Files\\Microsoft Visual Studio\\18\\VC\\vcvarsall.bat" x64\r\n'
                "if errorlevel 1 exit /b %errorlevel%\r\n"
                "set\r\n",
                wrapper_path.read_bytes().decode("utf-8"),
            )
            return CompletedProcess(
                args=command, returncode=0, stdout="Path=C:\\VC\\bin\r\n", stderr=""
            )

        run.side_effect = execute

        environment = msvc.capture_developer_environment(
            Path(r"C:\Program Files\Microsoft Visual Studio\18\VC\vcvarsall.bat"),
            "x64",
            comspec=Path(r"C:\Windows\System32\cmd.exe"),
        )

        self.assertEqual({"Path": r"C:\VC\bin"}, environment)
        command = run.call_args.args[0]
        self.assertEqual(
            [
                r"C:\Windows\System32\cmd.exe",
                "/d",
                "/c",
                wrapper_paths[0].name,
            ],
            command,
        )
        self.assertFalse(wrapper_paths[0].exists())


class GitHubEnvironmentFileTests(unittest.TestCase):
    def test_writes_single_and_multiline_values(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "github-env"
            msvc.append_github_environment(
                destination,
                {"PATH": r"C:\VC\bin;C:\Windows", "MULTI": "one\ntwo"},
            )

            self.assertEqual(
                "PATH=C:\\VC\\bin;C:\\Windows\n"
                "MULTI<<MSVC_ENV_1\n"
                "one\n"
                "two\n"
                "MSVC_ENV_1\n",
                destination.read_text(encoding="utf-8"),
            )


if __name__ == "__main__":
    unittest.main()
