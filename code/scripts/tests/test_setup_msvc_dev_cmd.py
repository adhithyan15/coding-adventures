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

    @patch("setup_msvc_dev_cmd.subprocess.run")
    def test_invokes_vcvarsall_directly_without_call_or_cmd_s(self, run) -> None:
        run.return_value = CompletedProcess(
            args=[], returncode=0, stdout="Path=C:\\VC\\bin\r\n", stderr=""
        )

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
                'set >nul && "C:\\Program Files\\Microsoft Visual Studio\\18\\VC\\vcvarsall.bat" x64 >nul && set',
            ],
            command,
        )
        self.assertNotIn("call ", command[-1].casefold())


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
