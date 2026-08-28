from __future__ import annotations

import contextlib
import io
import json
import sys
import unittest
from pathlib import Path
from unittest import mock

import jsonschema

REPO_ROOT = Path(__file__).resolve().parents[3]
SCRIPTS_DIR = REPO_ROOT / "code" / "scripts"
sys.path.insert(0, str(SCRIPTS_DIR))

import validate_elixir_windows_build_fronts as audit

EXPECTED_EXCEPTIONS = {
    "code/packages/elixir/conduit": ("ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE"),
    "code/packages/elixir/gf256_native": ("ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE"),
    "code/packages/elixir/irc-server-native": ("ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE"),
    "code/packages/elixir/paint_codec_png_native": (
        "ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE"
    ),
    "code/packages/elixir/paint_vm_metal_native": ("ELIXIR_WINDOWS_METAL_UNAVAILABLE"),
    "code/packages/elixir/polynomial_native": ("ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE"),
    "code/programs/elixir/conduit-hello": ("ELIXIR_WINDOWS_NIF_DEPENDENCY_UNAVAILABLE"),
}


class FixtureTests(unittest.TestCase):
    def test_contract_is_closed_by_its_schema(self) -> None:
        fixture_dir = (
            REPO_ROOT / "code" / "specs" / "fixtures" / "elixir-windows-build-front-v1"
        )
        schema = json.loads((fixture_dir / "schema.json").read_text(encoding="utf-8"))
        contract = json.loads(
            (fixture_dir / "contract.json").read_text(encoding="utf-8")
        )

        jsonschema.Draft202012Validator(schema).validate(contract)
        self.assertEqual(
            {row["root"]: row["code"] for row in contract["exceptions"]},
            EXPECTED_EXCEPTIONS,
        )

    def test_contract_rejects_duplicate_keys_and_duplicate_roots(self) -> None:
        with self.assertRaisesRegex(audit.AuditError, "duplicate JSON key"):
            audit.parse_json_strict('{"schema_version": 1, "schema_version": 1}')

        contract = audit.load_contract(REPO_ROOT)
        duplicate = dict(contract)
        duplicate["exceptions"] = [
            *contract["exceptions"],
            dict(contract["exceptions"][0]),
        ]
        with self.assertRaisesRegex(audit.AuditError, "duplicate exception root"):
            audit.validate_contract(duplicate)


class FrontParserTests(unittest.TestCase):
    def setUp(self) -> None:
        self.contract = audit.load_contract(REPO_ROOT)

    def test_exact_unsupported_front_returns_its_stable_code(self) -> None:
        code = "ELIXIR_WINDOWS_NIF_LINK_UNAVAILABLE"
        text = (
            f"# build-tool: unsupported={code}\n"
            f"echo BUILD_TOOL_UNSUPPORTED:{code} -- skipped\n"
        )

        self.assertEqual(audit.parse_unsupported_front(text, self.contract), code)

    def test_unsupported_front_rejects_mismatches_and_extra_commands(self) -> None:
        code = "ELIXIR_WINDOWS_METAL_UNAVAILABLE"
        with self.assertRaisesRegex(audit.AuditError, "directive and command codes"):
            audit.parse_unsupported_front(
                "# build-tool: unsupported=ELIXIR_WINDOWS_OTHER\n"
                f"echo BUILD_TOOL_UNSUPPORTED:{code} -- skipped\n",
                self.contract,
            )
        with self.assertRaisesRegex(audit.AuditError, "exactly one active command"):
            audit.parse_unsupported_front(
                f"# build-tool: unsupported={code}\n"
                f"echo BUILD_TOOL_UNSUPPORTED:{code} -- skipped\n"
                "mix test\n",
                self.contract,
            )

    def test_cmd_syntax_audit_rejects_posix_forms_and_accepts_cmd_env(self) -> None:
        issues = audit.cmd_syntax_issues(
            "MIX_ENV=test mix compile\n"
            "mix test 2>/dev/null\n"
            "cd -\n"
            "echo $(uname)\n"
            "mkdir -p priv\n"
        )
        self.assertEqual(
            [issue["code"] for issue in issues],
            [
                "POSIX_ENV_PREFIX",
                "POSIX_DEV_NULL",
                "POSIX_CD_DASH",
                "POSIX_COMMAND_SUBSTITUTION",
                "POSIX_MKDIR_P",
            ],
        )
        self.assertEqual(
            audit.cmd_syntax_issues(
                "set MIX_ENV=test&& mix compile --warnings-as-errors\n"
            ),
            [],
        )

    def test_non_protocol_echo_is_not_an_unsupported_front(self) -> None:
        self.assertIsNone(
            audit.parse_unsupported_front(
                "echo this is only a human skip message\n", self.contract
            )
        )


