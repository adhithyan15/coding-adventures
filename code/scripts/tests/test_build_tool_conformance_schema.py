from __future__ import annotations

import base64
import copy
import json
import re
import unicodedata
import unittest
from pathlib import Path
from typing import Any


ROOT = Path(__file__).resolve().parents[3]
FIXTURE_ROOT = ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
SCHEMA_PATH = FIXTURE_ROOT / "schema.json"
PURE_SCHEMA_PATH = FIXTURE_ROOT / "pure-domains.schema.json"
EXAMPLE_ROOT = FIXTURE_ROOT / "cases"
MAX_SAFE_INTEGER = 9_007_199_254_740_991
RESERVED_ADAPTER_FLAGS = ("--conformance", "--workspace-root", "--output")
DOMAIN_CAPABILITIES = {
    "discovery": {"discovery"},
    "resolution": {"resolution"},
    "graph": {"graph"},
    "diff_selection": {"diff_selection"},
    "hashing_cache": {"hashing_cache"},
    "starlark": {"starlark"},
    "plan": {"plan_v1_read", "plan_v1_write"},
    "sharding": {"sharding"},
    "execution": {"execution", "trusted_execution"},
    "validation": {"validation"},
    "toolchain_detection": {"toolchain_detection"},
    "cli": {"cli"},
}
WINDOWS_RESERVED_BASENAMES = {
    "CON",
    "PRN",
    "AUX",
    "NUL",
    *(f"COM{index}" for index in range(1, 10)),
    *(f"LPT{index}" for index in range(1, 10)),
}


