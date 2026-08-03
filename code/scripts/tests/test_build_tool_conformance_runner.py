from __future__ import annotations

import copy
import io
import json
import os
import subprocess
import sys
import tempfile
import unittest
from contextlib import redirect_stderr, redirect_stdout
from pathlib import Path
from unittest import mock


SCRIPTS_DIR = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(SCRIPTS_DIR))

import build_tool_conformance as runner  # noqa: E402


FIXTURE_ROOT = runner.DEFAULT_FIXTURE_ROOT
CASES_ROOT = FIXTURE_ROOT / "cases"


def load_case(name: str) -> dict[str, object]:
    return runner.load_document(CASES_ROOT / name)


class StrictJsonTests(unittest.TestCase):
    def assert_parse_error(self, raw: bytes, code: str) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.strict_load_bytes(raw)
        self.assertEqual(raised.exception.code, code)

    def test_rejects_ambiguous_and_nonportable_json(self) -> None:
        cases = {
            b'{"domain":"discovery","domain":"execution"}': "JSON_DUPLICATE_KEY",
            b'{"value":NaN}': "JSON_NON_FINITE",
            b'{"value":1.5}': "JSON_FLOAT_FORBIDDEN",
            b'{"value":9007199254740992}': "JSON_INTEGER_RANGE",
            b'{"value":"\\ud800"}': "JSON_UNICODE_SURROGATE",
            b"\xef\xbb\xbf{}": "JSON_BOM_FORBIDDEN",
        }
        for raw, code in cases.items():
            with self.subTest(code=code):
                self.assert_parse_error(raw, code)

    def test_rejects_invalid_utf8_syntax_delimiters_and_top_level_values(
        self,
    ) -> None:
        cases = {
            b'{"value":"\xff"}': "JSON_UTF8_INVALID",
            b"}": "JSON_SYNTAX_INVALID",
            b'{"value":': "JSON_SYNTAX_INVALID",
            b"[]": "JSON_TOP_LEVEL_INVALID",
        }
        for raw, code in cases.items():
            with self.subTest(code=code):
                self.assert_parse_error(raw, code)

    def test_rejects_moderate_and_extreme_depth_with_the_same_code(self) -> None:
        for depth in (65, 1100):
            raw = b'{"value":' + (b"[" * depth) + b"0" + (b"]" * depth) + b"}"
            with self.subTest(depth=depth):
                self.assert_parse_error(raw, "JSON_DEPTH_EXCEEDED")

    def test_rejects_oversized_input_before_decoding(self) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.strict_load_bytes(b'{"value":"oversized"}', max_bytes=8)
        self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")

    def test_load_document_reports_missing_files(self) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.load_document(Path("definitely-not-present.json"))
        self.assertEqual(raised.exception.code, "DOCUMENT_READ_FAILED")

    def test_load_document_bounds_the_file_read_and_rejects_links(self) -> None:
        with tempfile.TemporaryDirectory() as directory:
            root = Path(directory)
            oversized = root / "oversized.json"
            oversized.write_bytes(b'{"value":"' + (b"x" * 100) + b'"}')
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.load_document(oversized, max_bytes=16)
            self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")

            target = root / "target.json"
            target.write_text("{}", encoding="utf-8")
            link = root / "link.json"
            try:
                link.symlink_to(target)
            except OSError:
                return
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.load_document(link)
            self.assertIn(
                raised.exception.code,
                {"DOCUMENT_READ_FAILED", "DOCUMENT_TYPE_INVALID"},
            )

    def test_portable_path_validation_covers_canonical_edge_cases(self) -> None:
        self.assertIsNotNone(runner.portable_path_error(None))
        self.assertIsNotNone(runner.portable_path_error("a" * 513))
        self.assertIsNotNone(runner.portable_path_error("fixtures/e\u0301.txt"))
        self.assertIsNotNone(runner.portable_path_error("fixtures/name."))
        self.assertIsNotNone(runner.portable_path_error("fixtures/COM¹.txt"))
        self.assertIsNotNone(runner.portable_path_error("fixtures/LPT².txt"))
        self.assertIsNone(runner.portable_path_error("fixtures/.hidden"))
        self.assertIsNone(runner.portable_glob_error("src/foo.*"))
        self.assertIsNone(runner.portable_glob_error("src/*.*"))
        self.assertIsNotNone(runner.portable_glob_error("src/foo."))

    def test_schema_validation_never_retrieves_external_references(self) -> None:
        for keyword in ("$ref", "$dynamicRef"):
            schema = {
                "$schema": "https://json-schema.org/draft/2020-12/schema",
                keyword: "http://127.0.0.1:9/schema.json",
            }
            with (
                self.subTest(keyword=keyword),
                mock.patch("urllib.request.urlopen") as retrieve,
                self.assertRaises(runner.ConformanceError) as raised,
            ):
                runner._schema_errors({}, schema)
            self.assertEqual(raised.exception.code, "SCHEMA_REFERENCE_FORBIDDEN")
            retrieve.assert_not_called()


