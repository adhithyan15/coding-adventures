from __future__ import annotations

import copy
import json
import shutil
import sys
import tempfile
import unittest
from pathlib import Path

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
        assert isinstance(expected, dict)
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
        self.assertEqual(
            bootstrap._schema_errors(case["expected"], self.result_schema),  # type: ignore[arg-type]
            [],
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
