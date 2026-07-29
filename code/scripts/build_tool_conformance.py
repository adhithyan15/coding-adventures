#!/usr/bin/env python3
"""Validate the language-neutral build-tool conformance corpus and results.

This bootstrap runner is intentionally process-free. It validates and
materializes data-only, non-execution cases, and it compares externally
produced results with the shared fixture oracle. Adapter orchestration belongs
to the later trusted-sandbox tranche.
"""

from __future__ import annotations

import argparse
import base64
import binascii
import json
import os
import re
import stat
import sys
import tempfile
import unicodedata
from contextlib import contextmanager
from dataclasses import dataclass
from pathlib import Path
from typing import Any, Iterator, Sequence


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = (
    REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
)
MAX_SAFE_INTEGER = 9_007_199_254_740_991
MAX_DOCUMENT_BYTES = 2_000_000
MAX_RESULT_BYTES = 16_777_216
MAX_JSON_DEPTH = 64
MAX_WORKSPACE_FILES = 4096
MAX_WORKSPACE_BYTES = 268_435_456
RESERVED_ADAPTER_FLAGS = ("--conformance", "--workspace-root", "--output")
EXECUTION_CAPABILITIES = {"execution", "trusted_execution"}
ESTABLISHED_LANGUAGES = (
    "csharp",
    "dart",
    "elixir",
    "fsharp",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
)
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
    "CONIN$",
    "CONOUT$",
    "CLOCK$",
    *(f"COM{index}" for index in range(1, 10)),
    *(f"LPT{index}" for index in range(1, 10)),
}


class ConformanceError(ValueError):
    """A stable machine-readable conformance runner failure."""

    def __init__(self, code: str, message: str) -> None:
        super().__init__(f"{code}: {message}")
        self.code = code
        self.message = message


@dataclass(frozen=True)
class WorkspaceFile:
    path: str
    content: bytes
    executable: bool


def _raise_duplicate_key(pairs: list[tuple[str, Any]]) -> dict[str, Any]:
    result: dict[str, Any] = {}
    for key, value in pairs:
        if key in result:
            raise ConformanceError(
                "JSON_DUPLICATE_KEY",
                f"duplicate JSON key: {key}",
            )
        result[key] = value
    return result


def _raise_non_finite(value: str) -> None:
    raise ConformanceError(
        "JSON_NON_FINITE",
        f"non-finite JSON number: {value}",
    )


def _raise_float(value: str) -> None:
    raise ConformanceError(
        "JSON_FLOAT_FORBIDDEN",
        f"floating-point JSON value is forbidden: {value}",
    )


def _scan_json_depth(text: str, max_depth: int) -> None:
    depth = 0
    in_string = False
    escaped = False
    for character in text:
        if in_string:
            if escaped:
                escaped = False
            elif character == "\\":
                escaped = True
            elif character == '"':
                in_string = False
            continue
        if character == '"':
            in_string = True
        elif character in "[{":
            depth += 1
            if depth > max_depth:
                raise ConformanceError(
                    "JSON_DEPTH_EXCEEDED",
                    f"JSON nesting exceeds {max_depth} levels",
                )
        elif character in "]}":
            depth -= 1
            if depth < 0:
                raise ConformanceError(
                    "JSON_SYNTAX_INVALID",
                    "JSON has an unmatched closing delimiter",
                )


def _validate_json_value(value: Any, depth: int = 0) -> None:
    if depth > MAX_JSON_DEPTH:
        raise ConformanceError(
            "JSON_DEPTH_EXCEEDED",
            f"JSON nesting exceeds {MAX_JSON_DEPTH} levels",
        )
    if isinstance(value, str):
        if any(0xD800 <= ord(character) <= 0xDFFF for character in value):
            raise ConformanceError(
                "JSON_UNICODE_SURROGATE",
                "unpaired Unicode surrogate",
            )
        return
    if value is None or isinstance(value, bool):
        return
    if isinstance(value, int):
        if not -MAX_SAFE_INTEGER <= value <= MAX_SAFE_INTEGER:
            raise ConformanceError(
                "JSON_INTEGER_RANGE",
                "integer is outside the interoperable range",
            )
        return
    if isinstance(value, float):
        raise ConformanceError(
            "JSON_FLOAT_FORBIDDEN",
            "floating-point JSON values are forbidden",
        )
    if isinstance(value, list):
        for item in value:
            _validate_json_value(item, depth + 1)
        return
    if isinstance(value, dict):
        for key, item in value.items():
            _validate_json_value(key, depth + 1)
            _validate_json_value(item, depth + 1)
        return
    raise ConformanceError(
        "JSON_VALUE_INVALID",
        f"unsupported JSON value: {type(value).__name__}",
    )


