from __future__ import annotations

import copy
import json
import os
import shutil
import sys
import tempfile
import unittest
from pathlib import Path
from unittest import mock

SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as bootstrap
import build_tool_conformance_execution as execution

FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT


def base_case() -> dict[str, object]:
    return {
        "schema_version": 1,
        "id": "execution/structured-command",
        "domain": "execution",
        "summary": "A structured command remains a direct argument vector.",
        "platforms": ["linux"],
        "capabilities": ["execution", "trusted_execution"],
        "workspace": {"files": []},
        "input": {
            "operation": "execution",
            "options": {
                "platform": "linux",
                "jobs": 1,
                "dry_run": False,
                "packages": [
                    {
                        "name": "python/example",
                        "rel_path": "code/packages/python/example",
                        "language": "python",
                        "commands": [
                            {
                                "kind": "structured",
                                "program": "tools/conformance-probe",
                                "args": ["literal;not-shell", "$(still-data)"],
                            }
                        ],
                        "resource_locks": [],
                    }
                ],
                "dependency_edges": [],
            },
            "arguments": ["--fixture-data-only"],
            "environment": {"CONFORMANCE_MODE": "test"},
        },
        "expected": {
            "schema_version": 1,
            "case_id": "execution/structured-command",
            "domain": "execution",
            "outcome": "ok",
            "result": {
                "packages": [
                    {
                        "name": "python/example",
                        "status": "built",
                        "return_code": 0,
                        "commands": [
                            {
                                "index": 0,
                                "status": "succeeded",
                                "exit_code": 0,
                            }
                        ],
                    }
                ]
            },
            "diagnostics": [],
        },
        "limits": {
            "wall_time_ms": 1000,
            "output_bytes": 4096,
            "workspace_bytes": 4096,
            "process_count": 2,
            "memory_mib": 64,
            "cpu_time_ms": 1000,
        },
    }


class ExecutionSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.case_schema = bootstrap.load_document(FIXTURE_ROOT / "schema.json")
        cls.result_schema = bootstrap.load_document(FIXTURE_ROOT / "result.schema.json")
        cls.execution_schema = bootstrap.load_document(
            FIXTURE_ROOT / "execution.schema.json"
        )
        cls.policy_schema = bootstrap.load_document(
            FIXTURE_ROOT / "execution-policy.schema.json"
        )

    def assert_schema_error(
        self,
        instance: dict[str, object],
        schema: dict[str, object],
    ) -> None:
        self.assertTrue(bootstrap._schema_errors(instance, schema))

    def projection(self, case: dict[str, object]) -> dict[str, object]:
        expected = case["expected"]
        if not isinstance(expected, dict):
            self.fail("base execution case expected record is not an object")
        return {
            "domain": case["domain"],
            "outcome": expected["outcome"],
            "input": case["input"],
            "result": expected["result"],
        }

    def test_closed_execution_case_and_projection_validate(self) -> None:
        case = base_case()
        self.assertEqual(bootstrap._schema_errors(case, self.case_schema), [])
        self.assertEqual(
            bootstrap._schema_errors(self.projection(case), self.execution_schema),
            [],
        )
        self.assertEqual(
            bootstrap._schema_errors(case["expected"], self.result_schema),
            [],
        )
        execution.validate_execution_semantics(case)

    def test_unknown_execution_fields_are_rejected(self) -> None:
        case = base_case()
        options = case["input"]["options"]  # type: ignore[index]
        options["shell"] = "host"  # type: ignore[index]
        self.assert_schema_error(case, self.case_schema)
        self.assert_schema_error(self.projection(case), self.execution_schema)

        case = base_case()
        command = case["input"]["options"]["packages"][0]["commands"][0]  # type: ignore[index]
        command["cwd"] = "../../host"  # type: ignore[index]
        self.assert_schema_error(case, self.case_schema)
        self.assert_schema_error(self.projection(case), self.execution_schema)

    def test_empty_or_nul_legacy_commands_are_rejected(self) -> None:
        for line in ("", "printf ok\u0000printf escaped"):
            case = base_case()
            commands = case["input"]["options"]["packages"][0]["commands"]  # type: ignore[index]
            commands[0] = {"kind": "legacy", "line": line}  # type: ignore[index]
            with self.subTest(line=repr(line)):
                self.assert_schema_error(case, self.case_schema)

    def test_command_result_exit_code_matches_status(self) -> None:
        case = base_case()
        command = case["expected"]["result"]["packages"][0]["commands"][0]  # type: ignore[index]
        command["status"] = "not-run"  # type: ignore[index]
        self.assert_schema_error(case["expected"], self.result_schema)  # type: ignore[arg-type]

        command["exit_code"] = None  # type: ignore[index]
        package = case["expected"]["result"]["packages"][0]  # type: ignore[index]
        package["status"] = "would-build"  # type: ignore[index]
        package["return_code"] = None  # type: ignore[index]
        self.assertEqual(
            bootstrap._schema_errors(case["expected"], self.result_schema),  # type: ignore[arg-type]
            [],
        )

    def test_command_result_status_exit_code_matrix_is_closed(self) -> None:
        invalid_pairs = [
            ("succeeded", 1),
            ("succeeded", None),
            ("failed", 0),
            ("failed", None),
            ("not-run", 0),
        ]
        for status, exit_code in invalid_pairs:
            case = base_case()
            command = case["expected"]["result"]["packages"][0]["commands"][0]  # type: ignore[index]
            command["status"] = status  # type: ignore[index]
            command["exit_code"] = exit_code  # type: ignore[index]
            with self.subTest(status=status, exit_code=exit_code):
                self.assert_schema_error(case["expected"], self.result_schema)  # type: ignore[arg-type]
                self.assert_schema_error(
                    self.projection(case),
                    self.execution_schema,
                )

    def test_package_result_state_matrix_is_closed(self) -> None:
        mutations: list[tuple[str, dict[str, object]]] = []

        legacy = base_case()
        legacy["expected"]["result"]["packages"][0]["status"] = (  # type: ignore[index]
            "dependency-skipped"
        )
        mutations.append(("legacy-term", legacy))

        built_nonzero = base_case()
        built_nonzero["expected"]["result"]["packages"][0]["return_code"] = 1  # type: ignore[index]
        mutations.append(("built-nonzero", built_nonzero))

        built_not_run = base_case()
        built_command = built_not_run["expected"]["result"]["packages"][0]["commands"][
            0
        ]  # type: ignore[index]
        built_command["status"] = "not-run"  # type: ignore[index]
        built_command["exit_code"] = None  # type: ignore[index]
        mutations.append(("built-not-run", built_not_run))

        failed_without_failure = base_case()
        failed_package = failed_without_failure["expected"]["result"]["packages"][0]  # type: ignore[index]
        failed_package["status"] = "failed"  # type: ignore[index]
        failed_package["return_code"] = 7  # type: ignore[index]
        failed_without_failure["expected"]["outcome"] = "error"  # type: ignore[index]
        mutations.append(("failed-without-failed-command", failed_without_failure))

        for status in ("dep-skipped", "would-build"):
            executed = base_case()
            package = executed["expected"]["result"]["packages"][0]  # type: ignore[index]
            package["status"] = status  # type: ignore[index]
            package["return_code"] = None  # type: ignore[index]
            mutations.append((f"{status}-executed", executed))

        for name, mutated in mutations:
            with self.subTest(name=name):
                self.assert_schema_error(mutated["expected"], self.result_schema)  # type: ignore[arg-type]
                self.assert_schema_error(
                    self.projection(mutated),
                    self.execution_schema,
                )

    def test_valid_failure_and_dry_run_state_records(self) -> None:
        failed = base_case()
        package = failed["expected"]["result"]["packages"][0]  # type: ignore[index]
        package["status"] = "failed"  # type: ignore[index]
        package["return_code"] = 7  # type: ignore[index]
        command = package["commands"][0]  # type: ignore[index]
        command["status"] = "failed"  # type: ignore[index]
        command["exit_code"] = 7  # type: ignore[index]
        failed["expected"]["outcome"] = "error"  # type: ignore[index]
        self.assertEqual(
            bootstrap._schema_errors(failed["expected"], self.result_schema),  # type: ignore[arg-type]
            [],
        )
        self.assertEqual(
            bootstrap._schema_errors(self.projection(failed), self.execution_schema),
            [],
        )
        execution.validate_execution_semantics(failed)

        dry_run = base_case()
        dry_run["input"]["options"]["dry_run"] = True  # type: ignore[index]
        package = dry_run["expected"]["result"]["packages"][0]  # type: ignore[index]
        package["status"] = "would-build"  # type: ignore[index]
        package["return_code"] = None  # type: ignore[index]
        command = package["commands"][0]  # type: ignore[index]
        command["status"] = "not-run"  # type: ignore[index]
        command["exit_code"] = None  # type: ignore[index]
        self.assertEqual(
            bootstrap._schema_errors(dry_run["expected"], self.result_schema),  # type: ignore[arg-type]
            [],
        )
        self.assertEqual(
            bootstrap._schema_errors(self.projection(dry_run), self.execution_schema),
            [],
        )
        execution.validate_execution_semantics(dry_run)

    def test_projection_rejects_outcome_and_dry_run_status_conflicts(self) -> None:
        dry_run_built = base_case()
        dry_run_built["input"]["options"]["dry_run"] = True  # type: ignore[index]
        self.assert_schema_error(
            self.projection(dry_run_built),
            self.execution_schema,
        )

        ok_would_build = base_case()
        package = ok_would_build["expected"]["result"]["packages"][0]  # type: ignore[index]
        package["status"] = "would-build"  # type: ignore[index]
        package["return_code"] = None  # type: ignore[index]
        command = package["commands"][0]  # type: ignore[index]
        command["status"] = "not-run"  # type: ignore[index]
        command["exit_code"] = None  # type: ignore[index]
        self.assert_schema_error(
            self.projection(ok_would_build),
            self.execution_schema,
        )

        error_without_failure = base_case()
        error_without_failure["expected"]["outcome"] = "error"  # type: ignore[index]
        self.assert_schema_error(
            error_without_failure["expected"],  # type: ignore[arg-type]
            self.result_schema,
        )
        self.assert_schema_error(
            self.projection(error_without_failure),
            self.execution_schema,
        )

    def test_semantics_reject_failed_command_order_and_return_code_drift(self) -> None:
        out_of_order = base_case()
        input_commands = out_of_order["input"]["options"]["packages"][0]["commands"]  # type: ignore[index]
        input_commands.append(copy.deepcopy(input_commands[0]))  # type: ignore[attr-defined]
        package = out_of_order["expected"]["result"]["packages"][0]  # type: ignore[index]
        package["status"] = "failed"  # type: ignore[index]
        package["return_code"] = 7  # type: ignore[index]
        package["commands"] = [  # type: ignore[index]
            {"index": 0, "status": "not-run", "exit_code": None},
            {"index": 1, "status": "failed", "exit_code": 7},
        ]
        out_of_order["expected"]["outcome"] = "error"  # type: ignore[index]
        self.assertEqual(
            bootstrap._schema_errors(
                self.projection(out_of_order),
                self.execution_schema,
            ),
            [],
        )
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(out_of_order)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_COMMAND_STATE_ORDER_INVALID",
        )

        return_code = copy.deepcopy(out_of_order)
        commands = return_code["expected"]["result"]["packages"][0]["commands"]  # type: ignore[index]
        commands[0] = {"index": 0, "status": "succeeded", "exit_code": 0}  # type: ignore[index]
        return_code["expected"]["result"]["packages"][0]["return_code"] = 9  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(return_code)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_PACKAGE_RETURN_CODE_MISMATCH",
        )

    def test_semantics_enforce_dependency_failure_propagation(self) -> None:
        case = base_case()
        packages = case["input"]["options"]["packages"]  # type: ignore[index]
        dependent = copy.deepcopy(packages[0])  # type: ignore[index]
        dependent["name"] = "python/dependent"
        dependent["rel_path"] = "code/packages/python/dependent"
        packages.append(dependent)  # type: ignore[attr-defined]
        case["input"]["options"]["dependency_edges"] = [  # type: ignore[index]
            ["python/example", "python/dependent"]
        ]
        case["expected"]["outcome"] = "error"  # type: ignore[index]
        failed = case["expected"]["result"]["packages"][0]  # type: ignore[index]
        failed["status"] = "failed"  # type: ignore[index]
        failed["return_code"] = 7  # type: ignore[index]
        failed["commands"][0]["status"] = "failed"  # type: ignore[index]
        failed["commands"][0]["exit_code"] = 7  # type: ignore[index]
        skipped = copy.deepcopy(failed)
        skipped["name"] = "python/dependent"
        skipped["status"] = "dep-skipped"
        skipped["return_code"] = None
        skipped["commands"][0]["status"] = "not-run"
        skipped["commands"][0]["exit_code"] = None
        case["expected"]["result"]["packages"] = [skipped, failed]  # type: ignore[index]
        self.assertEqual(
            bootstrap._schema_errors(self.projection(case), self.execution_schema),
            [],
        )
        execution.validate_execution_semantics(case)

        dependent_built = copy.deepcopy(case)
        result = dependent_built["expected"]["result"]["packages"][0]  # type: ignore[index]
        result["status"] = "built"  # type: ignore[index]
        result["return_code"] = 0  # type: ignore[index]
        result["commands"][0]["status"] = "succeeded"  # type: ignore[index]
        result["commands"][0]["exit_code"] = 0  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(dependent_built)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_DEPENDENCY_STATE_MISMATCH",
        )

        unjustified_skip = copy.deepcopy(case)
        unjustified_skip["input"]["options"]["dependency_edges"] = []  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(unjustified_skip)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_DEPENDENCY_STATE_MISMATCH",
        )

    def test_semantics_reject_duplicate_identities_unknown_edges_and_cycles(
        self,
    ) -> None:
        duplicate = base_case()
        packages = duplicate["input"]["options"]["packages"]  # type: ignore[index]
        packages.append(copy.deepcopy(packages[0]))  # type: ignore[attr-defined]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(duplicate)
        self.assertEqual(raised.exception.code, "EXECUTION_PACKAGE_DUPLICATE")

        duplicate_result = base_case()
        result_packages = duplicate_result["expected"]["result"]["packages"]  # type: ignore[index]
        conflicting = copy.deepcopy(result_packages[0])  # type: ignore[index]
        conflicting["status"] = "failed"
        conflicting["return_code"] = 7
        conflicting["commands"][0]["status"] = "failed"
        conflicting["commands"][0]["exit_code"] = 7
        result_packages.append(conflicting)  # type: ignore[attr-defined]
        duplicate_result["expected"]["outcome"] = "error"  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(duplicate_result)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_RESULT_PACKAGE_DUPLICATE",
        )

        unknown = base_case()
        unknown["input"]["options"]["dependency_edges"] = [  # type: ignore[index]
            ["python/missing", "python/example"]
        ]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(unknown)
        self.assertEqual(raised.exception.code, "EXECUTION_EDGE_UNKNOWN")

        cyclic = base_case()
        packages = cyclic["input"]["options"]["packages"]  # type: ignore[index]
        second = copy.deepcopy(packages[0])  # type: ignore[index]
        second["name"] = "python/second"
        second["rel_path"] = "code/packages/python/second"
        packages.append(second)  # type: ignore[attr-defined]
        cyclic["input"]["options"]["dependency_edges"] = [  # type: ignore[index]
            ["python/example", "python/second"],
            ["python/second", "python/example"],
        ]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(cyclic)
        self.assertEqual(raised.exception.code, "EXECUTION_GRAPH_CYCLE")

    def test_semantics_reject_platform_and_process_limit_mismatch(self) -> None:
        case = base_case()
        case["input"]["options"]["platform"] = "windows"  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(case)
        self.assertEqual(raised.exception.code, "EXECUTION_PLATFORM_MISMATCH")

        case = base_case()
        case["input"]["options"]["jobs"] = 3  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(case)
        self.assertEqual(raised.exception.code, "EXECUTION_JOB_LIMIT")

    def test_checked_in_policy_is_closed_disabled_and_complete(self) -> None:
        policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
        self.assertEqual(bootstrap._schema_errors(policy, self.policy_schema), [])
        summary = execution.validate_policy_semantics(policy)
        self.assertFalse(policy["enabled"])
        self.assertEqual(summary["ready_backend_count"], 0)
        self.assertEqual(summary["adapter_count"], 0)
        self.assertEqual(
            [item["platform"] for item in policy["backends"]],
            ["darwin", "linux", "windows"],
        )

    def test_corpus_digest_is_framed_and_detects_path_or_content_mutation(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            self.assertEqual(
                execution.execution_corpus_digest(root),
                "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855",
            )
            (root / "a.json").write_bytes(b"{}")
            first = execution.execution_corpus_digest(root)
            (root / "a.json").write_bytes(b'{ "changed": true }')
            self.assertNotEqual(execution.execution_corpus_digest(root), first)
            (root / "a.json").rename(root / "b.json")
            self.assertNotEqual(
                execution.execution_corpus_digest(root),
                execution.framed_corpus_digest([("a.json", b'{ "changed": true }')]),
            )

    def test_corpus_snapshot_retains_exact_hashed_bytes_after_path_mutation(
        self,
    ) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            case_path = root / "selected.json"
            original = b'{"value":"approved"}'
            case_path.write_bytes(original)

            snapshot = execution.capture_execution_case_snapshot(root)
            expected_digest = execution.framed_corpus_digest(
                [("selected.json", original)]
            )
            self.assertEqual(snapshot.corpus_sha256, expected_digest)
            self.assertEqual(len(snapshot.members), 1)
            self.assertEqual(
                snapshot.members[0].relative_path,
                "selected.json",
            )
            self.assertEqual(snapshot.members[0].raw, original)

            case_path.write_bytes(b'{"value":"changed-after-capture"}')
            case_path.rename(root / "renamed.json")
            selection = snapshot.select("selected.json")
            self.assertEqual(selection.relative_path, "selected.json")
            self.assertEqual(selection.corpus_sha256, expected_digest)
            self.assertEqual(selection.raw, original)

            with self.assertRaises(bootstrap.ConformanceError) as raised:
                snapshot.select("renamed.json")
            self.assertEqual(raised.exception.code, "EXECUTION_CASE_NOT_FOUND")

    def test_corpus_snapshot_canonicalizes_filesystem_enumeration_order(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "z-last.json").write_bytes(b"z")
            (root / "a-first.json").write_bytes(b"a")
            snapshot = execution.capture_execution_case_snapshot(root)
            self.assertEqual(
                tuple(member.relative_path for member in snapshot.members),
                ("a-first.json", "z-last.json"),
            )
            self.assertEqual(
                execution._execution_corpus_entries(root),
                [("a-first.json", b"a"), ("z-last.json", b"z")],
            )
            self.assertEqual(
                execution._read_raw_regular(root / "a-first.json"),
                b"a",
            )

    def test_corpus_snapshot_rejects_invalid_limits_names_and_roots(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "oversized.json").write_bytes(b"12")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(root, max_bytes=1)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_FILE_TOO_LARGE",
            )

            for invalid_limit in (-1, True, 1.5):
                with self.subTest(invalid_limit=invalid_limit):
                    with self.assertRaises(bootstrap.ConformanceError) as raised:
                        execution.capture_execution_case_snapshot(  # type: ignore[arg-type]
                            root,
                            max_bytes=invalid_limit,
                        )
                    self.assertEqual(
                        raised.exception.code,
                        "EXECUTION_CORPUS_LIMIT_INVALID",
                    )
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(
                    root,
                    max_bytes=execution.MAX_EXECUTION_CASE_BYTES + 1,
                )
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_LIMIT_INVALID",
            )

        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.snapshot_from_entries([("case\ud800.json", b"{}")])
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_PATH_UNSAFE",
        )

        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution._validated_execution_case_names(["Case.json", "case.json"])
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_PATH_DUPLICATE",
        )
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution._validated_execution_case_names(["Straße.json", "strasse.json"])
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_PATH_DUPLICATE",
        )

        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution._lexical_absolute_case_root(Path("child/../cases"))
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
        )

    def test_corpus_snapshot_selector_rejects_outside_and_alias_names(self) -> None:
        snapshot = execution.snapshot_from_entries([("Case.json", b"{}")])

        for unsafe in (
            "../Case.json",
            "nested/Case.json",
            r"nested\Case.json",
            "Case.JSON",
            "C:Case.json",
            "",
        ):
            with self.subTest(unsafe=unsafe):
                with self.assertRaises(bootstrap.ConformanceError) as raised:
                    snapshot.select(unsafe)
                self.assertEqual(
                    raised.exception.code,
                    "EXECUTION_CASE_SELECTOR_UNSAFE",
                )

        with self.assertRaises(bootstrap.ConformanceError) as raised:
            snapshot.select("case.json")
        self.assertEqual(raised.exception.code, "EXECUTION_CASE_SELECTOR_ALIAS")

        unicode_snapshot = execution.snapshot_from_entries([("é.json", b"{}")])
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            unicode_snapshot.select("e\u0301.json")
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CASE_SELECTOR_ALIAS",
        )

    def test_corpus_snapshot_rejects_casefold_aliases_and_non_bytes(self) -> None:
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.snapshot_from_entries(
                [("Case.json", b"{}"), ("case.json", b"{}")]
            )
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_PATH_DUPLICATE",
        )

        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.snapshot_from_entries([("case.json", bytearray(b"{}"))])  # type: ignore[list-item]
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_BYTES_INVALID",
        )

    def test_corpus_snapshot_factories_enforce_closed_types_and_aggregate_limits(
        self,
    ) -> None:
        for constructor, arguments in (
            (
                execution.ExecutionCaseMember,
                {"relative_path": "case.json", "raw": b"forged"},
            ),
            (
                execution.ExecutionCaseSnapshot,
                {"members": ()},
            ),
            (
                execution.ExecutionCaseSelection,
                {
                    "relative_path": "case.json",
                    "corpus_sha256": "0" * 64,
                    "raw": b"forged",
                },
            ),
        ):
            with (
                self.subTest(constructor=constructor.__name__),
                self.assertRaises(TypeError),
            ):
                constructor(**arguments)

        too_many = (
            (f"case-{index:03d}.json", b"")
            for index in range(execution.MAX_EXECUTION_CORPUS_MEMBERS + 1)
        )
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.snapshot_from_entries(too_many)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_MEMBER_LIMIT_EXCEEDED",
        )

        with (
            mock.patch.object(
                execution,
                "MAX_EXECUTION_CASE_BYTES",
                3,
            ),
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            execution.snapshot_from_entries([("case.json", b"1234")])
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_FILE_TOO_LARGE",
        )

        with (
            mock.patch.object(
                execution,
                "MAX_EXECUTION_CORPUS_TOTAL_BYTES",
                3,
            ),
            self.assertRaises(bootstrap.ConformanceError) as raised,
        ):
            execution.snapshot_from_entries([("a.json", b"12"), ("b.json", b"34")])
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_AGGREGATE_TOO_LARGE",
        )

        too_many_directory_entries = (
            f"ignored-{index}.txt"
            for index in range(execution.MAX_EXECUTION_DIRECTORY_ENTRIES + 1)
        )
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution._validated_execution_case_names(too_many_directory_entries)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_CORPUS_ENUMERATION_LIMIT_EXCEEDED",
        )

    def test_corpus_snapshot_detects_file_and_directory_change_races(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            case_path = root / "case.json"
            case_path.write_bytes(b"{}")
            before = case_path.stat()
            changed = os.stat_result(
                (
                    before.st_mode,
                    before.st_ino,
                    before.st_dev,
                    before.st_nlink,
                    before.st_uid,
                    before.st_gid,
                    before.st_size,
                    before.st_atime,
                    before.st_mtime + 1,
                    before.st_ctime,
                )
            )
            with (
                mock.patch.object(
                    execution.os,
                    "fstat",
                    side_effect=[before, changed],
                ),
                self.assertRaises(bootstrap.ConformanceError) as raised,
            ):
                execution._read_raw_regular_bound(case_path)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_FILE_CHANGED",
            )

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "case.json").write_bytes(b"{}")
            if os.name == "nt":
                original_reader = execution._read_raw_regular_bound

                def mutate_windows(
                    path: Path,
                    *,
                    max_bytes: int,
                ) -> tuple[bytes, os.stat_result]:
                    result = original_reader(path, max_bytes=max_bytes)
                    (root / "added.json").write_bytes(b"{}")
                    return result

                patcher = mock.patch.object(
                    execution,
                    "_read_raw_regular_bound",
                    side_effect=mutate_windows,
                )
            else:
                original_reader = execution._read_posix_snapshot_member

                def mutate_posix(
                    root_descriptor: int,
                    relative_path: str,
                    *,
                    max_bytes: int,
                ) -> tuple[bytes, tuple[int, int]]:
                    result = original_reader(
                        root_descriptor,
                        relative_path,
                        max_bytes=max_bytes,
                    )
                    (root / "added.json").write_bytes(b"{}")
                    return result

                patcher = mock.patch.object(
                    execution,
                    "_read_posix_snapshot_member",
                    side_effect=mutate_posix,
                )
            with (
                patcher,
                self.assertRaises(bootstrap.ConformanceError) as raised,
            ):
                execution.capture_execution_case_snapshot(root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_CHANGED",
            )

    def test_corpus_digest_rejects_unsafe_duplicate_and_missing_inputs(self) -> None:
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.framed_corpus_digest([("../escape.json", b"{}")])
        self.assertEqual(raised.exception.code, "EXECUTION_CORPUS_PATH_UNSAFE")

        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.framed_corpus_digest([("same.json", b"{}"), ("same.json", b"{}")])
        self.assertEqual(raised.exception.code, "EXECUTION_CORPUS_PATH_DUPLICATE")

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            missing = root / "missing"
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.execution_corpus_digest(missing)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_DIRECTORY_MISSING",
            )
            regular = root / "regular"
            regular.write_text("not a directory", encoding="utf-8")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.execution_corpus_digest(regular)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
            )

    def test_corpus_snapshot_rejects_links_and_identity_aliases(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            original = root / "original.json"
            original.write_bytes(b"{}")
            linked = root / "linked.json"
            try:
                os.link(original, linked)
            except (NotImplementedError, OSError) as error:
                self.skipTest(f"hard links unavailable: {error}")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_FILE_INVALID",
            )

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target"
            target.mkdir()
            linked_root = root / "linked-root"
            try:
                linked_root.symlink_to(target, target_is_directory=True)
            except (NotImplementedError, OSError) as error:
                self.skipTest(f"directory symlinks unavailable: {error}")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(linked_root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
            )

    def test_corpus_snapshot_rejects_symlink_and_nonregular_json_members(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            (root / "directory.json").mkdir()
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_FILE_INVALID",
            )

    def test_corpus_snapshot_rejects_linked_intermediate_root_component(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target_parent = root / "target-parent"
            case_root = target_parent / "execution-cases"
            case_root.mkdir(parents=True)
            linked_parent = root / "linked-parent"
            try:
                linked_parent.symlink_to(target_parent, target_is_directory=True)
            except (NotImplementedError, OSError) as error:
                self.skipTest(f"directory symlinks unavailable: {error}")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(
                    linked_parent / "execution-cases"
                )
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
            )

        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target"
            target.write_bytes(b"{}")
            linked = root / "linked.json"
            try:
                linked.symlink_to(target)
            except (NotImplementedError, OSError) as error:
                self.skipTest(f"file symlinks unavailable: {error}")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.capture_execution_case_snapshot(root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_FILE_INVALID",
            )

    def test_contract_rejects_linked_fixture_root_ancestor(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            target = root / "target"
            shutil.copytree(FIXTURE_ROOT, target)
            linked_root = root / "linked-fixture"
            try:
                linked_root.symlink_to(target, target_is_directory=True)
            except (NotImplementedError, OSError) as error:
                self.skipTest(f"directory symlinks unavailable: {error}")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.validate_contract(linked_root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_DIRECTORY_INVALID",
            )

    def test_corpus_reader_rejects_oversized_and_nonregular_members(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oversized = root / "oversized.json"
            oversized.write_bytes(b"12345")
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution._read_raw_regular(oversized, max_bytes=4)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_CORPUS_FILE_TOO_LARGE",
            )
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution._read_raw_regular(root)
            self.assertIn(
                raised.exception.code,
                {
                    "EXECUTION_CORPUS_FILE_INVALID",
                    "EXECUTION_CORPUS_READ_FAILED",
                },
            )

    def test_policy_semantics_reject_invalid_platform_and_adapter_identities(
        self,
    ) -> None:
        policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
        mutations = []

        unsorted = copy.deepcopy(policy)
        unsorted["backends"].reverse()
        mutations.append((unsorted, "EXECUTION_BACKENDS_NOT_CANONICAL"))

        duplicate = copy.deepcopy(policy)
        duplicate["backends"][1]["platform"] = "darwin"
        mutations.append((duplicate, "EXECUTION_BACKENDS_NOT_CANONICAL"))

        kind = copy.deepcopy(policy)
        kind["backends"][1]["kind"] = "macos_isolated"
        mutations.append((kind, "EXECUTION_BACKEND_KIND_MISMATCH"))

        adapter = {
            "language": "go",
            "platform": "linux",
            "executable": "code/programs/go/build-tool",
            "executable_sha256": "1" * 64,
        }
        adapters = copy.deepcopy(policy)
        adapters["adapters"] = [adapter, copy.deepcopy(adapter)]
        mutations.append((adapters, "EXECUTION_ADAPTER_DUPLICATE"))

        unsafe = copy.deepcopy(policy)
        unsafe_adapter = copy.deepcopy(adapter)
        unsafe_adapter["executable"] = "../host"
        unsafe["adapters"] = [unsafe_adapter]
        mutations.append((unsafe, "EXECUTION_ADAPTER_PATH_UNSAFE"))

        for mutated, code in mutations:
            with (
                self.subTest(code=code),
                self.assertRaises(bootstrap.ConformanceError) as raised,
            ):
                execution.validate_policy_semantics(mutated)
            self.assertEqual(raised.exception.code, code)

    def test_execution_semantics_reject_invalid_envelope_and_package_records(
        self,
    ) -> None:
        mutations = []

        domain = base_case()
        domain["domain"] = "graph"
        mutations.append((domain, "EXECUTION_DOMAIN_INVALID"))

        operation = base_case()
        operation["input"]["operation"] = "graph"  # type: ignore[index]
        mutations.append((operation, "EXECUTION_OPERATION_INVALID"))

        capability = base_case()
        capability["capabilities"] = ["execution"]
        mutations.append((capability, "EXECUTION_CAPABILITY_MISSING"))

        identity = base_case()
        identity["expected"]["case_id"] = "execution/other"  # type: ignore[index]
        mutations.append((identity, "EXECUTION_IDENTITY_MISMATCH"))

        packages = base_case()
        packages["input"]["options"]["packages"] = None  # type: ignore[index]
        mutations.append((packages, "EXECUTION_PACKAGES_INVALID"))

        unsafe_path = base_case()
        unsafe_path["input"]["options"]["packages"][0]["rel_path"] = "../host"  # type: ignore[index]
        mutations.append((unsafe_path, "EXECUTION_PACKAGE_PATH_UNSAFE"))

        path_collision = base_case()
        package_values = path_collision["input"]["options"]["packages"]  # type: ignore[index]
        second = copy.deepcopy(package_values[0])  # type: ignore[index]
        second["name"] = "python/second"
        second["rel_path"] = "CODE/packages/python/example"
        package_values.append(second)  # type: ignore[attr-defined]
        mutations.append((path_collision, "EXECUTION_PACKAGE_PATH_DUPLICATE"))

        locks = base_case()
        locks["input"]["options"]["packages"][0]["resource_locks"] = [  # type: ignore[index]
            "z",
            "a",
        ]
        mutations.append((locks, "EXECUTION_LOCKS_NOT_CANONICAL"))

        edges = base_case()
        edges["input"]["options"]["dependency_edges"] = None  # type: ignore[index]
        mutations.append((edges, "EXECUTION_EDGES_INVALID"))

        for mutated, code in mutations:
            with (
                self.subTest(code=code),
                self.assertRaises(bootstrap.ConformanceError) as raised,
            ):
                execution.validate_execution_semantics(mutated)
            self.assertEqual(raised.exception.code, code)

    def test_execution_semantics_reject_noncanonical_or_incomplete_results(
        self,
    ) -> None:
        unsorted = base_case()
        packages = unsorted["input"]["options"]["packages"]  # type: ignore[index]
        second = copy.deepcopy(packages[0])  # type: ignore[index]
        second["name"] = "python/aaa"
        second["rel_path"] = "code/packages/python/aaa"
        packages.append(second)  # type: ignore[attr-defined]
        result_packages = unsorted["expected"]["result"]["packages"]  # type: ignore[index]
        second_result = copy.deepcopy(result_packages[0])  # type: ignore[index]
        second_result["name"] = "python/aaa"
        result_packages.append(second_result)  # type: ignore[attr-defined]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(unsorted)
        self.assertEqual(raised.exception.code, "EXECUTION_RESULT_NOT_CANONICAL")

        missing = base_case()
        missing["expected"]["result"]["packages"] = []  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(missing)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_RESULT_PACKAGE_MISMATCH",
        )

        index = base_case()
        index["expected"]["result"]["packages"][0]["commands"][0]["index"] = 1  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(index)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_COMMAND_INDEX_INVALID",
        )

        count = base_case()
        count["expected"]["result"]["packages"][0]["commands"] = []  # type: ignore[index]
        with self.assertRaises(bootstrap.ConformanceError) as raised:
            execution.validate_execution_semantics(count)
        self.assertEqual(
            raised.exception.code,
            "EXECUTION_COMMAND_COUNT_MISMATCH",
        )

    def test_contract_validates_one_inert_case_and_detects_policy_drift(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            for name in (
                "schema.json",
                "result.schema.json",
                "execution.schema.json",
                "execution-policy.schema.json",
                "execution-authority.schema.json",
                "linux-oci-backend.schema.json",
            ):
                shutil.copyfile(FIXTURE_ROOT / name, root / name)
            case_root = root / "execution-cases"
            case_root.mkdir()
            case = base_case()
            (case_root / "structured.json").write_text(
                json.dumps(case, sort_keys=True),
                encoding="utf-8",
            )
            policy = bootstrap.load_document(FIXTURE_ROOT / "execution-policy.json")
            policy["execution_corpus_sha256"] = execution.execution_corpus_digest(
                case_root
            )
            (root / "execution-policy.json").write_text(
                json.dumps(policy, sort_keys=True),
                encoding="utf-8",
            )
            summary = execution.validate_contract(root)
            self.assertEqual(summary["execution_case_count"], 1)

            policy["execution_corpus_sha256"] = "0" * 64
            (root / "execution-policy.json").write_text(
                json.dumps(policy, sort_keys=True),
                encoding="utf-8",
            )
            with self.assertRaises(bootstrap.ConformanceError) as raised:
                execution.validate_contract(root)
            self.assertEqual(
                raised.exception.code,
                "EXECUTION_POLICY_CORPUS_MISMATCH",
            )


if __name__ == "__main__":
    unittest.main()
