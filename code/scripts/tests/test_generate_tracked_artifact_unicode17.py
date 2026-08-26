from __future__ import annotations

import ctypes
import hashlib
import json
import os
import signal
import subprocess
import sys
import tempfile
import time
import unittest
import urllib.request
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import generate_tracked_artifact_unicode17 as generator


def _test_process_is_running(process_id: int) -> bool:
    if os.name != "nt":
        try:
            os.kill(process_id, 0)
        except OSError:
            return False
        return True
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.argtypes = (ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong)
    kernel32.OpenProcess.restype = ctypes.c_void_p
    kernel32.WaitForSingleObject.argtypes = (ctypes.c_void_p, ctypes.c_ulong)
    kernel32.WaitForSingleObject.restype = ctypes.c_ulong
    kernel32.CloseHandle.argtypes = (ctypes.c_void_p,)
    kernel32.CloseHandle.restype = ctypes.c_int
    handle = kernel32.OpenProcess(0x00100000, False, process_id)
    if not handle:
        return False
    try:
        return kernel32.WaitForSingleObject(handle, 0) == 0x00000102
    finally:
        kernel32.CloseHandle(handle)


def _terminate_test_process(process_id: int) -> None:
    if os.name != "nt":
        os.kill(process_id, signal.SIGTERM)
        return
    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    kernel32.OpenProcess.argtypes = (ctypes.c_ulong, ctypes.c_int, ctypes.c_ulong)
    kernel32.OpenProcess.restype = ctypes.c_void_p
    kernel32.TerminateProcess.argtypes = (ctypes.c_void_p, ctypes.c_uint)
    kernel32.TerminateProcess.restype = ctypes.c_int
    kernel32.WaitForSingleObject.argtypes = (ctypes.c_void_p, ctypes.c_ulong)
    kernel32.WaitForSingleObject.restype = ctypes.c_ulong
    kernel32.CloseHandle.argtypes = (ctypes.c_void_p,)
    kernel32.CloseHandle.restype = ctypes.c_int
    handle = kernel32.OpenProcess(0x0001 | 0x00100000, False, process_id)
    if not handle:
        return
    try:
        kernel32.TerminateProcess(handle, 1)
        kernel32.WaitForSingleObject(handle, 5000)
    finally:
        kernel32.CloseHandle(handle)


def _wait_for_test_process_exit(process_id: int, timeout: float = 5) -> bool:
    deadline = time.monotonic() + timeout
    while _test_process_is_running(process_id):
        if time.monotonic() >= deadline:
            return False
        time.sleep(0.01)
    return True


class _Response:
    def __init__(self, url: str, payload: bytes) -> None:
        self.url = url
        self.payload = payload
        self.read_limit: int | None = None

    def __enter__(self):
        return self

    def __exit__(self, exc_type, exc_value, traceback) -> None:
        return None

    def geturl(self) -> str:
        return self.url

    def read(self, limit: int) -> bytes:
        self.read_limit = limit
        return self.payload[:limit]


