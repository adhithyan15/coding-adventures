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


class ClangClSelectionTests(unittest.TestCase):
    def test_parses_clang_version_output(self) -> None:
        self.assertEqual(
            (22, 1, 0),
            msvc.parse_clang_version("clang version 22.1.0\nTarget: x86_64-pc-windows-msvc"),
        )
        self.assertEqual((21, 1, 0), msvc.parse_clang_version("clang version 21.1"))
        self.assertIsNone(msvc.parse_clang_version("no version here"))

    def test_promotes_newer_vs_clang_cl_ahead_of_older_standalone_llvm(self) -> None:
        env = {
            "Path": r"C:\VC\bin;C:\Program Files\LLVM\bin;C:\VS\VC\Tools\Llvm\x64\bin;C:\Windows",
            "llvmX64": r"C:\VS\VC\Tools\Llvm\x64\bin",
        }
        versions = {
            r"C:\Program Files\LLVM\bin": (21, 1, 8),
            r"C:\VS\VC\Tools\Llvm\x64\bin": (22, 1, 0),
        }
        updated, candidates, selected = msvc.prefer_newest_clang_cl(
            env,
            "x64",
            has_clang_cl=lambda d: d in versions,
            probe_version=lambda d: versions.get(d),
        )
        self.assertEqual(
            r"C:\VC\bin;C:\VS\VC\Tools\Llvm\x64\bin;C:\Program Files\LLVM\bin;C:\Windows",
            updated["Path"],
        )
        self.assertEqual(2, len(candidates))
        self.assertEqual((r"C:\VS\VC\Tools\Llvm\x64\bin", (22, 1, 0)), selected)

    def test_keeps_path_unchanged_when_first_clang_cl_is_already_newest(self) -> None:
        env = {
            "Path": r"C:\VC\bin;C:\Program Files\LLVM\bin;C:\VS\VC\Tools\Llvm\x64\bin",
        }
        versions = {
            r"C:\Program Files\LLVM\bin": (22, 1, 0),
            r"C:\VS\VC\Tools\Llvm\x64\bin": (21, 1, 8),
        }
        updated, _, selected = msvc.prefer_newest_clang_cl(
            env,
            "x64",
            has_clang_cl=lambda d: d in versions,
            probe_version=lambda d: versions.get(d),
        )
        self.assertEqual(env["Path"], updated["Path"])
        self.assertEqual((r"C:\Program Files\LLVM\bin", (22, 1, 0)), selected)

    def test_reads_msvc_stl_min_clang_major_from_yvals_core(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            header = Path(directory) / "yvals_core.h"
            header.write_text(
                "#error STL1000: Unexpected compiler version, expected Clang 22 or newer.\n",
                encoding="utf-8",
            )
            self.assertEqual(22, msvc.msvc_stl_min_clang_major({"INCLUDE": directory}))


class WindowsMixedToolchainWorkflowTests(unittest.TestCase):
    def test_swift_paths_are_appended_without_preempting_msvc_or_llvm(self) -> None:
        repository = Path(__file__).resolve().parents[3]
        workflow = (repository / ".github/workflows/ci.yml").read_text(
            encoding="utf-8"
        )
        marker = "      - name: Set up Swift (Windows)\n"
        self.assertIn(marker, workflow)
        swift_step = workflow.split(marker, 1)[1].split("\n      - name:", 1)[0]

        # GITHUB_PATH prepends paths in subsequent steps. Swift's installer
        # therefore updates the existing ordered PATH through GITHUB_ENV,
        # keeping MSVC bin and the selected LLVM bin ahead of Swift.
        self.assertNotIn("$env:GITHUB_PATH", swift_step)
        self.assertIn('$env:Path = "$env:Path;$($newPathEntries -join \';\')"', swift_step)
        self.assertIn('$env:Path = "$env:Path;$toolchainBin"', swift_step)
        self.assertIn('"PATH=$env:Path" | Out-File -FilePath $env:GITHUB_ENV', swift_step)
        self.assertLess(
            swift_step.index('$env:Path = "$env:Path;$($newPathEntries -join \';\')"'),
            swift_step.index('"PATH=$env:Path" | Out-File'),
        )
        self.assertLess(
            swift_step.index('$env:Path = "$env:Path;$toolchainBin"'),
            swift_step.index('"PATH=$env:Path" | Out-File'),
        )

        # The dedicated Swift build, affected package build, and full build
        # all observe the exported PATH. MSVC setup occurs earlier and its
        # precedence is preserved because Swift no longer writes GITHUB_PATH.
        msvc_setup = workflow.index("- name: Set up MSVC (Lua + C/C++ + Venture + LANG)")
        swift_setup = workflow.index(marker)
        swift_build = workflow.index("- name: Build and test affected Swift packages on Windows")
        affected_build = workflow.index("- name: Build and test affected packages")
        full_build = workflow.index("- name: Full build on main merge")
        self.assertLess(msvc_setup, swift_setup)
        self.assertLess(swift_setup, swift_build)
        self.assertLess(swift_setup, affected_build)
        self.assertLess(swift_setup, full_build)


if __name__ == "__main__":
    unittest.main()
