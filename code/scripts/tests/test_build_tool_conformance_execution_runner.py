from __future__ import annotations

import copy
import io
import json
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as bootstrap
import build_tool_conformance_execution as execution

FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT
EMPTY_DIGEST = "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"


class ExecutionPolicyRunnerTests(unittest.TestCase):
    def test_checked_in_contract_validates_without_execution(self) -> None:
        summary = execution.validate_contract(FIXTURE_ROOT)
        self.assertEqual(summary["schema_version"], 1)
        self.assertEqual(summary["execution_case_count"], 0)
        self.assertEqual(summary["execution_corpus_sha256"], EMPTY_DIGEST)
        self.assertEqual(summary["ready_backend_count"], 0)
        self.assertEqual(summary["status"], "valid")
        self.assertEqual(summary["conformance_status"], "not-run")

    def test_operator_authorization_is_checked_before_case_read(self) -> None:
        missing = Path("definitely-not-present.json")
        with (
            mock.patch.object(execution.authority, "authorize_preflight") as verifier,
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            execution.run_case(
                missing,
                language="go",
                authority_bundle=Path("missing-authority.json"),
                approved_authority_digest="0" * 64,
                expected_commit_oid="a" * 40,
                expected_tree_oid="b" * 40,
                allow_trusted_execution=False,
            )
        self.assertEqual(raised.exception.code, "EXECUTION_AUTHORIZATION_REQUIRED")
        verifier.assert_not_called()

    def test_approved_authority_syntax_is_checked_before_bundle_or_case(self) -> None:
        missing = Path("definitely-not-present.json")
        with (
            mock.patch.object(execution.authority, "authorize_preflight") as verifier,
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            execution.run_case(
                missing,
                language="go",
                authority_bundle=Path("missing-authority.json"),
                approved_authority_digest="not-a-digest",
                expected_commit_oid="a" * 40,
                expected_tree_oid="b" * 40,
                allow_trusted_execution=True,
            )
        self.assertEqual(raised.exception.code, "AUTHORITY_DIGEST_INVALID")
        verifier.assert_not_called()

    def test_preflight_scope_cannot_authorize_case_or_import_process_api(self) -> None:
        with mock.patch.object(
            execution.authority,
            "authorize_preflight",
            return_value=mock.sentinel.preflight_authority,
        ) as verifier:
            result = execution.run_case(
                Path("never-read-case.json"),
                language="go",
                authority_bundle=Path("authority.json"),
                approved_authority_digest="0" * 64,
                expected_commit_oid="a" * 40,
                expected_tree_oid="b" * 40,
                allow_trusted_execution=True,
            )
        self.assertEqual(result["outcome"], "skipped")
        self.assertEqual(result["conformance_status"], "non-passing")
        self.assertEqual(
            result["diagnostics"][0]["code"],
            "EXECUTION_AUTHORITY_SCOPE_UNAVAILABLE",
        )
        verifier.assert_called_once()
        self.assertFalse(hasattr(execution, "subprocess"))

    def test_platform_mapping_is_explicit_and_fail_closed(self) -> None:
        with mock.patch.object(execution.sys, "platform", "linux-x"):
            self.assertEqual(execution._platform_name(), "linux")
        with mock.patch.object(execution.sys, "platform", "darwin"):
            self.assertEqual(execution._platform_name(), "darwin")
        with (
            mock.patch.object(execution.sys, "platform", "other"),
            mock.patch.object(execution.os, "name", "nt"),
        ):
            self.assertEqual(execution._platform_name(), "windows")
        with (
            mock.patch.object(execution.sys, "platform", "other"),
            mock.patch.object(execution.os, "name", "posix"),
        ):
            self.assertEqual(execution._platform_name(), "unsupported")

    def test_cli_validate_contract_and_preflight_only_run_case_have_stable_codes(
        self,
    ) -> None:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = execution.main(["validate-contract"])
        self.assertEqual(exit_code, 0)
        self.assertEqual(json.loads(stdout.getvalue())["status"], "valid")

        stdout = io.StringIO()
        stderr = io.StringIO()
        with (
            mock.patch.object(
                execution.authority,
                "authorize_preflight",
                return_value=mock.sentinel.preflight_authority,
            ),
            redirect_stdout(stdout),
            redirect_stderr(stderr),
        ):
            exit_code = execution.main(
                [
                    "run-case",
                    "--case",
                    "never-read.json",
                    "--language",
                    "go",
                    "--authority-bundle",
                    "authority.json",
                    "--approved-authority-sha256",
                    "0" * 64,
                    "--source-commit",
                    "a" * 40,
                    "--source-tree",
                    "b" * 40,
                    "--allow-trusted-execution",
                ]
            )
        self.assertEqual(exit_code, 1)
        self.assertEqual(json.loads(stdout.getvalue())["outcome"], "skipped")
        self.assertEqual(stderr.getvalue(), "")

        stderr = io.StringIO()
        with redirect_stderr(stderr):
            exit_code = execution.main(
                [
                    "run-case",
                    "--case",
                    "never-read.json",
                    "--language",
                    "go",
                    "--authority-bundle",
                    "authority.json",
                    "--approved-authority-sha256",
                    "0" * 64,
                    "--source-commit",
                    "a" * 40,
                    "--source-tree",
                    "b" * 40,
                ]
            )
        self.assertEqual(exit_code, 2)
        self.assertEqual(
            json.loads(stderr.getvalue())["code"],
            "EXECUTION_AUTHORIZATION_REQUIRED",
        )

        legacy_stderr = io.StringIO()
        with redirect_stderr(legacy_stderr):
            exit_code = execution.main(
                [
                    "run-case",
                    "--case",
                    "never-read.json",
                    "--language",
                    "go",
                    "--authority-bundle",
                    "authority.json",
                    "--approved-authority-sha256",
                    "0" * 64,
                    "--source-commit",
                    "a" * 40,
                    "--source-tree",
                    "b" * 40,
                    "--approved-corpus-sha256",
                    EMPTY_DIGEST,
                    "--allow-trusted-execution",
                ]
            )
        self.assertEqual(exit_code, 2)
        self.assertIn("unrecognized arguments", legacy_stderr.getvalue())

        with redirect_stderr(io.StringIO()):
            self.assertEqual(execution.main([]), 2)

    def test_cli_errors_redact_host_paths(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            secret_root = Path(directory) / "host-secret-root"
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                exit_code = execution.main(
                    [
                        "validate-contract",
                        "--fixture-root",
                        str(secret_root),
                    ]
                )
        self.assertEqual(exit_code, 2)
        error = json.loads(stderr.getvalue())
        self.assertEqual(error["code"], "DOCUMENT_READ_FAILED")
        self.assertEqual(
            error["message"],
            "trusted-execution contract validation failed",
        )
        self.assertNotIn(str(secret_root), stderr.getvalue())
        self.assertNotIn(Path(directory).name, stderr.getvalue())

    def test_fixture_arguments_environment_and_manifest_never_select_authority(
        self,
    ) -> None:
        policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
        mutated = copy.deepcopy(policy)
        mutated["fixture_arguments"] = ["--replace-adapter", "host.exe"]
        schema = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.schema.json")
        self.assertTrue(bootstrap._schema_errors(mutated, schema))


if __name__ == "__main__":
    unittest.main()