class UnicodeDownloadBoundaryTests(unittest.TestCase):
    def test_runtime_self_check_selection_defaults_to_every_emitted_runtime(
        self,
    ) -> None:
        self.assertEqual(
            generator._selected_runtime_self_checks(None),
            ("typescript", "ruby", "elixir", "lua", "perl", "haskell", "swift"),
        )

    def test_runtime_self_check_selection_can_isolate_elixir_for_ci(self) -> None:
        self.assertEqual(
            generator._selected_runtime_self_checks(["elixir"]),
            ("elixir",),
        )

    def test_download_requires_exact_origin_url_size_and_digest(self) -> None:
        url = "https://www.unicode.org/test.txt"
        payload = b"abc"
        response = _Response(url, payload)
        with mock.patch.object(generator._HTTPS_OPENER, "open", return_value=response):
            actual = generator._download_exact(
                url,
                expected_size=len(payload),
                expected_hash=hashlib.sha256(payload).hexdigest(),
                label="test",
            )

        self.assertEqual(actual, payload)
        self.assertEqual(response.read_limit, len(payload) + 1)

    def test_download_rejects_final_url_drift(self) -> None:
        url = "https://www.unicode.org/test.txt"
        response = _Response("https://internal.example/test.txt", b"abc")
        with (
            mock.patch.object(generator._HTTPS_OPENER, "open", return_value=response),
            self.assertRaisesRegex(RuntimeError, "final URL drift"),
        ):
            generator._download_exact(
                url,
                expected_size=3,
                expected_hash=hashlib.sha256(b"abc").hexdigest(),
                label="test",
            )

    def test_download_rejects_lookalike_origin_before_open(self) -> None:
        with (
            mock.patch.object(generator._HTTPS_OPENER, "open") as open_mock,
            self.assertRaisesRegex(RuntimeError, "left the pinned HTTPS origin"),
        ):
            generator._download_exact(
                "https://www.unicode.org.evil.example/test.txt",
                expected_size=3,
                expected_hash=hashlib.sha256(b"abc").hexdigest(),
                label="test",
            )
        open_mock.assert_not_called()

    def test_redirect_handler_fails_closed(self) -> None:
        request = urllib.request.Request("https://www.unicode.org/test.txt")
        with self.assertRaisesRegex(RuntimeError, "refused redirect"):
            generator._RejectRedirects().redirect_request(
                request,
                None,
                302,
                "Found",
                {},
                "https://internal.example/test.txt",
            )

    def test_typescript_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_typescript(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn('export const UNICODE_VERSION = "17.0.0";', rendered)
        self.assertIn("export function nfc", rendered)
        self.assertIn("export function nfkcCasefold", rendered)
        self.assertIn("export function fullUppercase", rendered)
        self.assertNotIn(".normalize(", rendered)
        self.assertNotIn("toLocale", rendered)

    def test_typescript_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.TYPESCRIPT_TARGET,
            Path(
                "code/programs/typescript/build-tool/src/tracked-artifact-unicode17.ts"
            ),
        )
        self.assertIn(
            Path("code/programs/typescript/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_ruby_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_ruby(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn('UNICODE_VERSION = "17.0.0"', rendered)
        self.assertIn("def nfc", rendered)
        self.assertIn("def nfkc_casefold", rendered)
        self.assertIn("def full_uppercase", rendered)
        self.assertNotIn("unicode_normalize", rendered)
        self.assertNotIn("downcase", rendered)

    def test_ruby_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.RUBY_TARGET,
            Path(
                "code/programs/ruby/build-tool/lib/build_tool/"
                "tracked_artifact_unicode17.rb"
            ),
        )
        self.assertIn(
            Path("code/programs/ruby/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_elixir_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_elixir(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn('@unicode_version "17.0.0"', rendered)
        self.assertIn("def unicode_version", rendered)
        self.assertIn("def nfc", rendered)
        self.assertIn("def nfkc", rendered)
        self.assertIn("def casefold", rendered)
        self.assertIn("def nfkc_casefold", rendered)
        self.assertIn("def full_uppercase", rendered)
        self.assertNotIn(":unicode.characters_to_nfc", rendered)
        self.assertNotIn("String.upcase", rendered)

    def test_elixir_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.ELIXIR_TARGET,
            Path(
                "code/programs/elixir/build-tool/lib/build_tool/"
                "tracked_artifact_unicode17.ex"
            ),
        )
        self.assertIn(
            Path("code/programs/elixir/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_lua_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_lua(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn('Unicode.UNICODE_VERSION = "17.0.0"', rendered)
        self.assertIn("function Unicode.nfc", rendered)
        self.assertIn("function Unicode.nfkc", rendered)
        self.assertIn("function Unicode.casefold", rendered)
        self.assertIn("function Unicode.nfkc_casefold", rendered)
        self.assertIn("function Unicode.full_uppercase", rendered)
        self.assertNotIn("utf8.nf", rendered)
        self.assertNotIn("string.lower", rendered)

    def test_lua_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.LUA_TARGET,
            Path(
                "code/programs/lua/build-tool/lib/build_tool/"
                "tracked_artifact_unicode17.lua"
            ),
        )
        self.assertIn(
            Path("code/programs/lua/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_perl_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_perl(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn(
            "package CodingAdventures::BuildTool::TrackedArtifactUnicode17;", rendered
        )
        self.assertIn("our $UNICODE_VERSION = '17.0.0';", rendered)
        self.assertIn("sub nfc", rendered)
        self.assertIn("sub nfkc", rendered)
        self.assertIn("sub casefold", rendered)
        self.assertIn("sub nfkc_casefold", rendered)
        self.assertIn("sub full_uppercase", rendered)
        self.assertNotIn("Unicode::Normalize", rendered)
        self.assertNotIn("uc(", rendered)

    def test_perl_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.PERL_TARGET,
            Path(
                "code/programs/perl/build-tool/lib/CodingAdventures/BuildTool/"
                "TrackedArtifactUnicode17.pm"
            ),
        )
        self.assertIn(
            Path("code/programs/perl/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_haskell_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_haskell(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn("module TrackedArtifactUnicode17", rendered)
        self.assertIn('unicodeVersion = "17.0.0"', rendered)
        self.assertIn("nfc :: String -> String", rendered)
        self.assertIn("nfkcCasefold :: String -> String", rendered)
        self.assertIn("fullUppercase :: String -> String", rendered)
        self.assertNotIn("Data.Text.Normalize", rendered)
        self.assertNotIn("toUpper", rendered)

    def test_haskell_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.HASKELL_TARGET,
            Path("code/programs/haskell/build-tool/src/TrackedArtifactUnicode17.hs"),
        )
        self.assertIn(
            Path("code/programs/haskell/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_swift_renderer_exports_the_pinned_process_free_api(self) -> None:
        rendered = generator._render_swift(
            (
                [(0x0300, 230)],
                [(0x00C0, False, (0x0041, 0x0300))],
                [(0x0041, 0x0300, 0x00C0)],
                [(0x0041, (0x0061,))],
                [(0x0061, (0x0041,))],
            )
        )

        self.assertIn("enum TrackedArtifactUnicode17", rendered)
        self.assertIn('static let unicodeVersion = "17.0.0"', rendered)
        self.assertIn("static func nfc(_ value: String) -> String", rendered)
        self.assertIn("static func nfkcCasefold(_ value: String) -> String", rendered)
        self.assertIn("static func fullUppercase(_ value: String) -> String", rendered)
        self.assertNotIn("precomposedStringWithCanonicalMapping", rendered)
        self.assertNotIn(".uppercased()", rendered)
        self.assertNotIn("import Foundation", rendered)

    def test_swift_output_and_license_are_declared_targets(self) -> None:
        self.assertEqual(
            generator.SWIFT_TARGET,
            Path(
                "code/programs/swift/build-tool/Sources/BuildToolCore/"
                "TrackedArtifactUnicode17.swift"
            ),
        )
        self.assertIn(
            Path("code/programs/swift/build-tool/UNICODE-LICENSE.txt"),
            generator.LICENSE_TARGETS,
        )

    def test_typescript_self_check_runs_every_official_vector_family(self) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        with tempfile.TemporaryDirectory() as temporary:
            root = Path(temporary)
            tsx_cli = (
                root
                / "code/programs/typescript/build-tool/node_modules/tsx/dist/cli.mjs"
            )
            tsx_cli.parent.mkdir(parents=True)
            tsx_cli.write_text("", encoding="utf-8")
            completed = subprocess.CompletedProcess([], 0, "ok\n", "")
            with (
                mock.patch.object(generator.shutil, "which", return_value="node"),
                mock.patch.object(
                    generator.subprocess, "run", return_value=completed
                ) as run,
            ):
                generator._self_check_typescript(
                    root,
                    "export {};\n",
                    sources,
                    _Module(),
                )

        invocation = run.call_args.kwargs
        payload = json.loads(invocation["input"])
        self.assertEqual(payload["unicodeVersion"], "17.0.0")
        self.assertEqual(payload["normalization"], [["A", "A", "A", "A", "A"]])
        self.assertEqual(payload["folding"], [["A", "a", "a"]])
        self.assertEqual(payload["uppercase"], [["a", "A"]])
        self.assertEqual(invocation["timeout"], 180)
        self.assertFalse(invocation["check"])

    def test_ruby_self_check_runs_every_official_vector_family(self) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        completed = subprocess.CompletedProcess([], 0, "ok\n", "")
        with (
            mock.patch.object(generator.shutil, "which", return_value="ruby"),
            mock.patch.object(
                generator.subprocess, "run", return_value=completed
            ) as run,
        ):
            generator._self_check_ruby(
                Path("C:/repo"),
                "module BuildTool; module TrackedArtifactUnicode17; end; end\n",
                sources,
                _Module(),
            )

        invocation = run.call_args.kwargs
        payload = json.loads(invocation["input"])
        self.assertEqual(payload["unicodeVersion"], "17.0.0")
        self.assertEqual(payload["normalization"], [["A", "A", "A", "A", "A"]])
        self.assertEqual(payload["folding"], [["A", "a", "a"]])
        self.assertEqual(payload["uppercase"], [["a", "A"]])
        self.assertEqual(invocation["timeout"], 180)
        self.assertFalse(invocation["check"])

    def test_elixir_self_check_runs_every_official_vector_family(self) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        completed = subprocess.CompletedProcess([], 0, "ok\n", "")
        with (
            mock.patch.object(generator.shutil, "which", return_value="elixir"),
            mock.patch.object(
                generator.subprocess, "run", return_value=completed
            ) as run,
        ):
            generator._self_check_elixir(
                Path("C:/repo"),
                "defmodule BuildTool.TrackedArtifactUnicode17 do\nend\n",
                sources,
                _Module(),
            )

        invocation = run.call_args.kwargs
        payload = json.loads(invocation["input"])
        self.assertEqual(payload["unicodeVersion"], "17.0.0")
        self.assertEqual(payload["normalization"], [["A", "A", "A", "A", "A"]])
        self.assertEqual(payload["folding"], [["A", "a", "a"]])
        self.assertEqual(payload["uppercase"], [["a", "A"]])
        self.assertEqual(invocation["timeout"], 180)
        self.assertFalse(invocation["check"])

    def test_lua_self_check_runs_every_official_vector_family(self) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        executable = Path(sys.executable).resolve()
        version = subprocess.CompletedProcess([], 0, "", "Lua 5.4.7\n")
        completed = subprocess.CompletedProcess([], 0, "ok\r\n", "")
        with mock.patch.object(
            generator,
            "_run_bounded_process",
            side_effect=(version, completed),
        ) as run:
            generator._self_check_lua(
                Path("C:/repo"),
                "return {}\n",
                sources,
                _Module(),
                executable,
            )

        version_command = run.call_args_list[0].args[0]
        self.assertEqual(version_command, [str(executable), "-E", "-v"])
        command = run.call_args_list[1].args[0]
        self.assertEqual(command[:2], [str(executable), "-E"])
        invocation = run.call_args_list[1].kwargs
        self.assertEqual(
            invocation["input_text"],
            "V;17.0.0\nN;41;41;41;41;41\nF;41;61;61\nU;61;41\n",
        )
        self.assertEqual(invocation["timeout"], 180)
        self.assertNotIn("PATH", invocation["env"])
        self.assertNotIn("LUA_INIT", invocation["env"])
        self.assertNotIn("LUA_PATH", invocation["env"])

    def test_perl_self_check_runs_every_official_vector_family(self) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        executable = Path(sys.executable).resolve()
        version = subprocess.CompletedProcess(
            [], 0, "This is perl 5, version 38, subversion 2 (v5.38.2)\n", ""
        )
        completed = subprocess.CompletedProcess([], 0, "ok\r\n", "")
        with mock.patch.object(
            generator,
            "_run_bounded_process",
            side_effect=(version, completed),
        ) as run:
            generator._self_check_perl(
                Path("C:/repo"),
                "1;\n",
                sources,
                _Module(),
                executable,
            )

        version_command = run.call_args_list[0].args[0]
        self.assertEqual(version_command, [str(executable), "-T", "-v"])
        command = run.call_args_list[1].args[0]
        self.assertEqual(command[:2], [str(executable), "-T"])
        self.assertIn("-I", command)
        invocation = run.call_args_list[1].kwargs
        self.assertEqual(
            invocation["input_text"],
            "V;17.0.0\nN;41;41;41;41;41\nF;41;61;61\nU;61;41\n",
        )
        self.assertEqual(invocation["timeout"], 180)
        self.assertNotIn("PATH", invocation["env"])
        self.assertNotIn("PERL5OPT", invocation["env"])
        self.assertNotIn("PERL5LIB", invocation["env"])

    def test_haskell_self_check_runs_every_official_vector_family(self) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        executable = Path(sys.executable).resolve()
        runghc_version = subprocess.CompletedProcess([], 0, "runghc 9.4.8\n", "")
        ghc_version = subprocess.CompletedProcess([], 0, "9.4.8\n", "")
        completed = subprocess.CompletedProcess([], 0, "ok\r\n", "")
        with mock.patch.object(
            generator,
            "_run_bounded_process",
            side_effect=(runghc_version, ghc_version, completed),
        ) as run:
            generator._self_check_haskell(
                Path("C:/repo"),
                "module TrackedArtifactUnicode17 where\n",
                sources,
                _Module(),
                executable,
                executable,
            )

        self.assertEqual(run.call_args_list[0].args[0], [str(executable), "--version"])
        self.assertEqual(
            run.call_args_list[1].args[0], [str(executable), "--numeric-version"]
        )
        command = run.call_args_list[2].args[0]
        self.assertEqual(command[:3], [str(executable), "-f", str(executable)])
        self.assertIn("--ghc-arg=-ignore-dot-ghci", command)
        self.assertIn("--ghc-arg=-clear-package-db", command)
        self.assertIn("--ghc-arg=-global-package-db", command)
        self.assertIn("--ghc-arg=-package-env=-", command)
        self.assertIn("--ghc-arg=-hide-all-packages", command)
        self.assertIn("--ghc-arg=-package=base", command)
        self.assertIn("--ghc-arg=-package=containers", command)
        self.assertTrue(
            any(argument.startswith("--ghc-arg=-tmpdir=") for argument in command)
        )
        invocation = run.call_args_list[2].kwargs
        self.assertEqual(
            invocation["input_text"],
            "V;17.0.0\nN;41;41;41;41;41\nF;41;61;61\nU;61;41\n",
        )
        self.assertEqual(invocation["timeout"], 180)
        self.assertNotIn("PATH", invocation["env"])
        self.assertNotIn("GHC_ENVIRONMENT", invocation["env"])
        self.assertNotIn("CABAL_DIR", invocation["env"])

    def test_swift_self_check_compiles_and_runs_every_official_vector_family(
        self,
    ) -> None:
        sources = {
            "NormalizationTest.txt": "0041;0041;0041;0041;0041; # LATIN A\n",
            "CaseFolding.txt": "0041; C; 0061; # LATIN A\n",
            "UnicodeData.txt": ";".join(["0061"] + [""] * 11 + ["0041"] + [""] * 2),
            "SpecialCasing.txt": "0061; 0061; 0041; 0041; ; # LATIN A\n",
        }

        class _Module:
            @staticmethod
            def nfkc_casefold(value: str) -> str:
                return value.lower()

        executable = Path(sys.executable).resolve()
        version = subprocess.CompletedProcess(
            [],
            0,
            ("Swift version 6.3.3 (swift-6.3.3-RELEASE)\nTarget: test-target\n"),
            "",
        )
        compiled = subprocess.CompletedProcess([], 0, "", "")
        completed = subprocess.CompletedProcess([], 0, "ok\r\n", "")
        reviewed_runtime = Path("C:/reviewed-swift-runtime")
        reviewed_sdk = Path("C:/reviewed-swift-sdk")
        with (
            mock.patch.object(
                generator,
                "_swift_windows_runtime_directory",
                return_value=reviewed_runtime,
            ),
            mock.patch.object(
                generator,
                "_swift_windows_sdk_directory",
                return_value=reviewed_sdk,
            ),
            mock.patch.object(
                generator,
                "_swift_windows_linker_arguments",
                return_value=["-use-ld=lld", "-L", "C:/reviewed-swift-libs"],
            ),
            mock.patch.object(
                generator,
                "_run_bounded_process",
                side_effect=(version, version, compiled, completed),
            ) as run,
        ):
            generator._self_check_swift(
                Path("C:/repo"),
                "enum TrackedArtifactUnicode17 {}\n",
                sources,
                _Module(),
                executable,
                executable,
            )

        self.assertEqual(run.call_args_list[0].args[0], [str(executable), "--version"])
        self.assertEqual(run.call_args_list[1].args[0], [str(executable), "--version"])
        compile_command = run.call_args_list[2].args[0]
        self.assertEqual(compile_command[0], str(executable))
        self.assertIn("-no-color-diagnostics", compile_command)
        self.assertIn("-module-cache-path", compile_command)
        self.assertIn("-o", compile_command)
        if os.name == "nt":
            self.assertEqual(compile_command[1:3], ["-sdk", str(reviewed_sdk)])
            self.assertIn("-use-ld=lld", compile_command)
            self.assertIn("C:/reviewed-swift-libs", compile_command)
        self.assertTrue(
            any(
                argument.endswith("TrackedArtifactUnicode17.swift")
                for argument in compile_command
            )
        )
        self.assertTrue(
            any(argument.endswith("main.swift") for argument in compile_command)
        )
        self.assertEqual(run.call_args_list[2].kwargs["input_text"], "")
        self.assertEqual(run.call_args_list[2].kwargs["timeout"], 180)

        run_command = run.call_args_list[3].args[0]
        self.assertEqual(len(run_command), 1)
        self.assertTrue(
            run_command[0].endswith(
                "self-check.exe" if os.name == "nt" else "self-check"
            )
        )
        invocation = run.call_args_list[3].kwargs
        self.assertEqual(
            invocation["input_text"],
            "V;17.0.0\nN;41;41;41;41;41\nF;41;61;61\nU;61;41\n",
        )
        self.assertEqual(invocation["timeout"], 180)
        self.assertNotIn("HOME", invocation["env"])
        self.assertNotIn("TOOLCHAINS", invocation["env"])
        self.assertNotIn("SWIFT_EXEC", invocation["env"])
        if os.name == "nt":
            self.assertEqual(
                invocation["env"]["PATH"],
                os.pathsep.join((str(executable.parent), str(reviewed_runtime))),
            )
            self.assertEqual(invocation["env"]["SDKROOT"], str(reviewed_sdk))
        else:
            self.assertNotIn("PATH", invocation["env"])
            self.assertNotIn("SDKROOT", invocation["env"])

    def test_swift_self_check_requires_exact_scalar_sequences(self) -> None:
        runner = generator._SWIFT_SELF_CHECK
        self.assertIn("func scalarEqual(_ left: String, _ right: String)", runner)
        self.assertIn(
            "scalarEqual(TrackedArtifactUnicode17.nfc(c1), c2)",
            runner,
        )
        self.assertIn(
            "scalarEqual(TrackedArtifactUnicode17.nfkc(c5), c4)",
            runner,
        )
        self.assertIn(
            "scalarEqual(\n                        TrackedArtifactUnicode17.casefold(source)",
            runner,
        )
        self.assertNotIn("TrackedArtifactUnicode17.nfc(c1) == c2", runner)

    def test_swift_driver_entrypoint_preserves_symlink_name(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            temporary_path = Path(temporary)
            entrypoint = temporary_path / "swift"
            driver = temporary_path / "swift-driver"
            driver.write_text("driver", encoding="utf-8")
            with mock.patch.object(Path, "resolve", return_value=driver):
                validated = generator._swift_driver_entrypoint(
                    entrypoint,
                    "runtime",
                )

        self.assertEqual(validated, entrypoint.absolute())
        self.assertEqual(validated.name, "swift")

    def test_bounded_process_discards_output_past_the_limit(self) -> None:
        with tempfile.TemporaryDirectory() as temporary:
            completed = generator._run_bounded_process(
                [
                    sys.executable,
                    "-c",
                    (
                        "import sys; sys.stdout.write('x' * 4096); "
                        "sys.stderr.write('y' * 4096)"
                    ),
                ],
                cwd=Path(temporary),
                env=generator._lua_self_check_environment(),
                input_text="",
                timeout=10,
                output_limit=64,
            )

        self.assertEqual(completed.returncode, 0)
        self.assertEqual(len(completed.stdout), 64)
        self.assertEqual(len(completed.stderr), 64)

    def test_bounded_process_terminates_a_timed_out_process_tree(self) -> None:
        with (
            tempfile.TemporaryDirectory() as temporary,
            self.assertRaisesRegex(RuntimeError, "exceeded 1 seconds"),
        ):
            generator._run_bounded_process(
                [sys.executable, "-c", "import time; time.sleep(60)"],
                cwd=Path(temporary),
                env=generator._lua_self_check_environment(),
                input_text="",
                timeout=1,
                output_limit=64,
            )

    def test_bounded_process_contains_descendants_after_root_exit(self) -> None:
        child_source = "import time; time.sleep(60)"
        root_source = (
            "import subprocess, sys; "
            "child = subprocess.Popen("
            "[sys.executable, '-c', sys.argv[1]], "
            "stdin=subprocess.DEVNULL, stdout=subprocess.DEVNULL, "
            "stderr=subprocess.DEVNULL); "
            "print(child.pid, flush=True)"
        )
        child_pid = None
        try:
            completed = generator._run_bounded_process(
                [sys.executable, "-c", root_source, child_source],
                cwd=Path(__file__).resolve().parent,
                env=generator._lua_self_check_environment(),
                input_text="",
                timeout=10,
                output_limit=64,
            )
            self.assertEqual(completed.returncode, 0)
            child_pid = int(completed.stdout.strip())
            self.assertTrue(_wait_for_test_process_exit(child_pid))
        finally:
            if child_pid is not None and _test_process_is_running(child_pid):
                _terminate_test_process(child_pid)

    @unittest.skipUnless(os.name == "nt", "Windows Job Object regression")
    def test_bounded_process_does_not_reuse_a_consumed_windows_job(self) -> None:
        with (
            mock.patch.object(
                generator,
                "_terminate_process_tree",
                side_effect=RuntimeError("cleanup failed"),
            ) as terminate,
            self.assertRaisesRegex(RuntimeError, "cleanup failed"),
        ):
            generator._run_bounded_process(
                [sys.executable, "-c", "pass"],
                cwd=Path(__file__).resolve().parent,
                env=generator._lua_self_check_environment(),
                input_text="",
                timeout=10,
                output_limit=64,
            )
        terminate.assert_called_once()

    @unittest.skipUnless(os.name == "nt", "Windows Job Object regression")
    def test_bounded_process_reaps_a_job_setup_failure(self) -> None:
        started: list[subprocess.Popen] = []
        real_popen = subprocess.Popen

        def start_process(*args, **kwargs):
            process = real_popen(*args, **kwargs)
            started.append(process)
            return process

        try:
            with (
                mock.patch.object(
                    generator.subprocess,
                    "Popen",
                    side_effect=start_process,
                ),
                mock.patch.object(
                    generator,
                    "_create_windows_kill_on_close_job",
                    side_effect=OSError("job setup failed"),
                ),
                self.assertRaisesRegex(OSError, "job setup failed"),
            ):
                generator._run_bounded_process(
                    [sys.executable, "-c", "pass"],
                    cwd=Path(__file__).resolve().parent,
                    env=generator._lua_self_check_environment(),
                    input_text="",
                    timeout=10,
                    output_limit=64,
                )
            self.assertEqual(len(started), 1)
            self.assertIsNotNone(started[0].poll())
        finally:
            for process in started:
                if process.poll() is None:
                    process.kill()
                    process.wait(timeout=5)


if __name__ == "__main__":
    unittest.main()