def _reject_duplicate_pairs(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ValueError(f"duplicate JSON key: {key}")
        result[key] = value
    return result


def _reject_non_finite(value: str) -> None:
    raise ValueError(f"non-finite JSON number: {value}")


def _validate_json_value(value: Any, depth: int = 0) -> None:
    if depth > 64:
        raise ValueError("JSON nesting exceeds 64 levels")
    if isinstance(value, str):
        if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
            raise ValueError("unpaired Unicode surrogate")
        return
    if value is None or isinstance(value, bool):
        return
    if isinstance(value, int):
        if not -MAX_SAFE_INTEGER <= value <= MAX_SAFE_INTEGER:
            raise ValueError("integer outside interoperable range")
        return
    if isinstance(value, float):
        raise ValueError("floating-point JSON values are forbidden")
    if isinstance(value, list):
        for item in value:
            _validate_json_value(item, depth + 1)
        return
    if isinstance(value, dict):
        for key, item in value.items():
            _validate_json_value(key, depth + 1)
            _validate_json_value(item, depth + 1)
        return
    raise ValueError(f"unsupported JSON value: {type(value).__name__}")


def strict_loads(raw: bytes, max_bytes: int = 2_000_000) -> dict[str, Any]:
    if len(raw) > max_bytes:
        raise ValueError("JSON input exceeds byte limit")
    if raw.startswith(b"\xef\xbb\xbf"):
        raise ValueError("UTF-8 BOM is forbidden")
    text = raw.decode("utf-8", errors="strict")
    value = json.loads(
        text,
        object_pairs_hook=_reject_duplicate_pairs,
        parse_constant=_reject_non_finite,
    )
    _validate_json_value(value)
    if not isinstance(value, dict):
        raise ValueError("top-level JSON value must be an object")
    return value


def portable_path_error(path: str) -> str | None:
    if (
        not path
        or path.startswith("/")
        or (len(path) >= 2 and path[0].isalpha() and path[1] == ":")
        or "\\" in path
        or "//" in path
        or any(ord(character) < 32 or character in '<>:"|?*' for character in path)
    ):
        return "path is not a portable relative path"
    for segment in path.split("/"):
        if segment in {".", ".."}:
            return "path contains a dot segment"
        if segment.endswith((" ", ".")):
            return "path segment has a trailing dot or space"
        basename = segment.split(".", 1)[0].upper()
        if basename in WINDOWS_RESERVED_BASENAMES:
            return "path segment uses a Windows reserved basename"
    return None


def load_json(path: Path) -> dict[str, Any]:
    return strict_loads(path.read_bytes())


def semantic_errors(case: dict[str, Any]) -> list[str]:
    """Check cross-field and filesystem invariants JSON Schema cannot express."""

    errors: list[str] = []
    case_id = case.get("id")
    domain = case.get("domain")
    operation = case.get("input", {}).get("operation")
    expected = case.get("expected", {})

    if domain != operation:
        errors.append("domain must equal input.operation")
    if case_id != expected.get("case_id"):
        errors.append("id must equal expected.case_id")
    if domain != expected.get("domain"):
        errors.append("domain must equal expected.domain")

    capabilities = case.get("capabilities", [])
    required_capabilities = DOMAIN_CAPABILITIES.get(domain, set())
    if domain == "plan":
        if not required_capabilities.intersection(capabilities):
            errors.append(
                "plan cases require a plan_v1_read or plan_v1_write capability"
            )
    elif not required_capabilities.issubset(capabilities):
        errors.append(f"{domain} case is missing its domain capability")

    trusted_execution = "trusted_execution" in capabilities
    if (domain == "execution") != trusted_execution:
        errors.append("trusted_execution is required only for execution cases")

    normalized_paths: set[str] = set()
    workspace_bytes = 0
    for file_entry in case.get("workspace", {}).get("files", []):
        path = file_entry.get("path", "")
        if path_error := portable_path_error(path):
            errors.append(f"unsafe portable path {path}: {path_error}")
        normalized = unicodedata.normalize("NFC", path).casefold()
        if normalized in normalized_paths:
            errors.append(f"duplicate normalized path: {path}")
        if any(
            normalized.startswith(f"{existing}/")
            or existing.startswith(f"{normalized}/")
            for existing in normalized_paths
        ):
            errors.append(f"file/directory path conflict: {path}")
        normalized_paths.add(normalized)

        if "content_base64" in file_entry:
            try:
                decoded = base64.b64decode(file_entry["content_base64"], validate=True)
                canonical = base64.b64encode(decoded).decode("ascii")
                if canonical != file_entry["content_base64"]:
                    errors.append(f"non-canonical base64 content: {path}")
                workspace_bytes += len(decoded)
            except (ValueError, TypeError):
                errors.append(f"invalid base64 content: {path}")
        elif "content_utf8" in file_entry:
            workspace_bytes += len(file_entry["content_utf8"].encode("utf-8"))

    workspace_limit = case.get("limits", {}).get("workspace_bytes")
    if isinstance(workspace_limit, int) and workspace_bytes > workspace_limit:
        errors.append("decoded workspace exceeds requested workspace_bytes")

    normalized_changed_paths: set[str] = set()
    for path in case.get("input", {}).get("changed_paths", []):
        if path_error := portable_path_error(path):
            errors.append(f"unsafe portable path {path}: {path_error}")
        normalized = unicodedata.normalize("NFC", path).casefold()
        if normalized in normalized_changed_paths:
            errors.append(f"duplicate normalized changed path: {path}")
        normalized_changed_paths.add(normalized)

    for key, value in case.get("input", {}).get("options", {}).items():
        if (
            isinstance(value, str)
            and key.endswith(("_path", "_root", "_file"))
            and (path_error := portable_path_error(value))
        ):
            errors.append(f"unsafe portable path {value}: {path_error}")

    for argument in case.get("input", {}).get("arguments", []):
        if any(
            argument == flag or argument.startswith(f"{flag}=")
            for flag in RESERVED_ADAPTER_FLAGS
        ):
            errors.append(f"reserved adapter flag in input.arguments: {argument}")

    outcome = expected.get("outcome")
    if outcome in {"unsupported", "skipped"} and not expected.get("diagnostics"):
        errors.append(f"{outcome} outcome requires a diagnostic")

    return errors


class BuildToolConformanceSchemaTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls) -> None:
        cls.schema = load_json(SCHEMA_PATH)
        cls.pure_schema = load_json(PURE_SCHEMA_PATH)
        cls.examples = [load_json(path) for path in sorted(EXAMPLE_ROOT.glob("*.json"))]
        cls.base_example = next(
            example for example in cls.examples if example["id"] == "discovery/simple"
        )

    def test_schema_is_draft_2020_12_and_closed_at_the_boundary(self) -> None:
        self.assertEqual(
            self.schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema",
        )
        self.assertFalse(self.schema["additionalProperties"])
        self.assertEqual(self.schema["properties"]["schema_version"], {"const": 1})
        self.assertEqual(
            set(self.schema["required"]),
            {
                "schema_version",
                "id",
                "domain",
                "summary",
                "platforms",
                "capabilities",
                "workspace",
                "input",
                "expected",
                "limits",
            },
        )

    def test_checked_in_examples_are_formally_valid(self) -> None:
        import jsonschema

        jsonschema.Draft202012Validator.check_schema(self.schema)
        validator = jsonschema.Draft202012Validator(self.schema)
        for example in self.examples:
            errors = sorted(
                validator.iter_errors(example), key=lambda error: list(error.path)
            )
            self.assertEqual(
                errors,
                [],
                "\n".join(error.message for error in errors),
            )

    def test_checked_in_pure_domain_records_are_closed_and_formally_valid(
        self,
    ) -> None:
        import jsonschema

        jsonschema.Draft202012Validator.check_schema(self.pure_schema)
        validator = jsonschema.Draft202012Validator(self.pure_schema)
        pure_domains = set(self.pure_schema["$defs"]["pure_domain"]["enum"])
        seen: set[str] = set()
        for example in self.examples:
            if example["domain"] not in pure_domains:
                continue
            record = {
                "domain": example["domain"],
                "outcome": example["expected"]["outcome"],
                "input": example["input"],
                "result": example["expected"]["result"],
            }
            errors = list(validator.iter_errors(record))
            self.assertEqual(
                errors,
                [],
                "\n".join(error.message for error in errors),
            )
            seen.add(example["domain"])

        self.assertEqual(seen, pure_domains)

    def test_every_pure_domain_rejects_unknown_input_fields(self) -> None:
        import jsonschema

        validator = jsonschema.Draft202012Validator(self.pure_schema)
        pure_domains = set(self.pure_schema["$defs"]["pure_domain"]["enum"])
        examples = {
            example["domain"]: example
            for example in self.examples
            if example["domain"] in pure_domains
        }
        for domain, example in examples.items():
            with self.subTest(domain=domain):
                record = {
                    "domain": domain,
                    "outcome": example["expected"]["outcome"],
                    "input": copy.deepcopy(example["input"]),
                    "result": example["expected"]["result"],
                }
                record["input"]["options"]["unexpected"] = True
                self.assertTrue(list(validator.iter_errors(record)))

    def test_starlark_sources_accept_portable_recursive_globs(self) -> None:
        import jsonschema

        example = next(
            example
            for example in self.examples
            if example["id"] == "starlark/structured-context"
        )
        record = {
            "domain": example["domain"],
            "outcome": example["expected"]["outcome"],
            "input": example["input"],
            "result": copy.deepcopy(example["expected"]["result"]),
        }
        record["result"]["targets"][0]["srcs"] = ["src/**/*.py"]
        errors = list(
            jsonschema.Draft202012Validator(self.pure_schema).iter_errors(record)
        )
        self.assertEqual(errors, [])

    def test_package_names_require_nonempty_safe_segments(self) -> None:
        pattern = re.compile(self.pure_schema["$defs"]["package_name"]["pattern"])
        for valid in (
            "python/demo",
            "rust/http-core",
            "typescript/pkg/subpkg",
        ):
            self.assertIsNotNone(pattern.fullmatch(valid), valid)
        for invalid in (
            "python/",
            "python//demo",
            "python/../demo",
            "python/./demo",
        ):
            self.assertIsNone(pattern.fullmatch(invalid), invalid)

    def test_pure_domain_cases_carry_no_executable_or_host_input(self) -> None:
        pure_domains = set(self.pure_schema["$defs"]["pure_domain"]["enum"])
        for example in self.examples:
            if example["domain"] not in pure_domains:
                continue
            self.assertNotIn("arguments", example["input"])
            self.assertNotIn("environment", example["input"])
            self.assertFalse(
                any(
                    entry.get("executable", False)
                    for entry in example["workspace"]["files"]
                ),
                example["id"],
            )

    def test_checked_in_examples_satisfy_semantic_invariants(self) -> None:
        self.assertGreater(len(self.examples), 0)
        for example in self.examples:
            self.assertEqual(semantic_errors(example), [])

    def test_safe_path_pattern_rejects_portable_escape_shapes(self) -> None:
        pattern = re.compile(self.schema["$defs"]["safe_path"]["pattern"])
        for valid in (
            "code/packages/python/demo/BUILD",
            "fixtures/space and & metacharacters.txt",
            "a/.hidden",
        ):
            self.assertIsNotNone(pattern.fullmatch(valid), valid)

        for invalid in (
            "",
            "/absolute",
            "C:/drive-relative",
            r"\\server\share",
            r"code\package\BUILD",
            "../escape",
            "code/../escape",
            "code/./package",
            "code//package",
            "file:stream",
            "nul\u0000byte",
        ):
            self.assertIsNone(pattern.fullmatch(invalid), invalid)

    def test_fixture_environment_is_data_with_a_narrow_key_namespace(self) -> None:
        environment_pattern = re.compile(
            self.schema["$defs"]["input"]["properties"]["environment"]["propertyNames"][
                "pattern"
            ]
        )
        self.assertIsNotNone(environment_pattern.fullmatch("CONFORMANCE_MODE"))
        for dangerous in (
            "LD_PRELOAD",
            "DYLD_INSERT_LIBRARIES",
            "NODE_OPTIONS",
            "PYTHONPATH",
            "RUBYOPT",
            "PERL5OPT",
            "JAVA_TOOL_OPTIONS",
            "DOTNET_STARTUP_HOOKS",
            "BASH_ENV",
        ):
            self.assertIsNone(environment_pattern.fullmatch(dangerous), dangerous)

    def test_domain_operation_and_expected_identity_mismatches_are_rejected(
        self,
    ) -> None:
        base = self.base_example

        mismatch = copy.deepcopy(base)
        mismatch["input"]["operation"] = "execution"
        self.assertIn("domain must equal input.operation", semantic_errors(mismatch))

        mismatch = copy.deepcopy(base)
        mismatch["expected"]["case_id"] = "discovery/not-the-case"
        self.assertIn("id must equal expected.case_id", semantic_errors(mismatch))

        mismatch = copy.deepcopy(base)
        mismatch["expected"]["domain"] = "resolution"
        self.assertIn("domain must equal expected.domain", semantic_errors(mismatch))

        mismatch = copy.deepcopy(base)
        mismatch["input"]["arguments"] = ["--workspace-root=../../outside"]
        self.assertTrue(
            any(
                error.startswith("reserved adapter flag in input.arguments:")
                for error in semantic_errors(mismatch)
            )
        )

        for unsafe in (
            "../../outside",
            "/absolute",
            r"C:\outside",
            "//server/share",
        ):
            mismatch = copy.deepcopy(base)
            mismatch["input"]["options"]["code_root"] = unsafe
            self.assertTrue(
                any(
                    error.startswith("unsafe portable path")
                    for error in semantic_errors(mismatch)
                ),
                unsafe,
            )

    def test_normalized_duplicate_paths_and_invalid_base64_are_rejected(self) -> None:
        duplicate = copy.deepcopy(self.base_example)
        duplicate["workspace"]["files"].append(
            {
                "path": "CODE/PACKAGES/PYTHON/DEMO/build",
                "content_utf8": "duplicate on a case-insensitive filesystem\n",
            }
        )
        self.assertTrue(
            any(
                error.startswith("duplicate normalized path:")
                for error in semantic_errors(duplicate)
            )
        )

        prefix_conflict = copy.deepcopy(self.base_example)
        prefix_conflict["workspace"]["files"] = [
            {"path": "fixtures/data", "content_utf8": "file\n"},
            {"path": "FIXTURES/DATA/child", "content_utf8": "child\n"},
        ]
        self.assertTrue(
            any(
                error.startswith("file/directory path conflict:")
                for error in semantic_errors(prefix_conflict)
            )
        )

        changed_duplicate = copy.deepcopy(self.base_example)
        changed_duplicate["input"]["changed_paths"] = ["Code/X", "code/x"]
        self.assertTrue(
            any(
                error.startswith("duplicate normalized changed path:")
                for error in semantic_errors(changed_duplicate)
            )
        )

        invalid_base64 = copy.deepcopy(self.base_example)
        invalid_base64["workspace"]["files"][0].pop("content_utf8")
        invalid_base64["workspace"]["files"][0]["content_base64"] = "not base64!"
        self.assertTrue(
            any(
                error.startswith("invalid base64 content:")
                for error in semantic_errors(invalid_base64)
            )
        )

        noncanonical_base64 = copy.deepcopy(self.base_example)
        noncanonical_base64["workspace"]["files"][0].pop("content_utf8")
        noncanonical_base64["workspace"]["files"][0]["content_base64"] = "AB=="
        self.assertTrue(
            any(
                error.startswith("non-canonical base64 content:")
                for error in semantic_errors(noncanonical_base64)
            )
        )

    def test_windows_reserved_and_trailing_names_are_semantically_rejected(
        self,
    ) -> None:
        for unsafe in ("fixtures/NUL.txt", "fixtures/name.", "fixtures/name "):
            invalid = copy.deepcopy(self.base_example)
            invalid["workspace"]["files"][0]["path"] = unsafe
            self.assertTrue(
                any(
                    error.startswith("unsafe portable path")
                    for error in semantic_errors(invalid)
                ),
                unsafe,
            )

    def test_resource_requests_cover_the_adapter_process_and_cpu(self) -> None:
        required = set(self.schema["$defs"]["limits"]["required"])
        self.assertIn("cpu_time_ms", required)
        self.assertGreaterEqual(
            self.schema["$defs"]["limits"]["properties"]["process_count"]["minimum"],
            1,
        )

    def test_strict_parser_rejects_ambiguous_or_nonportable_json(self) -> None:
        invalid_documents = (
            b'{"domain":"discovery","domain":"execution"}',
            b'{"input":{"operation":"discovery","operation":"execution"}}',
            b'{"limits":{"wall_time_ms":1,"wall_time_ms":2}}',
            b'{"value":NaN}',
            b'{"value":1.5}',
            b'{"value":9007199254740992}',
            b'{"value":"\\ud800"}',
            b"\xef\xbb\xbf{}",
        )
        for raw in invalid_documents:
            with self.subTest(raw=raw):
                with self.assertRaises((UnicodeDecodeError, ValueError)):
                    strict_loads(raw)

        too_deep = b'{"value":' + (b"[" * 66) + b"0" + (b"]" * 66) + b"}"
        with self.assertRaises(ValueError):
            strict_loads(too_deep)
        with self.assertRaises(ValueError):
            strict_loads(b'{"value":"oversized"}', max_bytes=8)

    def test_schema_forbids_floats_and_out_of_range_integers(self) -> None:
        json_value = self.schema["$defs"]["json_value"]["oneOf"]
        integer_schema = next(
            entry for entry in json_value if entry.get("type") == "integer"
        )
        self.assertEqual(integer_schema["minimum"], -MAX_SAFE_INTEGER)
        self.assertEqual(integer_schema["maximum"], MAX_SAFE_INTEGER)
        self.assertNotIn("number", {entry.get("type") for entry in json_value})


if __name__ == "__main__":
    unittest.main()