class CorpusTests(unittest.TestCase):
    def test_checked_in_corpus_and_manifest_validate(self) -> None:
        summary = runner.validate_corpus(FIXTURE_ROOT)

        self.assertEqual(summary["schema_version"], 1)
        self.assertEqual(summary["case_count"], 36)
        self.assertEqual(summary["implementation_count"], 16)
        self.assertEqual(summary["established_languages"], 15)
        self.assertEqual(summary["execution_case_count"], 0)
        self.assertEqual(summary["front_door_count"], 12)
        self.assertEqual(summary["adapter_ready_count"], 0)
        self.assertEqual(summary["conformance_run_count"], 0)
        self.assertEqual(summary["conformance_status"], "not-run")
        self.assertEqual(
            summary["domains"],
            [
                "cli",
                "diff_selection",
                "discovery",
                "graph",
                "hashing_cache",
                "plan",
                "resolution",
                "sharding",
                "starlark",
                "toolchain_detection",
                "validation",
            ],
        )

    def test_manifest_classifies_every_established_front_door(self) -> None:
        manifest = runner.load_document(FIXTURE_ROOT / "implementations.json")
        implementations = {
            item["language"]: item for item in manifest["implementations"]
        }
        established = {
            language
            for language, item in implementations.items()
            if item["lane_status"] == "established"
        }
        present = {
            language
            for language, item in implementations.items()
            if item["implementation_status"] in {"present", "shared_engine"}
        }
        missing = {
            language
            for language, item in implementations.items()
            if item["implementation_status"] == "missing"
        }

        self.assertEqual(established, set(runner.ESTABLISHED_LANGUAGES))
        self.assertEqual(
            present,
            {
                "csharp",
                "elixir",
                "fsharp",
                "go",
                "haskell",
                "lua",
                "perl",
                "python",
                "ruby",
                "rust",
                "swift",
                "typescript",
            },
        )
        self.assertEqual(missing, {"dart", "java", "kotlin", "ocaml"})
        self.assertEqual(implementations["fsharp"]["shared_engine"], "csharp")
        self.assertEqual(implementations["ocaml"]["lane_status"], "emerging")

    def test_expected_results_are_checked_in_canonical_order(self) -> None:
        for case_path in sorted(CASES_ROOT.glob("*.json")):
            case = runner.load_document(case_path)
            self.assertEqual(
                case["expected"],
                runner.canonicalize_result(case["expected"]),
                case_path.name,
            )

    def test_every_process_free_domain_is_bootstrap_modeled(self) -> None:
        self.assertEqual(
            runner.BOOTSTRAP_DOMAINS,
            set(runner.DOMAIN_CAPABILITIES) - {"execution"},
        )

    def test_malformed_capabilities_fail_schema_validation_not_routing(self) -> None:
        case = load_case("discovery-simple.json")
        case["capabilities"] = [{"execution": False}]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(
                case,
                case_schema=runner.load_document(FIXTURE_ROOT / "schema.json"),
                result_schema=runner.load_document(FIXTURE_ROOT / "result.schema.json"),
                plan_schema=runner.load_document(
                    runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
                ),
            )
        self.assertEqual(raised.exception.code, "CASE_SCHEMA_INVALID")