class RepositoryAuditTests(unittest.TestCase):
    report: dict[str, object]

    @classmethod
    def setUpClass(cls) -> None:
        cls.report = audit.build_report(REPO_ROOT)

    def test_report_classifies_every_current_root(self) -> None:
        summary = self.report["summary"]
        self.assertEqual(
            summary,
            {
                "canonical_fallbacks": 159,
                "declarative_starlark": 9,
                "native": 278,
                "package_roots": 276,
                "program_roots": 9,
                "total_roots": 285,
                "unsupported": 7,
                "windows_overrides": 126,
            },
        )
        roots = self.report["roots"]
        self.assertEqual(len(roots), 285)
        self.assertEqual(
            [row["root"] for row in roots],
            sorted(row["root"] for row in roots),
        )
        self.assertTrue(all(not row["issues"] for row in roots))

    def test_report_has_only_the_reviewed_exceptions(self) -> None:
        actual = {
            row["root"]: row["diagnostic_code"]
            for row in self.report["roots"]
            if row["classification"] == "unsupported"
        }
        self.assertEqual(actual, EXPECTED_EXCEPTIONS)
        for row in self.report["roots"]:
            if row["classification"] == "unsupported":
                self.assertEqual(row["selected_front"], "BUILD_windows")
            else:
                self.assertIsNone(row["diagnostic_code"])

    def test_pure_beam_ciphers_are_native_windows_fallbacks(self) -> None:
        by_root = {row["root"]: row for row in self.report["roots"]}
        for package in ("atbash_cipher", "scytale_cipher", "vigenere_cipher"):
            row = by_root[f"code/packages/elixir/{package}"]
            self.assertEqual(row["classification"], "native")
            self.assertEqual(row["selected_front"], "BUILD")

        zip_row = by_root["code/packages/elixir/zip"]
        self.assertEqual(zip_row["classification"], "native")
        self.assertEqual(zip_row["selected_front"], "BUILD_windows")

    def test_workflow_uses_the_exact_pinned_windows_contract(self) -> None:
        workflow = self.report["workflow"]
        self.assertEqual(workflow["pr_windows_runner"], "windows-2025")
        self.assertEqual(
            workflow["setup_action"],
            "erlef/setup-beam@54075bcc5e249e4758d363f27d099f55d843f124",
        )
        self.assertEqual(workflow["setup_action_occurrences"], 6)
        self.assertTrue(workflow["windows_setup_enabled"])
        self.assertTrue(workflow["windows_verification_enabled"])
        self.assertTrue(workflow["windows_affected_build_enabled"])

    def test_markdown_is_a_rendering_of_the_same_report(self) -> None:
        markdown = audit.render_markdown(self.report)
        self.assertIn("| Total Elixir roots | 285 |", markdown)
        self.assertIn("| Native Windows fronts | 278 |", markdown)
        self.assertIn("`ELIXIR_WINDOWS_METAL_UNAVAILABLE`", markdown)

    def test_cli_renders_both_formats_and_fails_closed(self) -> None:
        for output_format, expected in (
            ("json", '"schema_version": 1'),
            ("markdown", "# Elixir Windows BUILD-front audit"),
        ):
            with self.subTest(output_format=output_format):
                stdout = io.StringIO()
                argv = [
                    "validate_elixir_windows_build_fronts.py",
                    "--format",
                    output_format,
                ]
                with (
                    mock.patch.object(sys, "argv", argv),
                    mock.patch.object(audit, "build_report", return_value=self.report),
                    contextlib.redirect_stdout(stdout),
                ):
                    self.assertEqual(audit.main(), 0)
                self.assertIn(expected, stdout.getvalue())

        stderr = io.StringIO()
        with (
            mock.patch.object(
                audit, "build_report", side_effect=audit.AuditError("broken contract")
            ),
            contextlib.redirect_stderr(stderr),
        ):
            self.assertEqual(audit.main([]), 1)
        self.assertIn("broken contract", stderr.getvalue())


if __name__ == "__main__":
    unittest.main()
