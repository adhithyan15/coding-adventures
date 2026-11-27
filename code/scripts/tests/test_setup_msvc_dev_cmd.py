from __future__ import annotations

import sys
import tempfile
import unittest
from pathlib import Path
from subprocess import CompletedProcess
from unittest.mock import patch

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import setup_msvc_dev_cmd as msvc


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

    def test_appends_one_path_entry_for_later_steps(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "github-path"
            msvc.append_github_path(destination, r"C:\VS\VC\Tools\Llvm\x64\bin")
            self.assertEqual(
                "C:\\VS\\VC\\Tools\\Llvm\\x64\\bin\n",
                destination.read_text(encoding="utf-8"),
            )

    def test_rejects_multiline_path_entries(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            destination = Path(directory) / "github-path"
            with self.assertRaises(ValueError):
                msvc.append_github_path(destination, "C:\\valid\n::warning::injected")
            self.assertFalse(destination.exists())


class ClangClSelectionTests(unittest.TestCase):
    def test_selected_executable_is_absolute_and_preserves_spaces(self) -> None:
        with tempfile.TemporaryDirectory(prefix="Visual Studio LLVM ") as directory:
            selected = msvc.selected_clang_cl_executable((directory, (22, 1, 3)))
            self.assertEqual(str((Path(directory) / "clang-cl.exe").resolve()), selected)
            with tempfile.TemporaryDirectory() as output_directory:
                destination = Path(output_directory) / "github-env"
                msvc.append_github_environment(destination, {"ISO_CLANG_CL": selected})
                self.assertEqual(f"ISO_CLANG_CL={selected}\n", destination.read_text())
        self.assertIsNone(msvc.selected_clang_cl_executable(None))

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

    def test_selects_compatible_vcvars_clang_after_swift_path_contribution(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            header = Path(directory) / "yvals_core.h"
            header.write_text(
                "#error STL1000: Unexpected compiler version, expected Clang 22 or newer.\n",
                encoding="utf-8",
            )
            vs_clang = r"C:\VS\VC\Tools\Llvm\x64\bin"
            environment = {
                "PATH": r"C:\VS\VC\bin;C:\Swift\Toolchain\usr\bin",
                "llvmX64": vs_clang,
                "INCLUDE": directory,
            }
            selected = msvc.select_compatible_vs_clang_cl(
                environment,
                "x64",
                has_clang_cl=lambda value: value == vs_clang,
                probe_version=lambda value: (22, 1, 3) if value == vs_clang else None,
            )
            self.assertEqual((vs_clang, (22, 1, 3), 22), selected)

    def test_rejects_vcvars_clang_older_than_the_active_msvc_stl(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            header = Path(directory) / "yvals_core.h"
            header.write_text(
                "#error STL1000: Unexpected compiler version, expected Clang 22 or newer.\n",
                encoding="utf-8",
            )
            with self.assertRaisesRegex(RuntimeError, "older than MSVC STL minimum"):
                msvc.select_compatible_vs_clang_cl(
                    {"llvmX64": r"C:\VS\LLVM", "INCLUDE": directory},
                    "x64",
                    has_clang_cl=lambda _: True,
                    probe_version=lambda _: (21, 1, 3),
                )


if __name__ == "__main__":
    unittest.main()