class ExecutionDenialTests(unittest.TestCase):
    def assert_denied_before_side_effects(self, case: dict[str, object]) -> None:
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(runner.base64, "b64decode") as decode,
            mock.patch.object(os, "chmod") as chmod,
            mock.patch.object(subprocess, "run") as process,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(case)

        self.assertEqual(raised.exception.code, "EXECUTION_DISABLED")
        temporary.assert_not_called()
        decode.assert_not_called()
        chmod.assert_not_called()
        process.assert_not_called()

    def test_execution_intent_is_denied_in_every_routing_field(self) -> None:
        base = load_case("discovery-windows-override.json")

        domain = copy.deepcopy(base)
        domain["domain"] = "execution"
        self.assert_denied_before_side_effects(domain)

        operation = copy.deepcopy(base)
        operation["input"]["operation"] = "execution"
        self.assert_denied_before_side_effects(operation)

        execution_capability = copy.deepcopy(base)
        execution_capability["capabilities"].append("execution")
        self.assert_denied_before_side_effects(execution_capability)

        trusted_capability = copy.deepcopy(base)
        trusted_capability["capabilities"].append("trusted_execution")
        self.assert_denied_before_side_effects(trusted_capability)

    def test_validate_result_rejects_execution_before_reading_the_result(
        self,
    ) -> None:
        case = load_case("discovery-windows-override.json")
        case["domain"] = "execution"
        with tempfile.TemporaryDirectory() as directory:
            case_path = Path(directory) / "case.json"
            case_path.write_text(json.dumps(case), encoding="utf-8")
            missing_result = Path(directory) / "missing-result.json"
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_result_files(case_path, missing_result)
        self.assertEqual(raised.exception.code, "EXECUTION_DISABLED")

    def test_cli_execution_intent_is_denied_before_workspace_decoding(
        self,
    ) -> None:
        case = load_case("cli-dry-run-success.json")
        case["input"]["options"]["requires_execution"] = True
        case["workspace"]["files"] = [
            {
                "path": "fixtures/invalid.bin",
                "content_base64": "not base64!",
            }
        ]
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(runner.base64, "b64decode") as decode,
            mock.patch.object(subprocess, "run") as process,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(case)
        self.assertEqual(raised.exception.code, "EXECUTION_DISABLED")
        temporary.assert_not_called()
        decode.assert_not_called()
        process.assert_not_called()