def strict_load_bytes(
    raw: bytes,
    *,
    max_bytes: int = MAX_DOCUMENT_BYTES,
    max_depth: int = MAX_JSON_DEPTH,
) -> dict[str, Any]:
    """Parse a bounded, strict UTF-8 RFC 8259 object."""

    if len(raw) > max_bytes:
        raise ConformanceError(
            "JSON_INPUT_TOO_LARGE",
            f"JSON input exceeds {max_bytes} bytes",
        )
    if raw.startswith(b"\xef\xbb\xbf"):
        raise ConformanceError("JSON_BOM_FORBIDDEN", "UTF-8 BOM is forbidden")
    try:
        text = raw.decode("utf-8", errors="strict")
    except UnicodeDecodeError as error:
        raise ConformanceError(
            "JSON_UTF8_INVALID",
            "JSON input is not strict UTF-8",
        ) from error

    _scan_json_depth(text, max_depth)
    try:
        value = json.loads(
            text,
            object_pairs_hook=_raise_duplicate_key,
            parse_constant=_raise_non_finite,
            parse_float=_raise_float,
        )
    except ConformanceError:
        raise
    except RecursionError as error:
        raise ConformanceError(
            "JSON_DEPTH_EXCEEDED",
            f"JSON nesting exceeds {max_depth} levels",
        ) from error
    except json.JSONDecodeError as error:
        raise ConformanceError(
            "JSON_SYNTAX_INVALID",
            f"invalid JSON at line {error.lineno}, column {error.colno}",
        ) from error

    _validate_json_value(value)
    if not isinstance(value, dict):
        raise ConformanceError(
            "JSON_TOP_LEVEL_INVALID",
            "top-level JSON value must be an object",
        )
    return value


def load_document(
    path: Path,
    *,
    max_bytes: int = MAX_DOCUMENT_BYTES,
) -> dict[str, Any]:
    try:
        raw = path.read_bytes()
    except OSError as error:
        raise ConformanceError(
            "DOCUMENT_READ_FAILED",
            f"could not read {path}",
        ) from error
    return strict_load_bytes(raw, max_bytes=max_bytes)


