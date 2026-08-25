from __future__ import annotations

import hashlib
import json
import subprocess
import sys
import tempfile
import unittest
import urllib.request
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import generate_tracked_artifact_unicode17 as generator


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


if __name__ == "__main__":
    unittest.main()