class WorkspacePreflightTests(unittest.TestCase):
    def test_decodes_exact_files_without_creating_a_workspace(self) -> None:
        case = load_case("discovery-simple.json")
        case["workspace"]["files"].append(
            {
                "path": "fixtures/space and & metacharacters.bin",
                "content_base64": "AAEC/w==",
            }
        )

        with mock.patch.object(
            tempfile,
            "TemporaryDirectory",
        ) as temporary:
            staged = {
                entry.path: entry.content for entry in runner.preflight_workspace(case)
            }

        temporary.assert_not_called()
        self.assertEqual(
            staged["code/packages/python/demo/BUILD"],
            b"python -m unittest discover tests\n",
        )
        self.assertEqual(
            staged["fixtures/space and & metacharacters.bin"],
            b"\x00\x01\x02\xff",
        )

    def test_invalid_base64_and_workspace_limit_fail_before_root_creation(self) -> None:
        invalid = load_case("discovery-simple.json")
        invalid["workspace"]["files"][0] = {
            "path": "code/packages/python/demo/BUILD",
            "content_base64": "AB==",
        }
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(invalid)
        self.assertEqual(raised.exception.code, "WORKSPACE_BASE64_NONCANONICAL")
        temporary.assert_not_called()

        oversized = load_case("discovery-simple.json")
        oversized["limits"]["workspace_bytes"] = 1
        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.preflight_workspace(oversized)
        self.assertEqual(raised.exception.code, "WORKSPACE_BYTE_LIMIT")
        temporary.assert_not_called()

        malformed = load_case("discovery-simple.json")
        malformed["workspace"]["files"][0] = {
            "path": "code/packages/python/demo/BUILD",
            "content_base64": "not base64!",
        }
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.preflight_workspace(malformed)
        self.assertEqual(raised.exception.code, "WORKSPACE_BASE64_INVALID")

    def test_malformed_workspace_shapes_are_rejected(self) -> None:
        base = load_case("discovery-simple.json")
        for workspace, code in (
            ({}, "WORKSPACE_FILES_INVALID"),
            ({"files": ["not-an-object"]}, "WORKSPACE_FILE_INVALID"),
            (
                {"files": [{"path": "fixtures/no-content"}]},
                "WORKSPACE_CONTENT_MISSING",
            ),
        ):
            case = copy.deepcopy(base)
            case["workspace"] = workspace
            with self.subTest(code=code):
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.preflight_workspace(case)
                self.assertEqual(raised.exception.code, code)

    def test_path_aliases_collisions_and_prefix_conflicts_fail_preflight(self) -> None:
        base = load_case("discovery-simple.json")
        unsafe_paths = (
            "/absolute",
            "C:/drive",
            "//server/share",
            "../escape",
            "fixtures/CONIN$.txt",
            "fixtures/CONOUT$.txt",
            "fixtures/CLOCK$.txt",
        )
        for unsafe_path in unsafe_paths:
            case = copy.deepcopy(base)
            case["workspace"]["files"][0]["path"] = unsafe_path
            with self.subTest(path=unsafe_path):
                with (
                    mock.patch.object(
                        tempfile,
                        "TemporaryDirectory",
                    ) as temporary,
                    self.assertRaises(runner.ConformanceError) as raised,
                ):
                    runner.preflight_workspace(case)
                self.assertEqual(raised.exception.code, "WORKSPACE_PATH_UNSAFE")
                temporary.assert_not_called()

        collision = copy.deepcopy(base)
        collision["workspace"]["files"].append(
            {
                "path": "CODE/PACKAGES/PYTHON/DEMO/build",
                "content_utf8": "collision\n",
            }
        )
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.preflight_workspace(collision)
        self.assertEqual(raised.exception.code, "WORKSPACE_PATH_COLLISION")

        prefix = copy.deepcopy(base)
        prefix["workspace"]["files"] = [
            {"path": "fixtures/data", "content_utf8": "file\n"},
            {"path": "fixtures/data/child", "content_utf8": "child\n"},
        ]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.preflight_workspace(prefix)
        self.assertEqual(raised.exception.code, "WORKSPACE_PATH_PREFIX_CONFLICT")


