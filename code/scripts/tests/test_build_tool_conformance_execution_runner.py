from __future__ import annotations

import copy
import io
import json
import shutil
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


def write_policy_fixture(root: Path, policy: dict[str, object]) -> Path:
    fixture_root = root / "fixtures"
    fixture_root.mkdir()
    for name in (
        "schema.json",
        "result.schema.json",
        "execution.schema.json",
        "execution-policy.schema.json",
        "linux-oci-backend.schema.json",
    ):
        shutil.copyfile(FIXTURE_ROOT / name, fixture_root / name)
    (fixture_root / "execution-cases").mkdir()
    (fixture_root / "execution-policy.json").write_text(
        json.dumps(policy),
        encoding="utf-8",
    )
    return fixture_root


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
            mock.patch.object(execution, "_read_raw_regular") as reader,
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            execution.run_case(
                missing,
                language="go",
                approved_digest=EMPTY_DIGEST,
                allow_trusted_execution=False,
                fixture_root=FIXTURE_ROOT,
            )
        self.assertEqual(raised.exception.code, "EXECUTION_AUTHORIZATION_REQUIRED")
        reader.assert_not_called()

    def test_approved_digest_is_checked_before_case_decode(self) -> None:
        missing = Path("definitely-not-present.json")
        with (
            mock.patch.object(execution, "_read_raw_regular") as reader,
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            execution.run_case(
                missing,
                language="go",
                approved_digest="0" * 64,
                allow_trusted_execution=True,
                fixture_root=FIXTURE_ROOT,
            )
        self.assertEqual(raised.exception.code, "EXECUTION_DIGEST_MISMATCH")
        reader.assert_not_called()

    def test_disabled_policy_returns_nonpassing_skip_without_process_api(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            case = Path(directory) / "case.json"
            case.write_text("{}", encoding="utf-8")
            result = execution.run_case(
                case,
                language="go",
                approved_digest=EMPTY_DIGEST,
                allow_trusted_execution=True,
                fixture_root=FIXTURE_ROOT,
            )
        self.assertEqual(result["outcome"], "skipped")
        self.assertEqual(result["conformance_status"], "non-passing")
        self.assertEqual(
            result["diagnostics"][0]["code"],
            "EXECUTION_POLICY_DISABLED",
        )
        self.assertFalse(hasattr(execution, "subprocess"))

    def test_ready_backend_is_still_fail_closed_in_policy_only_tranche(self) -> None:
        policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
        policy["enabled"] = True
        linux = next(item for item in policy["backends"] if item["platform"] == "linux")
        linux["status"] = "ready"
        linux["identity_sha256"] = "1" * 64
        policy["adapters"] = [
            {
                "language": "go",
                "platform": "linux",
                "executable": "code/programs/go/build-tool",
                "executable_sha256": "2" * 64,
            }
        ]
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root = write_policy_fixture(root, policy)
            case = root / "case.json"
            case.write_text("{}", encoding="utf-8")
            result = execution.run_case(
                case,
                language="go",
                approved_digest=EMPTY_DIGEST,
                allow_trusted_execution=True,
                fixture_root=fixture_root,
                platform_name="linux",
            )
        self.assertEqual(result["outcome"], "skipped")
        self.assertEqual(
            result["diagnostics"][0]["code"],
            "EXECUTION_BACKEND_UNIMPLEMENTED",
        )

    def test_enabled_policy_skips_unavailable_backend_and_missing_adapter(self) -> None:
        policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
        policy["enabled"] = True
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root = write_policy_fixture(root, policy)
            result = execution.run_case(
                root / "unused.json",
                language="go",
                approved_digest=EMPTY_DIGEST,
                allow_trusted_execution=True,
                fixture_root=fixture_root,
                platform_name="windows",
            )
        self.assertEqual(
            result["diagnostics"][0]["code"],
            "EXECUTION_BACKEND_UNAVAILABLE",
        )

        linux = next(item for item in policy["backends"] if item["platform"] == "linux")
        linux["status"] = "ready"
        linux["identity_sha256"] = "1" * 64
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            fixture_root = write_policy_fixture(root, policy)
            result = execution.run_case(
                root / "unused.json",
                language="go",
                approved_digest=EMPTY_DIGEST,
                allow_trusted_execution=True,
                fixture_root=fixture_root,
                platform_name="linux",
            )
        self.assertEqual(
            result["diagnostics"][0]["code"],
            "EXECUTION_ADAPTER_UNAVAILABLE",
        )

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

    def test_cli_validate_contract_and_disabled_run_case_have_stable_exit_codes(
        self,
    ) -> None:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = execution.main(["validate-contract"])
        self.assertEqual(exit_code, 0)
        self.assertEqual(json.loads(stdout.getvalue())["status"], "valid")

        with tempfile.TemporaryDirectory() as directory:
            case = Path(directory) / "case.json"
            case.write_text("{}", encoding="utf-8")
            stdout = io.StringIO()
            stderr = io.StringIO()
            with redirect_stdout(stdout), redirect_stderr(stderr):
                exit_code = execution.main(
                    [
                        "run-case",
                        "--case",
                        str(case),
                        "--language",
                        "go",
                        "--approved-corpus-sha256",
                        EMPTY_DIGEST,
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
                    str(case),
                    "--language",
                    "go",
                    "--approved-corpus-sha256",
                    EMPTY_DIGEST,
                ]
            )
        self.assertEqual(exit_code, 2)
        self.assertEqual(
            json.loads(stderr.getvalue())["code"],
            "EXECUTION_AUTHORIZATION_REQUIRED",
        )

        with redirect_stderr(io.StringIO()):
            self.assertEqual(execution.main([]), 2)

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
