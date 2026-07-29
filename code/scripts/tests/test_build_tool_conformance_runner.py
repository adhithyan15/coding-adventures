from __future__ import annotations

import copy
import io
import json
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

    def test_rejects_moderate_and_extreme_depth_with_the_same_code(self) -> None:
        for depth in (65, 1100):
            raw = b'{"value":' + (b"[" * depth) + b"0" + (b"]" * depth) + b"}"
            with self.subTest(depth=depth):
                self.assert_parse_error(raw, "JSON_DEPTH_EXCEEDED")

    def test_rejects_oversized_input_before_decoding(self) -> None:
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.strict_load_bytes(b'{"value":"oversized"}', max_bytes=8)
        self.assertEqual(raised.exception.code, "JSON_INPUT_TOO_LARGE")


class CorpusTests(unittest.TestCase):
    def test_checked_in_corpus_and_manifest_validate(self) -> None:
        summary = runner.validate_corpus(FIXTURE_ROOT)

        self.assertEqual(summary["schema_version"], 1)
        self.assertEqual(summary["case_count"], 7)
        self.assertEqual(summary["implementation_count"], 16)
        self.assertEqual(summary["established_languages"], 15)
        self.assertEqual(summary["execution_case_count"], 0)
        self.assertEqual(
            summary["domains"],
            ["discovery", "graph", "plan", "resolution"],
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


class ExecutionDenialTests(unittest.TestCase):
    def assert_denied_before_side_effects(self, case: dict[str, object]) -> None:
        with (
            mock.patch.object(runner.tempfile, "TemporaryDirectory") as temporary,
            mock.patch.object(runner.base64, "b64decode") as decode,
            mock.patch.object(runner.os, "chmod") as chmod,
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


class MaterializationTests(unittest.TestCase):
    def test_materializes_exact_regular_files_and_cleans_up(self) -> None:
        case = load_case("discovery-simple.json")
        case["workspace"]["files"].append(
            {
                "path": "fixtures/space and & metacharacters.bin",
                "content_base64": "AAEC/w==",
            }
        )

        with runner.materialized_workspace(case) as root:
            retained_root = root
            self.assertEqual(
                (root / "code/packages/python/demo/BUILD").read_bytes(),
                b"python -m unittest discover tests\n",
            )
            self.assertEqual(
                (root / "fixtures/space and & metacharacters.bin").read_bytes(),
                b"\x00\x01\x02\xff",
            )
            for path in root.rglob("*"):
                self.assertFalse(path.is_symlink())
                if path.is_file():
                    self.assertTrue(path.stat().st_mode & 0o100000)

        self.assertFalse(retained_root.exists())

    def test_invalid_base64_and_workspace_limit_fail_before_root_creation(self) -> None:
        invalid = load_case("discovery-simple.json")
        invalid["workspace"]["files"][0] = {
            "path": "code/packages/python/demo/BUILD",
            "content_base64": "AB==",
        }
        with (
            mock.patch.object(runner.tempfile, "TemporaryDirectory") as temporary,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.materialized_workspace(invalid)
        self.assertEqual(raised.exception.code, "WORKSPACE_BASE64_NONCANONICAL")
        temporary.assert_not_called()

        oversized = load_case("discovery-simple.json")
        oversized["limits"]["workspace_bytes"] = 1
        with (
            mock.patch.object(runner.tempfile, "TemporaryDirectory") as temporary,
            self.assertRaises(runner.ConformanceError) as raised,
        ):
            runner.materialized_workspace(oversized)
        self.assertEqual(raised.exception.code, "WORKSPACE_BYTE_LIMIT")
        temporary.assert_not_called()

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
                        runner.tempfile,
                        "TemporaryDirectory",
                    ) as temporary,
                    self.assertRaises(runner.ConformanceError) as raised,
                ):
                    runner.materialized_workspace(case)
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
        mismatch["result"]["levels"] = [["python/a"]]
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, mismatch)
        self.assertEqual(raised.exception.code, "RESULT_MISMATCH")

        wrong_identity = copy.deepcopy(case["expected"])
        wrong_identity["case_id"] = "graph/not-this-case"
        with self.assertRaises(runner.ConformanceError) as raised:
            runner.assert_result_matches(case, wrong_identity)
        self.assertEqual(raised.exception.code, "RESULT_CASE_ID_MISMATCH")

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


class CommandLineTests(unittest.TestCase):
    def test_validate_corpus_prints_a_machine_readable_summary(self) -> None:
        stdout = io.StringIO()
        with redirect_stdout(stdout):
            exit_code = runner.main(
                ["validate-corpus", "--fixture-root", str(FIXTURE_ROOT)]
            )

        self.assertEqual(exit_code, 0)
        summary = json.loads(stdout.getvalue())
        self.assertEqual(summary["case_count"], 7)

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
            self.assertEqual(json.loads(stdout.getvalue())["status"], "pass")

        stderr = io.StringIO()
        with redirect_stderr(stderr):
            exit_code = runner.main(["validate-corpus", "--allow-execution"])
        self.assertEqual(exit_code, 2)


if __name__ == "__main__":
    unittest.main()