class ResultValidationTests(unittest.TestCase):
    def test_domain_result_schema_rejects_field_name_drift(self) -> None:
        discovery = load_case("discovery-simple.json")
        typo = copy.deepcopy(discovery["expected"])
        package = typo["result"]["packages"][0]
        package["buildfile"] = package.pop("build_file")
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(discovery, typo)
        self.assertEqual(raised.exception.code, "RESULT_SCHEMA_INVALID")

        toolchains = load_case("toolchain-detection-shared.json")
        incomplete = copy.deepcopy(toolchains["expected"])
        del incomplete["result"]["toolchains"]["ocaml"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(toolchains, incomplete)
        self.assertEqual(
            raised.exception.code,
            "RESULT_PURE_SCHEMA_INVALID",
        )

        graph = load_case("graph-diamond.json")
        extra = copy.deepcopy(graph["expected"])
        extra["result"]["unexpected"] = []
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(graph, extra)
        self.assertEqual(raised.exception.code, "RESULT_SCHEMA_INVALID")

    def test_domain_aware_canonicalization_accepts_set_order_variation(self) -> None:
        case = load_case("graph-diamond.json")
        actual = copy.deepcopy(case["expected"])
        actual["result"]["edges"].reverse()
        actual["result"]["levels"][1].reverse()

        canonical = runner.assert_result_matches(case, actual)

        self.assertEqual(canonical, case["expected"])

    def test_result_mismatch_and_identity_mismatch_are_distinct(self) -> None:
        case = load_case("graph-diamond.json")
        mismatch = copy.deepcopy(case["expected"])
        mismatch["result"]["edges"][0] = [
            "python/pkg-a",
            "python/pkg-b",
        ]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, mismatch)
        self.assertEqual(raised.exception.code, "RESULT_MISMATCH")

        wrong_identity = copy.deepcopy(case["expected"])
        wrong_identity["case_id"] = "graph/not-this-case"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, wrong_identity)
        self.assertEqual(raised.exception.code, "RESULT_CASE_ID_MISMATCH")

    def test_plan_semantics_reject_unknown_references_and_duplicate_names(
        self,
    ) -> None:
        case = load_case("plan-affected-empty.json")

        duplicate = copy.deepcopy(case["expected"])
        duplicate_package = copy.deepcopy(duplicate["result"]["plan"]["packages"][0])
        duplicate_package["build_commands"] = ["different"]
        duplicate["result"]["plan"]["packages"].append(duplicate_package)
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, duplicate)
        self.assertEqual(raised.exception.code, "RESULT_PLAN_PACKAGE_DUPLICATE")

        unknown_edge = copy.deepcopy(case["expected"])
        unknown_edge["result"]["plan"]["dependency_edges"] = [
            ["python/missing", "python/pkg-a"]
        ]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, unknown_edge)
        self.assertEqual(raised.exception.code, "RESULT_PLAN_EDGE_UNKNOWN")

        unknown_affected = copy.deepcopy(case["expected"])
        unknown_affected["result"]["plan"]["affected_packages"] = ["python/missing"]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, unknown_affected)
        self.assertEqual(raised.exception.code, "RESULT_PLAN_AFFECTED_UNKNOWN")

    def test_validate_result_uses_the_bounded_parser_for_result_bytes(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_bytes(
                b'{"schema_version":1,"case_id":"graph/diamond",'
                b'"domain":"graph","outcome":"ok","result":'
                + (b"[" * 1100)
                + b"0"
                + (b"]" * 1100)
                + b',"diagnostics":[]}'
            )
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_result_files(case_path, result_path)
        self.assertEqual(raised.exception.code, "JSON_DEPTH_EXCEEDED")

    def test_validate_result_enforces_the_case_output_limit(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        case = load_case(case_path.name)
        payload = json.dumps(case["expected"]).encode("utf-8")
        output_limit = case["limits"]["output_bytes"]
        oversized = (b" " * (output_limit + 1 - len(payload))) + payload
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_bytes(oversized)
            with self.assertRaises(runner.ConformanceError) as raised:
                runner.validate_result_files(case_path, result_path)
        self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")


class PureDomainValidationTests(unittest.TestCase):
    def _schema_args(self) -> dict[str, object]:
        return {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": runner.load_document(
                FIXTURE_ROOT / "pure-domains.schema.json"
            ),
        }

    def test_validation_allows_repeated_envelope_diagnostic_codes(self) -> None:
        case = load_case("validation-missing-build.json")
        case["input"]["options"]["packages"].append(
            {
                "name": "python/other",
                "rel_path": "code/packages/python/other",
                "language": "python",
                "build_file_state": "missing",
                "build_references": [],
                "is_starlark": False,
                "declared_srcs": [],
                "declared_deps": [],
            }
        )
        case["expected"]["diagnostics"].append(
            {
                "code": "BUILD_FILE_MISSING",
                "severity": "error",
                "path": "code/packages/python/other",
            }
        )

        runner.validate_case_document(case, **self._schema_args())
        runner.assert_result_matches(
            case,
            copy.deepcopy(case["expected"]),
            pure_domain_schema=self._schema_args()["pure_domain_schema"],
        )

    def test_starlark_load_scanner_handles_lexical_literal_forms(self) -> None:
        commented = load_case("starlark-structured-context.json")
        commented["workspace"]["files"][0]["content_utf8"] = (
            '# load("../../../../../outside.star", "x")\n'
            + commented["workspace"]["files"][0]["content_utf8"]
        )
        runner.validate_case_document(commented, **self._schema_args())

        escaping_literals = (
            '"../../../../../outside.star"',
            'r"../../../../../outside.star"',
            'b"../../../../../outside.star"',
            '"""../../../../../outside.star"""',
        )
        for literal in escaping_literals:
            with self.subTest(literal=literal):
                escaping = load_case("starlark-structured-context.json")
                escaping["workspace"]["files"][0]["content_utf8"] = escaping[
                    "workspace"
                ]["files"][0]["content_utf8"].replace(
                    '"code/build/defs.star"',
                    literal,
                )
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(escaping, **self._schema_args())
                self.assertEqual(
                    raised.exception.code,
                    "EXPECTED_STARLARK_MODULE_ERROR_INVALID",
                )

        spaced = load_case("starlark-structured-context.json")
        spaced["workspace"]["files"][0]["content_utf8"] = spaced["workspace"]["files"][
            0
        ]["content_utf8"].replace("load(", "load (", 1)
        runner.validate_case_document(spaced, **self._schema_args())

        member = load_case("starlark-structured-context.json")
        member["workspace"]["files"][0]["content_utf8"] = (
            'rules.load("../../../../../outside.star")\n'
            + member["workspace"]["files"][0]["content_utf8"]
        )
        runner.validate_case_document(member, **self._schema_args())

        split = load_case("starlark-structured-context.json")
        split["workspace"]["files"][0]["content_utf8"] = split["workspace"]["files"][0][
            "content_utf8"
        ].replace("load(", "load\n(", 1)
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(split, **self._schema_args())
        self.assertEqual(
            raised.exception.code,
            "EXPECTED_STARLARK_MODULE_ERROR_INVALID",
        )

    def test_toolchain_detection_ignores_unscheduled_unsupported_packages(
        self,
    ) -> None:
        case = load_case("toolchain-detection-empty.json")
        case["input"]["options"]["packages"].append(
            {"name": "zig/unused", "language": "zig"}
        )

        runner.validate_case_document(case, **self._schema_args())

    def test_sharding_ignores_unscheduled_unreferenced_toolchains(self) -> None:
        case = load_case("sharding-prerequisite-closed.json")
        case["input"]["options"]["packages"].append(
            {
                "name": "zig/unused",
                "language": "zig",
                "build_command_count": 0,
            }
        )

        runner.validate_case_document(case, **self._schema_args())

    def test_dependency_cycles_are_rejected_without_recursion(self) -> None:
        cyclic = load_case("diff-selection-transitive.json")
        cyclic["input"]["options"]["edges"].append(["python/app", "python/base"])
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(cyclic, **self._schema_args())
        self.assertEqual(raised.exception.code, "CASE_EDGE_CYCLE")

        names = {f"python/package-{index}" for index in range(2000)}
        edges = [
            [f"python/package-{index}", f"python/package-{index + 1}"]
            for index in range(1999)
        ]
        edges.append(["python/package-1999", "python/package-0"])
        with self.assertRaises(runner.ConformanceError) as raised:
            runner._validate_known_edges(edges, names)
        self.assertEqual(raised.exception.code, "CASE_EDGE_CYCLE")

    def test_semantics_reject_unknown_references_and_bad_oracles(self) -> None:
        pure_schema = runner.load_document(FIXTURE_ROOT / "pure-domains.schema.json")
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": pure_schema,
        }

        unknown_edge = load_case("diff-selection-transitive.json")
        unknown_edge["input"]["options"]["edges"][0][0] = "python/missing"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(unknown_edge, **schema_args)
        self.assertEqual(raised.exception.code, "CASE_EDGE_UNKNOWN")

        hashing = load_case("hashing-cache-corrupt.json")
        hashing["expected"]["result"]["package_digest"] = "0" * 64
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.validate_case_document(hashing, **schema_args)
        self.assertEqual(raised.exception.code, "EXPECTED_HASH_MISMATCH")

        mutations = (
            (
                "starlark-structured-context.json",
                lambda result: result["result"]["targets"][0][
                    "rendered_commands"
                ].__setitem__(0, "wrong"),
                "RESULT_STARLARK_RENDER_MISMATCH",
            ),
            (
                "sharding-prerequisite-closed.json",
                lambda result: result["result"]["shards"][0].__setitem__(
                    "estimated_cost", 10
                ),
                "RESULT_SHARD_MISMATCH",
            ),
            (
                "validation-missing-build.json",
                lambda result: result["result"].__setitem__("diagnostic_codes", []),
                "RESULT_VALIDATION_INCONSISTENT",
            ),
            (
                "toolchain-detection-shared.json",
                lambda result: result["result"]["toolchains"].__setitem__(
                    "ocaml", True
                ),
                "RESULT_TOOLCHAIN_MISMATCH",
            ),
            (
                "cli-dry-run-success.json",
                lambda result: result["result"].__setitem__("exit_code", 2),
                "RESULT_CLI_EXIT_MISMATCH",
            ),
        )
        for filename, mutate, code in mutations:
            with self.subTest(filename=filename):
                case = load_case(filename)
                actual = copy.deepcopy(case["expected"])
                mutate(actual)
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.assert_result_matches(
                        case,
                        actual,
                        pure_domain_schema=pure_schema,
                    )
                self.assertEqual(raised.exception.code, code)

    def test_nested_paths_and_pure_authority_are_rejected(self) -> None:
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": runner.load_document(
                FIXTURE_ROOT / "pure-domains.schema.json"
            ),
        }
        mutations = (
            (
                "diff-selection-transitive.json",
                lambda case: case["input"]["options"]["packages"][0].__setitem__(
                    "rel_path", "code/packages/python/NUL"
                ),
                "CASE_NESTED_PATH_UNSAFE",
            ),
            (
                "hashing-cache-hit.json",
                lambda case: case["input"]["options"]["include_paths"].append(
                    "code/packages/python/demo/host-only"
                ),
                "CASE_HASH_PATH_UNKNOWN",
            ),
            (
                "starlark-structured-context.json",
                lambda case: case["input"]["options"].__setitem__(
                    "entrypoint", "code/packages/python/demo/missing.star"
                ),
                "CASE_STARLARK_ENTRYPOINT_MISSING",
            ),
            (
                "hashing-cache-hit.json",
                lambda case: case["workspace"]["files"][0].__setitem__(
                    "executable", True
                ),
                "CASE_PURE_AUTHORITY",
            ),
        )
        for filename, mutate, code in mutations:
            with self.subTest(filename=filename, code=code):
                case = load_case(filename)
                mutate(case)
                with self.assertRaises(runner.ConformanceError) as raised:
                    runner.validate_case_document(case, **schema_args)
                self.assertEqual(raised.exception.code, code)

    def test_pure_domain_set_order_is_canonicalized(self) -> None:
        diff = load_case("diff-selection-transitive.json")
        actual = copy.deepcopy(diff["expected"])
        actual["result"]["affected_packages"].reverse()
        self.assertEqual(runner.assert_result_matches(diff, actual), diff["expected"])

        shard = load_case("sharding-prerequisite-closed.json")
        actual = copy.deepcopy(shard["expected"])
        actual["result"]["shards"].reverse()
        actual["result"]["shards"][0]["package_names"].reverse()
        actual["result"]["shards"][0]["toolchains"].reverse()
        self.assertEqual(
            runner.assert_result_matches(shard, actual),
            shard["expected"],
        )

    def test_pure_domain_validation_has_no_host_side_effects(self) -> None:
        pure_schema = runner.load_document(FIXTURE_ROOT / "pure-domains.schema.json")
        pure_domains = set(pure_schema["$defs"]["pure_domain"]["enum"])
        schema_args = {
            "case_schema": runner.load_document(FIXTURE_ROOT / "schema.json"),
            "result_schema": runner.load_document(FIXTURE_ROOT / "result.schema.json"),
            "plan_schema": runner.load_document(
                runner.REPO_ROOT / "code/specs/schemas/build-plan-v1.schema.json"
            ),
            "pure_domain_schema": pure_schema,
        }
        cases = [
            runner.load_document(path)
            for path in sorted(CASES_ROOT.glob("*.json"))
            if runner.load_document(path)["domain"] in pure_domains
        ]

        with (
            mock.patch.object(tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(subprocess, "run") as process,
            mock.patch.object(subprocess, "Popen") as popen,
            mock.patch.object(os, "system") as system,
            mock.patch.object(os, "chmod") as chmod,
        ):
            for case in cases:
                runner.validate_case_document(case, **schema_args)

        temporary.assert_not_called()
        process.assert_not_called()
        popen.assert_not_called()
        system.assert_not_called()
        chmod.assert_not_called()


class CommandLineTests(unittest.TestCase):
    def test_validate_corpus_prints_a_machine_readable_summary(self) -> None:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = runner.main(
                ["validate-corpus", "--fixture-root", str(FIXTURE_ROOT)]
            )

        self.assertEqual(exit_code, 0)
        summary = json.loads(stdout.getvalue())
        self.assertEqual(summary["case_count"], 36)

    def test_validate_result_reports_match_and_rejects_execution_override(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        case = load_case(case_path.name)
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_text(
                json.dumps(case["expected"]),
                encoding="utf-8",
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
            self.assertEqual(exit_code, 0)
            result = json.loads(stdout.getvalue())
            self.assertEqual(result["status"], "matched")
            self.assertEqual(result["conformance_status"], "pass")

        stderr = io.StringIO()
        with redirect_stderr(stderr):
            exit_code = runner.main(["validate-corpus", "--allow-execution"])
        self.assertEqual(exit_code, 2)

    def test_result_mismatch_is_a_conformance_exit_not_an_input_exit(self) -> None:
        case_path = CASES_ROOT / "graph-diamond.json"
        case = load_case(case_path.name)
        mismatch = copy.deepcopy(case["expected"])
        mismatch["result"]["edges"][0] = [
            "python/pkg-a",
            "python/pkg-b",
        ]
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_text(json.dumps(mismatch), encoding="utf-8")
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
        self.assertEqual(exit_code, 1)
        self.assertEqual(json.loads(stderr.getvalue())["code"], "RESULT_MISMATCH")

    def test_pure_semantic_mismatch_is_a_conformance_exit(self) -> None:
        case_path = CASES_ROOT / "hashing-cache-hit.json"
        case = load_case(case_path.name)
        mismatch = copy.deepcopy(case["expected"])
        mismatch["result"]["package_digest"] = "0" * 64
        with tempfile.TemporaryDirectory() as directory:
            result_path = Path(directory) / "result.json"
            result_path.write_text(json.dumps(mismatch), encoding="utf-8")
            stderr = io.StringIO()
            with redirect_stderr(stderr):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
        self.assertEqual(exit_code, 1)
        self.assertEqual(
            json.loads(stderr.getvalue())["code"],
            "RESULT_HASH_MISMATCH",
        )

    def test_matching_unsupported_result_is_never_reported_as_passing(self) -> None:
        case = load_case("discovery-simple.json")
        case["id"] = "discovery/unsupported"
        case["expected"] = {
            "schema_version": 1,
            "case_id": "discovery/unsupported",
            "domain": "discovery",
            "outcome": "unsupported",
            "result": {},
            "diagnostics": [
                {
                    "code": "DISCOVERY_UNSUPPORTED",
                    "severity": "error",
                }
            ],
        }
        with tempfile.TemporaryDirectory() as directory:
            case_path = Path(directory) / "case.json"
            result_path = Path(directory) / "result.json"
            case_path.write_text(json.dumps(case), encoding="utf-8")
            result_path.write_text(
                json.dumps(case["expected"]),
                encoding="utf-8",
            )
            stdout = io.StringIO()
            with redirect_stdout(stdout):
                exit_code = runner.main(
                    [
                        "validate-result",
                        "--case",
                        str(case_path),
                        "--result",
                        str(result_path),
                    ]
                )
        result = json.loads(stdout.getvalue())
        self.assertEqual(exit_code, 1)
        self.assertEqual(result["status"], "matched")
        self.assertEqual(result["conformance_status"], "non-passing")
        self.assertEqual(result["outcome"], "unsupported")


if __name__ == "__main__":
    unittest.main()