def portable_path_error(path: Any) -> str | None:
    if not isinstance(path, str) or not path:
        return "path is empty or not a string"
    if len(path) > 512:
        return "path exceeds 512 characters"
    if path != unicodedata.normalize("NFC", path):
        return "path is not NFC-normalized"
    if (
        path.startswith("/")
        or re.match(r"^[A-Za-z]:", path)
        or "\\" in path
        or "//" in path
        or any(
            ord(character) < 32 or character in '<>:"|?*'
            for character in path
        )
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


def reject_execution_intent(case: dict[str, Any]) -> None:
    domain = case.get("domain")
    input_value = case.get("input")
    operation = (
        input_value.get("operation") if isinstance(input_value, dict) else None
    )
    capabilities = case.get("capabilities")
    capability_set = set(capabilities) if isinstance(capabilities, list) else set()
    if (
        domain == "execution"
        or operation == "execution"
        or capability_set.intersection(EXECUTION_CAPABILITIES)
    ):
        raise ConformanceError(
            "EXECUTION_DISABLED",
            "the bootstrap runner never accepts execution intent",
        )


def _decode_workspace_file(entry: dict[str, Any]) -> bytes:
    if "content_utf8" in entry:
        value = entry["content_utf8"]
        if not isinstance(value, str):
            raise ConformanceError(
                "WORKSPACE_UTF8_INVALID",
                "content_utf8 must be a string",
            )
        return value.encode("utf-8")
    value = entry.get("content_base64")
    if not isinstance(value, str):
        raise ConformanceError(
            "WORKSPACE_CONTENT_MISSING",
            "workspace file must contain UTF-8 or base64 content",
        )
    try:
        decoded = base64.b64decode(value, validate=True)
    except (binascii.Error, ValueError) as error:
        raise ConformanceError(
            "WORKSPACE_BASE64_INVALID",
            "workspace file contains invalid base64",
        ) from error
    if base64.b64encode(decoded).decode("ascii") != value:
        raise ConformanceError(
            "WORKSPACE_BASE64_NONCANONICAL",
            "workspace file contains non-canonical base64",
        )
    return decoded


def preflight_workspace(case: dict[str, Any]) -> list[WorkspaceFile]:
    """Validate and decode a workspace completely before creating a root."""

    reject_execution_intent(case)
    workspace = case.get("workspace")
    files_value = workspace.get("files") if isinstance(workspace, dict) else None
    if not isinstance(files_value, list):
        raise ConformanceError(
            "WORKSPACE_FILES_INVALID",
            "workspace.files must be an array",
        )
    if len(files_value) > MAX_WORKSPACE_FILES:
        raise ConformanceError(
            "WORKSPACE_FILE_LIMIT",
            f"workspace contains more than {MAX_WORKSPACE_FILES} files",
        )

    normalized_paths: dict[str, str] = {}
    staged_files: list[WorkspaceFile] = []
    total_bytes = 0
    for entry in files_value:
        if not isinstance(entry, dict):
            raise ConformanceError(
                "WORKSPACE_FILE_INVALID",
                "workspace file entry must be an object",
            )
        path = entry.get("path")
        if error := portable_path_error(path):
            raise ConformanceError(
                "WORKSPACE_PATH_UNSAFE",
                f"unsafe workspace path {path!r}: {error}",
            )
        normalized = unicodedata.normalize("NFC", path).casefold()
        if normalized in normalized_paths:
            raise ConformanceError(
                "WORKSPACE_PATH_COLLISION",
                f"workspace path collides with {normalized_paths[normalized]}",
            )
        for existing, original in normalized_paths.items():
            if (
                normalized.startswith(f"{existing}/")
                or existing.startswith(f"{normalized}/")
            ):
                raise ConformanceError(
                    "WORKSPACE_PATH_PREFIX_CONFLICT",
                    f"workspace paths conflict: {original} and {path}",
                )
        normalized_paths[normalized] = path
        content = _decode_workspace_file(entry)
        total_bytes += len(content)
        staged_files.append(
            WorkspaceFile(
                path=path,
                content=content,
                executable=bool(entry.get("executable", False)),
            )
        )

    requested_limit = case.get("limits", {}).get("workspace_bytes")
    if not isinstance(requested_limit, int):
        requested_limit = MAX_WORKSPACE_BYTES
    effective_limit = min(requested_limit, MAX_WORKSPACE_BYTES)
    if total_bytes > effective_limit:
        raise ConformanceError(
            "WORKSPACE_BYTE_LIMIT",
            f"decoded workspace exceeds {effective_limit} bytes",
        )
    return sorted(staged_files, key=lambda entry: entry.path.casefold())


def _ensure_beneath(root: Path, destination: Path) -> None:
    try:
        destination.relative_to(root)
    except ValueError as error:
        raise ConformanceError(
            "WORKSPACE_PATH_ESCAPE",
            "workspace destination escapes the temporary root",
        ) from error


@contextmanager
def materialized_workspace(case: dict[str, Any]) -> Iterator[Path]:
    """Materialize a preflighted pure case in a private temporary directory."""

    staged_files = preflight_workspace(case)
    with tempfile.TemporaryDirectory(prefix="build-tool-conformance-") as directory:
        root = Path(directory).resolve(strict=True)
        for entry in staged_files:
            destination = root.joinpath(*entry.path.split("/"))
            _ensure_beneath(root, destination)
            destination.parent.mkdir(parents=True, exist_ok=True)
            _ensure_beneath(root, destination.parent.resolve(strict=True))
            flags = os.O_WRONLY | os.O_CREAT | os.O_EXCL
            if hasattr(os, "O_BINARY"):
                flags |= os.O_BINARY
            if hasattr(os, "O_NOFOLLOW"):
                flags |= os.O_NOFOLLOW
            mode = 0o700 if entry.executable else 0o600
            try:
                descriptor = os.open(destination, flags, mode)
                with os.fdopen(descriptor, "wb") as output:
                    output.write(entry.content)
                if entry.executable:
                    os.chmod(destination, 0o700)
            except OSError as error:
                raise ConformanceError(
                    "WORKSPACE_WRITE_FAILED",
                    f"could not materialize {entry.path}",
                ) from error
            if not stat.S_ISREG(destination.stat(follow_symlinks=False).st_mode):
                raise ConformanceError(
                    "WORKSPACE_FILE_TYPE_INVALID",
                    f"materialized path is not a regular file: {entry.path}",
                )
        yield root


def _schema_errors(
    instance: dict[str, Any],
    schema: dict[str, Any],
) -> list[str]:
    try:
        import jsonschema
    except ImportError as error:
        raise ConformanceError(
            "JSONSCHEMA_UNAVAILABLE",
            "install jsonschema==4.26.0 to validate conformance documents",
        ) from error

    jsonschema.Draft202012Validator.check_schema(schema)
    validator = jsonschema.Draft202012Validator(schema)
    errors = sorted(
        validator.iter_errors(instance),
        key=lambda error: [str(part) for part in error.absolute_path],
    )
    formatted: list[str] = []
    for error in errors:
        path = "/".join(str(part) for part in error.absolute_path) or "<root>"
        formatted.append(f"{path}: {error.message}")
    return formatted


def _validate_schema(
    instance: dict[str, Any],
    schema: dict[str, Any],
    code: str,
) -> None:
    if errors := _schema_errors(instance, schema):
        raise ConformanceError(code, errors[0])


def _validate_input_paths(case: dict[str, Any]) -> None:
    input_value = case.get("input", {})
    changed_paths = input_value.get("changed_paths", [])
    normalized_changed: set[str] = set()
    for path in changed_paths:
        if error := portable_path_error(path):
            raise ConformanceError(
                "CASE_CHANGED_PATH_UNSAFE",
                f"unsafe changed path {path!r}: {error}",
            )
        normalized = unicodedata.normalize("NFC", path).casefold()
        if normalized in normalized_changed:
            raise ConformanceError(
                "CASE_CHANGED_PATH_COLLISION",
                f"changed path collides after normalization: {path}",
            )
        normalized_changed.add(normalized)

    for key, value in input_value.get("options", {}).items():
        if (
            isinstance(value, str)
            and key.endswith(("_path", "_root", "_file"))
            and (error := portable_path_error(value))
        ):
            raise ConformanceError(
                "CASE_OPTION_PATH_UNSAFE",
                f"unsafe path option {key}: {error}",
            )

    for argument in input_value.get("arguments", []):
        if any(
            argument == flag or argument.startswith(f"{flag}=")
            for flag in RESERVED_ADAPTER_FLAGS
        ):
            raise ConformanceError(
                "CASE_RESERVED_ARGUMENT",
                f"input.arguments contains reserved flag: {argument}",
            )


def _validate_case_identity(case: dict[str, Any]) -> None:
    domain = case["domain"]
    if domain != case["input"]["operation"]:
        raise ConformanceError(
            "CASE_DOMAIN_OPERATION_MISMATCH",
            "domain must equal input.operation",
        )
    if case["id"] != case["expected"]["case_id"]:
        raise ConformanceError(
            "CASE_EXPECTED_ID_MISMATCH",
            "id must equal expected.case_id",
        )
    if domain != case["expected"]["domain"]:
        raise ConformanceError(
            "CASE_EXPECTED_DOMAIN_MISMATCH",
            "domain must equal expected.domain",
        )

    capabilities = set(case["capabilities"])
    required = DOMAIN_CAPABILITIES[domain]
    if domain == "plan":
        if not required.intersection(capabilities):
            raise ConformanceError(
                "CASE_CAPABILITY_MISSING",
                "plan case requires plan_v1_read or plan_v1_write",
            )
    elif not required.issubset(capabilities):
        raise ConformanceError(
            "CASE_CAPABILITY_MISSING",
            f"{domain} case is missing its domain capability",
        )


def _validate_plan_semantics(plan: dict[str, Any]) -> None:
    packages = plan.get("packages", [])
    package_names: set[str] = set()
    for package in packages:
        name = package["name"]
        if name in package_names:
            raise ConformanceError(
                "RESULT_PLAN_PACKAGE_DUPLICATE",
                f"duplicate plan package: {name}",
            )
        package_names.add(name)
        if error := portable_path_error(package["rel_path"]):
            raise ConformanceError(
                "RESULT_PLAN_PATH_UNSAFE",
                f"unsafe plan rel_path for {name}: {error}",
            )
    for edge in plan.get("dependency_edges", []):
        if edge[0] not in package_names or edge[1] not in package_names:
            raise ConformanceError(
                "RESULT_PLAN_EDGE_UNKNOWN",
                f"plan edge references an unknown package: {edge}",
            )
    affected = plan.get("affected_packages")
    if isinstance(affected, list):
        for name in affected:
            if name not in package_names:
                raise ConformanceError(
                    "RESULT_PLAN_AFFECTED_UNKNOWN",
                    f"affected package is not declared: {name}",
                )


def _sort_json_objects(value: Any) -> Any:
    if isinstance(value, dict):
        return {
            key: _sort_json_objects(value[key])
            for key in sorted(value)
        }
    if isinstance(value, list):
        return [_sort_json_objects(item) for item in value]
    return value


def canonicalize_result(result: dict[str, Any]) -> dict[str, Any]:
    canonical = json.loads(json.dumps(result))
    domain = canonical.get("domain")
    payload = canonical.get("result", {})

    diagnostics = canonical.get("diagnostics", [])
    diagnostics.sort(
        key=lambda item: (
            item.get("code", ""),
            item.get("path", ""),
            item.get("package", ""),
            json.dumps(item.get("details", {}), sort_keys=True),
        )
    )

    if domain == "discovery" and "packages" in payload:
        payload["packages"].sort(key=lambda package: package["name"])
    elif domain == "resolution" and "edges" in payload:
        payload["edges"].sort(key=lambda edge: (edge[0], edge[1]))
    elif domain == "graph":
        if "edges" in payload:
            payload["edges"].sort(key=lambda edge: (edge[0], edge[1]))
        if "levels" in payload:
            payload["levels"] = [sorted(level) for level in payload["levels"]]
    elif domain == "plan" and "plan" in payload:
        plan = payload["plan"]
        plan["packages"].sort(key=lambda package: package["name"])
        plan["dependency_edges"].sort(key=lambda edge: (edge[0], edge[1]))
        if isinstance(plan.get("affected_packages"), list):
            plan["affected_packages"].sort()
        if "shards" in plan:
            plan["shards"].sort(key=lambda shard: shard["index"])
            for shard in plan["shards"]:
                shard["assigned_packages"].sort()
                shard["package_names"].sort()

    return _sort_json_objects(canonical)


def _validate_result_shape(
    case: dict[str, Any],
    result: dict[str, Any],
    *,
    result_schema: dict[str, Any],
    plan_schema: dict[str, Any],
    code: str,
) -> None:
    _validate_schema(result, result_schema, code)
    if result["case_id"] != case["id"]:
        raise ConformanceError(
            "RESULT_CASE_ID_MISMATCH",
            "result case_id does not match the fixture",
        )
    if result["domain"] != case["domain"]:
        raise ConformanceError(
            "RESULT_DOMAIN_MISMATCH",
            "result domain does not match the fixture",
        )
    payload = result["result"]
    if result["outcome"] == "ok" and result["domain"] == "discovery":
        names = [package["name"] for package in payload["packages"]]
        if len(names) != len(set(names)):
            raise ConformanceError(
                "RESULT_PACKAGE_NAME_DUPLICATE",
                "discovery result contains a duplicate package name",
            )
    if result["outcome"] == "ok" and result["domain"] == "graph":
        flattened = [
            package
            for level in payload["levels"]
            for package in level
        ]
        if len(flattened) != len(set(flattened)):
            raise ConformanceError(
                "RESULT_GRAPH_LEVEL_DUPLICATE",
                "a graph package appears in more than one level",
            )
        level_packages = set(flattened)
        edge_packages = {
            package
            for edge in payload["edges"]
            for package in edge
        }
        if not edge_packages.issubset(level_packages):
            raise ConformanceError(
                "RESULT_GRAPH_LEVEL_MISSING",
                "a graph edge endpoint is absent from the dependency levels",
            )
    if (
        result["domain"] == "plan"
        and result["outcome"] == "ok"
        and "plan" in result["result"]
    ):
        plan = result["result"]["plan"]
        _validate_schema(plan, plan_schema, "RESULT_PLAN_SCHEMA_INVALID")
        _validate_plan_semantics(plan)


def assert_result_matches(
    case: dict[str, Any],
    actual: dict[str, Any],
    *,
    result_schema: dict[str, Any] | None = None,
    plan_schema: dict[str, Any] | None = None,
) -> dict[str, Any]:
    reject_execution_intent(case)
    result_schema = result_schema or load_document(
        DEFAULT_FIXTURE_ROOT / "result.schema.json"
    )
    plan_schema = plan_schema or load_document(
        REPO_ROOT / "code" / "specs" / "schemas" / "build-plan-v1.schema.json"
    )
    _validate_result_shape(
        case,
        actual,
        result_schema=result_schema,
        plan_schema=plan_schema,
        code="RESULT_SCHEMA_INVALID",
    )
    canonical_actual = canonicalize_result(actual)
    canonical_expected = canonicalize_result(case["expected"])
    if canonical_actual != canonical_expected:
        raise ConformanceError(
            "RESULT_MISMATCH",
            "canonical adapter result does not match the fixture oracle",
        )
    return canonical_actual


def validate_case_document(
    case: dict[str, Any],
    *,
    case_schema: dict[str, Any],
    result_schema: dict[str, Any],
    plan_schema: dict[str, Any],
) -> list[WorkspaceFile]:
    reject_execution_intent(case)
    _validate_schema(case, case_schema, "CASE_SCHEMA_INVALID")
    _validate_case_identity(case)
    _validate_input_paths(case)
    staged_files = preflight_workspace(case)
    _validate_result_shape(
        case,
        case["expected"],
        result_schema=result_schema,
        plan_schema=plan_schema,
        code="EXPECTED_SCHEMA_INVALID",
    )
    expected = case["expected"]
    if expected["outcome"] in {"unsupported", "skipped"}:
        if not expected["diagnostics"]:
            raise ConformanceError(
                "EXPECTED_DIAGNOSTIC_MISSING",
                f"{expected['outcome']} requires a diagnostic",
            )
    if expected != canonicalize_result(expected):
        raise ConformanceError(
            "EXPECTED_NOT_CANONICAL",
            "checked-in expected result is not canonically ordered",
        )
    return staged_files


def _validate_manifest(
    manifest: dict[str, Any],
    schema: dict[str, Any],
) -> dict[str, int]:
    _validate_schema(manifest, schema, "MANIFEST_SCHEMA_INVALID")
    implementations = manifest["implementations"]
    languages = [item["language"] for item in implementations]
    if languages != sorted(languages):
        raise ConformanceError(
            "MANIFEST_NOT_CANONICAL",
            "implementations must be sorted by language",
        )
    if len(languages) != len(set(languages)):
        raise ConformanceError(
            "MANIFEST_LANGUAGE_DUPLICATE",
            "implementation languages must be unique",
        )
    established = {
        item["language"]
        for item in implementations
        if item["lane_status"] == "established"
    }
    if established != set(ESTABLISHED_LANGUAGES):
        raise ConformanceError(
            "MANIFEST_ESTABLISHED_SET",
            "manifest must contain exactly the established language registry",
        )
    by_language = {item["language"]: item for item in implementations}
    for item in implementations:
        status_value = item["implementation_status"]
        front_door = item["front_door"]
        capabilities = item["capabilities"]
        if capabilities != sorted(capabilities):
            raise ConformanceError(
                "MANIFEST_CAPABILITIES_NOT_CANONICAL",
                f"capabilities are not sorted for {item['language']}",
            )
        if status_value == "missing":
            if front_door is not None or capabilities:
                raise ConformanceError(
                    "MANIFEST_MISSING_HAS_IMPLEMENTATION",
                    f"missing implementation has a front door: {item['language']}",
                )
        else:
            if not isinstance(front_door, str):
                raise ConformanceError(
                    "MANIFEST_FRONT_DOOR_MISSING",
                    f"implementation has no front door: {item['language']}",
                )
            if error := portable_path_error(front_door):
                raise ConformanceError(
                    "MANIFEST_FRONT_DOOR_UNSAFE",
                    f"unsafe front door for {item['language']}: {error}",
                )
            if not (REPO_ROOT / front_door).is_dir():
                raise ConformanceError(
                    "MANIFEST_FRONT_DOOR_ABSENT",
                    f"front door does not exist: {front_door}",
                )
        if status_value == "shared_engine":
            shared_engine = item["shared_engine"]
            if (
                not isinstance(shared_engine, str)
                or shared_engine not in by_language
                or shared_engine == item["language"]
            ):
                raise ConformanceError(
                    "MANIFEST_SHARED_ENGINE_INVALID",
                    f"invalid shared engine for {item['language']}",
                )
        elif item["shared_engine"] is not None:
            raise ConformanceError(
                "MANIFEST_SHARED_ENGINE_UNEXPECTED",
                f"unexpected shared engine for {item['language']}",
            )
    return {
        "implementation_count": len(implementations),
        "established_languages": len(established),
        "front_door_count": sum(
            item["implementation_status"] in {"present", "shared_engine"}
            for item in implementations
        ),
        "adapter_ready_count": sum(
            item["adapter_status"] == "ready"
            for item in implementations
        ),
    }


def validate_corpus(
    fixture_root: Path = DEFAULT_FIXTURE_ROOT,
) -> dict[str, Any]:
    fixture_root = fixture_root.resolve()
    case_schema = load_document(fixture_root / "schema.json")
    result_schema = load_document(fixture_root / "result.schema.json")
    manifest_schema = load_document(
        fixture_root / "implementations.schema.json"
    )
    manifest = load_document(fixture_root / "implementations.json")
    plan_schema = load_document(
        REPO_ROOT / "code" / "specs" / "schemas" / "build-plan-v1.schema.json"
    )
    manifest_summary = _validate_manifest(manifest, manifest_schema)

    case_paths = sorted((fixture_root / "cases").glob("*.json"))
    if not case_paths:
        raise ConformanceError(
            "CORPUS_EMPTY",
            "the conformance corpus contains no cases",
        )
    case_ids: set[str] = set()
    domains: set[str] = set()
    materialized_files = 0
    for case_path in case_paths:
        case = load_document(case_path)
        if case.get("id") in case_ids:
            raise ConformanceError(
                "CORPUS_CASE_ID_DUPLICATE",
                f"duplicate corpus case id: {case.get('id')}",
            )
        staged_files = validate_case_document(
            case,
            case_schema=case_schema,
            result_schema=result_schema,
            plan_schema=plan_schema,
        )
        with materialized_workspace(case):
            pass
        case_ids.add(case["id"])
        domains.add(case["domain"])
        materialized_files += len(staged_files)

    return {
        "schema_version": 1,
        "case_count": len(case_paths),
        "implementation_count": manifest_summary["implementation_count"],
        "established_languages": manifest_summary["established_languages"],
        "front_door_count": manifest_summary["front_door_count"],
        "adapter_ready_count": manifest_summary["adapter_ready_count"],
        "conformance_run_count": 0,
        "conformance_status": "not-run",
        "execution_case_count": 0,
        "materialized_file_count": materialized_files,
        "domains": sorted(domains),
        "status": "valid",
    }


def validate_result_files(case_path: Path, result_path: Path) -> dict[str, Any]:
    case = load_document(case_path)
    reject_execution_intent(case)
    result = load_document(result_path, max_bytes=MAX_RESULT_BYTES)
    case_schema = load_document(DEFAULT_FIXTURE_ROOT / "schema.json")
    result_schema = load_document(DEFAULT_FIXTURE_ROOT / "result.schema.json")
    plan_schema = load_document(
        REPO_ROOT / "code" / "specs" / "schemas" / "build-plan-v1.schema.json"
    )
    validate_case_document(
        case,
        case_schema=case_schema,
        result_schema=result_schema,
        plan_schema=plan_schema,
    )
    return assert_result_matches(
        case,
        result,
        result_schema=result_schema,
        plan_schema=plan_schema,
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate build-tool conformance fixtures and results."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    corpus_parser = subparsers.add_parser(
        "validate-corpus",
        help="Validate and safely materialize the non-execution corpus.",
    )
    corpus_parser.add_argument(
        "--fixture-root",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
    )

    result_parser = subparsers.add_parser(
        "validate-result",
        help="Compare an externally produced result with one fixture.",
    )
    result_parser.add_argument("--case", type=Path, required=True)
    result_parser.add_argument("--result", type=Path, required=True)
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    try:
        arguments = parser.parse_args(argv)
    except SystemExit as error:
        return int(error.code)
    try:
        if arguments.command == "validate-corpus":
            output = validate_corpus(arguments.fixture_root)
        else:
            canonical = validate_result_files(
                arguments.case,
                arguments.result,
            )
            output = {
                "case_id": canonical["case_id"],
                "domain": canonical["domain"],
                "status": "pass",
            }
    except ConformanceError as error:
        print(
            json.dumps(
                {
                    "code": error.code,
                    "message": error.message,
                    "status": "error",
                },
                sort_keys=True,
            ),
            file=sys.stderr,
        )
        return 1 if error.code == "RESULT_MISMATCH" else 2

    print(json.dumps(output, sort_keys=True))
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
