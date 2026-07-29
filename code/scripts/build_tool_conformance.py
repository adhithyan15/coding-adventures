#!/usr/bin/env python3
"""Validate the language-neutral build-tool conformance corpus and results.

This bootstrap runner is intentionally process-free. It validates data-only,
non-execution cases entirely in memory, and it compares externally produced
results with the shared fixture oracle. Adapter orchestration belongs to the
later trusted-sandbox tranche.
"""

from __future__ import annotations

import argparse
import ast
import base64
import binascii
import fnmatch
import hashlib
import json
import os
import posixpath
import re
import stat
import sys
import unicodedata
from dataclasses import dataclass
from functools import lru_cache
from pathlib import Path
from typing import Any, Sequence


REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
MAX_SAFE_INTEGER = 9_007_199_254_740_991
MAX_DOCUMENT_BYTES = 2_000_000
MAX_RESULT_BYTES = 16_777_216
MAX_JSON_DEPTH = 64
MAX_WORKSPACE_FILES = 4096
MAX_WORKSPACE_BYTES = 268_435_456
RESERVED_ADAPTER_FLAGS = ("--conformance", "--workspace-root", "--output")
EXECUTION_CAPABILITIES = {"execution", "trusted_execution"}
PURE_DOMAINS = {
    "cli",
    "diff_selection",
    "hashing_cache",
    "sharding",
    "starlark",
    "toolchain_detection",
    "validation",
}
BOOTSTRAP_DOMAINS = {
    "discovery",
    "graph",
    "plan",
    "resolution",
    *PURE_DOMAINS,
}
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
TOOLCHAINS = (
    "cpp",
    "dart",
    "dotnet",
    "elixir",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "swift",
    "typescript",
)
LANGUAGE_TOOLCHAINS = {
    **{toolchain: toolchain for toolchain in TOOLCHAINS},
    "c": "cpp",
    "csharp": "dotnet",
    "fsharp": "dotnet",
    "wasm": "rust",
}
TOOLCHAIN_WEIGHTS = {
    "rust": 6,
    "dotnet": 4,
    "haskell": 4,
    "swift": 4,
    "typescript": 4,
    "java": 3,
    "kotlin": 3,
    "elixir": 2,
    "python": 2,
    "ruby": 2,
}
DISPLAY_SAFE_TOKEN = re.compile(r"^[A-Za-z0-9_@%+=:,./-]+$")
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
    *(f"COM{index}" for index in "¹²³"),
    *(f"LPT{index}" for index in "¹²³"),
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


def _open_windows_regular_no_follow(path: Path) -> Any:
    import ctypes
    import msvcrt
    from ctypes import wintypes

    path_text = str(path)
    if path_text.startswith("\\\\"):
        raise ConformanceError(
            "DOCUMENT_PATH_UNSAFE",
            "Windows UNC, device, and extended paths are forbidden",
        )

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel32.CreateFileW
    create_file.argtypes = (
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    )
    create_file.restype = wintypes.HANDLE
    close_handle = kernel32.CloseHandle
    close_handle.argtypes = (wintypes.HANDLE,)
    close_handle.restype = wintypes.BOOL

    handle = create_file(
        path_text,
        0x80000000,
        0x00000001,
        None,
        3,
        0x00200000 | 0x08000000,
        None,
    )
    invalid_handle = ctypes.c_void_p(-1).value
    if handle == invalid_handle:
        raise OSError(ctypes.get_last_error(), "CreateFileW failed")

    class FileAttributeTagInfo(ctypes.Structure):
        _fields_ = [
            ("file_attributes", wintypes.DWORD),
            ("reparse_tag", wintypes.DWORD),
        ]

    info = FileAttributeTagInfo()
    get_info = kernel32.GetFileInformationByHandleEx
    get_info.argtypes = (
        wintypes.HANDLE,
        ctypes.c_int,
        wintypes.LPVOID,
        wintypes.DWORD,
    )
    get_info.restype = wintypes.BOOL
    if not get_info(handle, 9, ctypes.byref(info), ctypes.sizeof(info)):
        error_code = ctypes.get_last_error()
        close_handle(handle)
        raise OSError(error_code, "GetFileInformationByHandleEx failed")
    if info.file_attributes & 0x00000400:
        close_handle(handle)
        raise ConformanceError(
            "DOCUMENT_TYPE_INVALID",
            "symbolic links and reparse points are forbidden",
        )
    try:
        descriptor = msvcrt.open_osfhandle(
            handle,
            os.O_RDONLY | os.O_BINARY,
        )
    except OSError:
        close_handle(handle)
        raise
    return os.fdopen(descriptor, "rb")


def _open_regular_no_follow(path: Path) -> Any:
    if os.name == "nt":
        return _open_windows_regular_no_follow(path)
    flags = os.O_RDONLY
    for option in ("O_CLOEXEC", "O_NOFOLLOW", "O_NONBLOCK"):
        flags |= getattr(os, option, 0)
    descriptor = os.open(path, flags)
    return os.fdopen(descriptor, "rb")


def load_document(
    path: Path,
    *,
    max_bytes: int = MAX_DOCUMENT_BYTES,
) -> dict[str, Any]:
    try:
        with _open_regular_no_follow(path) as source:
            if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                raise ConformanceError(
                    "DOCUMENT_TYPE_INVALID",
                    f"document is not a regular file: {path}",
                )
            raw = source.read(max_bytes + 1)
    except ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise ConformanceError(
            "DOCUMENT_READ_FAILED",
            f"could not read {path}",
        ) from error
    if len(raw) > max_bytes:
        raise ConformanceError(
            "JSON_INPUT_TOO_LARGE",
            f"JSON input exceeds {max_bytes} bytes",
        )
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


def portable_glob_error(value: Any) -> str | None:
    if not isinstance(value, str) or not value:
        return "glob is empty or not a string"
    if len(value) > 512:
        return "glob exceeds 512 characters"
    if value != unicodedata.normalize("NFC", value):
        return "glob is not NFC-normalized"
    if (
        value.startswith("/")
        or re.match(r"^[A-Za-z]:", value)
        or "\\" in value
        or "//" in value
        or any(ord(character) < 32 or character in '<>:"|?' for character in value)
    ):
        return "glob is not portable"
    for segment in value.split("/"):
        if segment in {"", ".", ".."}:
            return "glob contains an empty or dot segment"
        if segment.endswith((" ", ".")):
            return "glob segment has a trailing dot or space"
        if not any(character in segment for character in "*[]{}"):
            basename = segment.split(".", 1)[0].upper()
            if basename in WINDOWS_RESERVED_BASENAMES:
                return "glob segment uses a Windows reserved basename"
    return None


def reject_execution_intent(case: dict[str, Any]) -> None:
    domain = case.get("domain")
    input_value = case.get("input")
    operation = input_value.get("operation") if isinstance(input_value, dict) else None
    capabilities = case.get("capabilities")
    capability_values = capabilities if isinstance(capabilities, list) else []
    cli_requires_execution = (
        domain == "cli"
        and isinstance(input_value, dict)
        and isinstance(input_value.get("options"), dict)
        and input_value["options"].get("requires_execution") is True
    )
    if (
        domain == "execution"
        or operation == "execution"
        or cli_requires_execution
        or any(
            isinstance(capability, str) and capability in EXECUTION_CAPABILITIES
            for capability in capability_values
        )
    ):
        raise ConformanceError(
            "EXECUTION_DISABLED",
            "the bootstrap runner never accepts execution intent",
        )


def reject_unmodeled_domain(case: dict[str, Any]) -> None:
    domain = case.get("domain")
    if domain not in BOOTSTRAP_DOMAINS:
        raise ConformanceError(
            "CASE_DOMAIN_UNMODELED",
            f"bootstrap runner has no closed schema for domain: {domain}",
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
            if normalized.startswith(f"{existing}/") or existing.startswith(
                f"{normalized}/"
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

    def reject_external_refs(value: Any) -> None:
        if isinstance(value, dict):
            for keyword in ("$ref", "$dynamicRef"):
                reference = value.get(keyword)
                if isinstance(reference, str) and not reference.startswith("#"):
                    raise ConformanceError(
                        "SCHEMA_REFERENCE_FORBIDDEN",
                        f"external schema reference is forbidden: {reference}",
                    )
            for item in value.values():
                reject_external_refs(item)
        elif isinstance(value, list):
            for item in value:
                reject_external_refs(item)

    reject_external_refs(schema)
    try:
        jsonschema.Draft202012Validator.check_schema(schema)
        validator = jsonschema.Draft202012Validator(schema)
        errors = sorted(
            validator.iter_errors(instance),
            key=lambda error: [str(part) for part in error.absolute_path],
        )
    except ConformanceError:
        raise
    except Exception as error:
        raise ConformanceError(
            "SCHEMA_VALIDATION_FAILED",
            "schema validation could not be completed safely",
        ) from error
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


def _pure_record(
    case: dict[str, Any],
    result: dict[str, Any],
) -> dict[str, Any]:
    return {
        "domain": case["domain"],
        "outcome": result["outcome"],
        "input": case["input"],
        "result": result["result"],
    }


def _validate_pure_domain_record(
    case: dict[str, Any],
    result: dict[str, Any],
    schema: dict[str, Any],
    code: str,
) -> None:
    if case["domain"] in PURE_DOMAINS:
        _validate_schema(_pure_record(case, result), schema, code)


def _package_index(
    packages: list[dict[str, Any]],
) -> dict[str, dict[str, Any]]:
    result: dict[str, dict[str, Any]] = {}
    normalized: set[str] = set()
    for package in packages:
        name = package["name"]
        identity = unicodedata.normalize("NFC", name).casefold()
        if identity in normalized:
            raise ConformanceError(
                "CASE_PACKAGE_DUPLICATE",
                f"duplicate normalized package identity: {name}",
            )
        normalized.add(identity)
        result[name] = package
    return result


def _validate_known_edges(
    edges: list[list[str]],
    package_names: set[str],
) -> None:
    adjacency = {name: [] for name in package_names}
    indegree = dict.fromkeys(package_names, 0)
    for edge in edges:
        if edge[0] == edge[1]:
            raise ConformanceError(
                "CASE_EDGE_SELF",
                f"self dependency edge is forbidden: {edge[0]}",
            )
        if edge[0] not in package_names or edge[1] not in package_names:
            raise ConformanceError(
                "CASE_EDGE_UNKNOWN",
                f"dependency edge references an unknown package: {edge}",
            )
        adjacency[edge[0]].append(edge[1])
        indegree[edge[1]] += 1

    ready = [name for name, degree in indegree.items() if degree == 0]
    visited = 0
    while ready:
        name = ready.pop()
        visited += 1
        for dependent in adjacency[name]:
            indegree[dependent] -= 1
            if indegree[dependent] == 0:
                ready.append(dependent)
    if visited != len(package_names):
        raise ConformanceError(
            "CASE_EDGE_CYCLE",
            "dependency edges contain a cycle",
        )


def _validate_unique_paths(
    paths: list[str],
    code: str,
) -> None:
    normalized: dict[str, str] = {}
    for path in paths:
        if error := portable_path_error(path):
            raise ConformanceError(code, f"unsafe nested path {path!r}: {error}")
        identity = unicodedata.normalize("NFC", path).casefold()
        if identity in normalized:
            raise ConformanceError(
                code,
                f"nested path collides with {normalized[identity]}: {path}",
            )
        for existing, original in normalized.items():
            if identity.startswith(f"{existing}/") or existing.startswith(
                f"{identity}/"
            ):
                raise ConformanceError(
                    code,
                    f"nested paths have a prefix conflict: {original} and {path}",
                )
        normalized[identity] = path


def _toolchain_for_language(language: str) -> str:
    toolchain = LANGUAGE_TOOLCHAINS.get(language)
    if toolchain is None:
        raise ConformanceError(
            "CASE_TOOLCHAIN_UNSUPPORTED",
            f"unsupported implementation language: {language}",
        )
    return toolchain


def _validate_pure_case_semantics(
    case: dict[str, Any],
    staged_files: list[WorkspaceFile],
) -> None:
    domain = case["domain"]
    if domain not in PURE_DOMAINS:
        return
    if any(file.executable for file in staged_files):
        raise ConformanceError(
            "CASE_PURE_AUTHORITY",
            "pure-domain workspace files must not be executable",
        )

    options = case["input"]["options"]
    if domain == "diff_selection":
        packages = options["packages"]
        by_name = _package_index(packages)
        roots = [package["rel_path"] for package in packages]
        _validate_unique_paths(roots, "CASE_NESTED_PATH_UNSAFE")
        for package in packages:
            for pattern in package.get("source_globs", []):
                if error := portable_glob_error(pattern):
                    raise ConformanceError(
                        "CASE_NESTED_GLOB_UNSAFE",
                        f"unsafe source glob {pattern!r}: {error}",
                    )
        _validate_known_edges(options["edges"], set(by_name))
        for name in options["forced_packages"]:
            if name not in by_name:
                raise ConformanceError(
                    "CASE_PACKAGE_REFERENCE_UNKNOWN",
                    f"forced package is not declared: {name}",
                )
    elif domain == "hashing_cache":
        workspace_paths = {entry.path for entry in staged_files}
        include_paths = options["include_paths"]
        _validate_unique_paths(include_paths, "CASE_NESTED_PATH_UNSAFE")
        for path in include_paths:
            if path not in workspace_paths:
                raise ConformanceError(
                    "CASE_HASH_PATH_UNKNOWN",
                    f"hash include path is not in the inline workspace: {path}",
                )
        dependency_names: set[str] = set()
        for dependency in options["dependency_digests"]:
            name = dependency["package"]
            if name == options["package"] or name in dependency_names:
                raise ConformanceError(
                    "CASE_HASH_DEPENDENCY_DUPLICATE",
                    f"invalid or duplicate hash dependency: {name}",
                )
            dependency_names.add(name)
        if options["package"] in options["dependents"]:
            raise ConformanceError(
                "CASE_HASH_DEPENDENT_SELF",
                "a package cannot be its own dependent",
            )
    elif domain == "starlark":
        workspace_paths = {entry.path for entry in staged_files}
        entrypoint = options["entrypoint"]
        if entrypoint not in workspace_paths:
            raise ConformanceError(
                "CASE_STARLARK_ENTRYPOINT_MISSING",
                f"Starlark entrypoint is not in the inline workspace: {entrypoint}",
            )
    elif domain == "sharding":
        by_name = _package_index(options["packages"])
        _validate_known_edges(options["edges"], set(by_name))
        for name in options["scheduled_packages"]:
            if name not in by_name:
                raise ConformanceError(
                    "CASE_PACKAGE_REFERENCE_UNKNOWN",
                    f"scheduled package is not declared: {name}",
                )
        selected = set(options["scheduled_packages"])
        pending = list(selected)
        prerequisites = {name: set() for name in by_name}
        for prerequisite, dependent in options["edges"]:
            prerequisites[dependent].add(prerequisite)
        while pending:
            name = pending.pop()
            for prerequisite in prerequisites[name]:
                if prerequisite not in selected:
                    selected.add(prerequisite)
                    pending.append(prerequisite)
        for name in selected:
            _toolchain_for_language(by_name[name]["language"])
    elif domain == "validation":
        by_name = _package_index(options["packages"])
        _validate_unique_paths(
            [package["rel_path"] for package in by_name.values()],
            "CASE_NESTED_PATH_UNSAFE",
        )
        _validate_known_edges(options["dependency_edges"], set(by_name))
        for package in by_name.values():
            for pattern in package["declared_srcs"]:
                if error := portable_glob_error(pattern):
                    raise ConformanceError(
                        "CASE_NESTED_GLOB_UNSAFE",
                        f"unsafe declared source {pattern!r}: {error}",
                    )
            for field in ("build_references", "declared_deps"):
                for name in package[field]:
                    if name not in by_name:
                        raise ConformanceError(
                            "CASE_PACKAGE_REFERENCE_UNKNOWN",
                            f"{field} references an unknown package: {name}",
                        )
    elif domain == "toolchain_detection":
        by_name = _package_index(options["packages"])
        selected = options["scheduled_packages"]
        if isinstance(selected, list):
            for name in selected:
                if name not in by_name:
                    raise ConformanceError(
                        "CASE_PACKAGE_REFERENCE_UNKNOWN",
                        f"scheduled package is not declared: {name}",
                    )


def _framed_digest(items: list[tuple[str, bytes]]) -> bytes:
    digest = hashlib.sha256()
    for name, content in sorted(items, key=lambda item: item[0]):
        name_bytes = name.encode("utf-8")
        digest.update(len(name_bytes).to_bytes(8, "big"))
        digest.update(name_bytes)
        digest.update(len(content).to_bytes(8, "big"))
        digest.update(content)
    return digest.digest()


def _portable_glob_matches(pattern: str, path: str) -> bool:
    pattern_segments = pattern.split("/")
    path_segments = path.split("/")

    @lru_cache(maxsize=None)
    def matches(pattern_index: int, path_index: int) -> bool:
        if pattern_index == len(pattern_segments):
            return path_index == len(path_segments)
        segment = pattern_segments[pattern_index]
        if segment == "**":
            return matches(pattern_index + 1, path_index) or (
                path_index < len(path_segments)
                and matches(pattern_index, path_index + 1)
            )
        return (
            path_index < len(path_segments)
            and fnmatch.fnmatchcase(path_segments[path_index], segment)
            and matches(pattern_index + 1, path_index + 1)
        )

    return matches(0, 0)


def _expected_diff_selection(
    options: dict[str, Any],
    changed_paths: list[str],
) -> tuple[set[str], set[str], set[str]] | None:
    packages = options["packages"]
    changed: set[str] = set(options["forced_packages"])
    unknown = False
    build_names = {
        "BUILD",
        "BUILD_windows",
        "BUILD_mac",
        "BUILD_linux",
        "BUILD_mac_and_linux",
    }
    for path in changed_paths:
        path_known = False
        for package in packages:
            root = package["rel_path"]
            if path != root and not path.startswith(f"{root}/"):
                continue
            path_known = True
            relative = path[len(root) :].lstrip("/")
            if package["source_mode"] == "package_prefix":
                changed.add(package["name"])
            elif relative in build_names or any(
                _portable_glob_matches(pattern, relative)
                for pattern in package["source_globs"]
            ):
                changed.add(package["name"])
        if not path_known:
            unknown = True

    package_names = {package["name"] for package in packages}
    if unknown:
        if options["unknown_path_policy"] == "error":
            return None
        changed = set(package_names)

    dependents: dict[str, set[str]] = {name: set() for name in package_names}
    prerequisites: dict[str, set[str]] = {name: set() for name in package_names}
    for prerequisite, dependent in options["edges"]:
        dependents[prerequisite].add(dependent)
        prerequisites[dependent].add(prerequisite)

    affected = set(changed)
    pending = list(changed)
    while pending:
        name = pending.pop()
        for dependent in dependents[name]:
            if dependent not in affected:
                affected.add(dependent)
                pending.append(dependent)

    closed = set(affected)
    pending = list(affected)
    while pending:
        name = pending.pop()
        for prerequisite in prerequisites[name]:
            if prerequisite not in closed:
                closed.add(prerequisite)
                pending.append(prerequisite)
    return changed, affected, closed - affected


def _expected_hashes(
    options: dict[str, Any],
    staged_files: list[WorkspaceFile],
) -> tuple[str, str, str]:
    workspace = {entry.path: entry.content for entry in staged_files}
    package_digest = _framed_digest(
        [(path, workspace[path]) for path in options["include_paths"]]
    )
    dependencies_digest = _framed_digest(
        [
            (entry["package"], bytes.fromhex(entry["digest"]))
            for entry in options["dependency_digests"]
        ]
    )
    combined = hashlib.sha256(package_digest + dependencies_digest).hexdigest()
    return package_digest.hex(), dependencies_digest.hex(), combined


def _render_display_command(command: dict[str, Any]) -> str:
    tokens = [command["program"], *command["args"]]
    rendered = [
        token
        if DISPLAY_SAFE_TOKEN.fullmatch(token)
        else json.dumps(token, ensure_ascii=False, separators=(",", ":"))
        for token in tokens
    ]
    return " ".join(rendered)


def _starlark_module_error(
    options: dict[str, Any],
    staged_files: list[WorkspaceFile],
) -> tuple[str, str] | None:
    sources = {entry.path: entry.content for entry in staged_files}
    pending = [(options["entrypoint"], 0)]
    visited: set[str] = set()
    limits = options["evaluation_limits"]
    while pending:
        module, depth = pending.pop()
        if module in visited:
            continue
        visited.add(module)
        if len(visited) > limits["module_count"]:
            return "STARLARK_MODULE_LIMIT", module
        if depth > limits["load_depth"]:
            return "STARLARK_LOAD_DEPTH_LIMIT", module
        try:
            source = sources[module].decode("utf-8", errors="strict")
        except UnicodeDecodeError:
            return "STARLARK_SOURCE_INVALID", module
        try:
            syntax = ast.parse(source, mode="exec")
        except SyntaxError:
            return "STARLARK_SOURCE_INVALID", module
        parents = {
            child: parent
            for parent in ast.walk(syntax)
            for child in ast.iter_child_nodes(parent)
        }
        top_level_loads: list[ast.Call] = []
        for statement in syntax.body:
            if not (
                isinstance(statement, ast.Expr)
                and isinstance(statement.value, ast.Call)
                and isinstance(statement.value.func, ast.Name)
                and statement.value.func.id == "load"
            ):
                continue
            top_level_loads.append(statement.value)
        top_level_load_ids = {id(call) for call in top_level_loads}
        for name in (
            node
            for node in ast.walk(syntax)
            if isinstance(node, ast.Name) and node.id == "load"
        ):
            parent = parents.get(name)
            if not (
                isinstance(parent, ast.Call)
                and parent.func is name
                and id(parent) in top_level_load_ids
            ):
                return "STARLARK_SOURCE_INVALID", module
        for call in top_level_loads:
            if not call.args:
                return "STARLARK_SOURCE_INVALID", module
            label_node = call.args[0]
            if not (
                isinstance(label_node, ast.Constant)
                and isinstance(label_node.value, str)
            ):
                return "STARLARK_SOURCE_INVALID", module
            label = label_node.value
            if label.startswith("//"):
                resolved = label[2:]
            elif label.startswith(("./", "../")):
                resolved = posixpath.normpath(
                    posixpath.join(posixpath.dirname(module), label)
                )
            else:
                resolved = label
            if (
                resolved in {"", ".", ".."}
                or resolved.startswith("../")
                or portable_path_error(resolved)
            ):
                return "STARLARK_LOAD_OUTSIDE_REPOSITORY", module
            if resolved not in sources:
                return "STARLARK_MODULE_MISSING", resolved
            pending.append((resolved, depth + 1))
    return None


def _expected_toolchains(
    options: dict[str, Any],
) -> dict[str, bool]:
    by_name = {package["name"]: package for package in options["packages"]}
    selected = options["scheduled_packages"]
    selected_names = sorted(by_name) if selected is None else selected
    enabled = {
        _toolchain_for_language(by_name[name]["language"]) for name in selected_names
    }
    enabled.update(options["forced_toolchains"])
    return {toolchain: toolchain in enabled for toolchain in TOOLCHAINS}


def _package_cost(package: dict[str, Any]) -> int:
    toolchain = _toolchain_for_language(package["language"])
    return 1 + package["build_command_count"] + TOOLCHAIN_WEIGHTS.get(toolchain, 0)


def _expected_shards(options: dict[str, Any]) -> list[dict[str, Any]]:
    packages = {package["name"]: package for package in options["packages"]}
    scheduled = options["scheduled_packages"]
    count = 1 if not scheduled else min(options["shard_count"], len(scheduled))
    assigned: list[list[str]] = [[] for _ in range(count)]
    direct_costs = [0] * count
    for name in sorted(
        scheduled,
        key=lambda item: (-_package_cost(packages[item]), item),
    ):
        index = min(range(count), key=lambda item: (direct_costs[item], item))
        assigned[index].append(name)
        direct_costs[index] += _package_cost(packages[name])

    prerequisites: dict[str, set[str]] = {name: set() for name in packages}
    for prerequisite, dependent in options["edges"]:
        prerequisites[dependent].add(prerequisite)

    def closure(roots: list[str]) -> set[str]:
        result = set(roots)
        pending = list(roots)
        while pending:
            package = pending.pop()
            for prerequisite in prerequisites[package]:
                if prerequisite not in result:
                    result.add(prerequisite)
                    pending.append(prerequisite)
        return result

    shards: list[dict[str, Any]] = []
    for index, roots in enumerate(assigned):
        package_names = closure(roots)
        toolchains = sorted(
            {
                _toolchain_for_language(packages[name]["language"])
                for name in package_names
            }
        )
        shards.append(
            {
                "index": index,
                "name": f"shard-{index + 1}-of-{count}",
                "assigned_packages": sorted(roots),
                "package_names": sorted(package_names),
                "toolchains": toolchains,
                "estimated_cost": sum(
                    _package_cost(packages[name]) for name in package_names
                ),
            }
        )
    return shards


def _validate_pure_result_semantics(
    case: dict[str, Any],
    result: dict[str, Any],
    staged_files: list[WorkspaceFile],
    prefix: str,
) -> None:
    domain = case["domain"]
    if domain not in PURE_DOMAINS:
        return
    options = case["input"]["options"]
    payload = result["result"]
    outcome = result["outcome"]
    diagnostic_codes = [item["code"] for item in result["diagnostics"]]

    if domain == "diff_selection":
        expected_sets = _expected_diff_selection(
            options,
            case["input"]["changed_paths"],
        )
        if expected_sets is None:
            if outcome != "error" or "DIFF_UNKNOWN_PATH" not in diagnostic_codes:
                raise ConformanceError(
                    f"{prefix}_DIFF_UNKNOWN_PATH_INVALID",
                    "unknown changed paths require a stable error",
                )
            return
        if outcome != "ok":
            return
        actual_sets = (
            set(payload["changed_packages"]),
            set(payload["affected_packages"]),
            set(payload["prerequisite_packages"]),
        )
        if actual_sets != expected_sets:
            raise ConformanceError(
                f"{prefix}_DIFF_RESULT_INVALID",
                "diff result does not match changed, affected, and prerequisite closure",
            )
    elif domain == "hashing_cache" and outcome == "ok":
        expected_hashes = _expected_hashes(options, staged_files)
        actual_hashes = (
            payload["package_digest"],
            payload["dependencies_digest"],
            payload["combined_digest"],
        )
        allowed_invalidations = {
            options["package"],
            *options["dependents"],
        }
        if actual_hashes != expected_hashes:
            raise ConformanceError(
                f"{prefix}_HASH_MISMATCH",
                "hash result does not match the framed SHA-256 oracle",
            )
        prior = options["prior_cache"]
        if prior["state"] == "corrupt":
            expected_status = "recovered"
            expected_invalidations = allowed_invalidations
        elif prior["state"] == "missing":
            expected_status = "miss"
            expected_invalidations = allowed_invalidations
        elif (
            prior["status"] == "success"
            and prior["combined_digest"] == expected_hashes[2]
        ):
            expected_status = "hit"
            expected_invalidations = set()
        else:
            expected_status = "miss"
            expected_invalidations = allowed_invalidations
        if (
            payload["cache_status"] != expected_status
            or set(payload["invalidated_packages"]) != expected_invalidations
        ):
            raise ConformanceError(
                f"{prefix}_HASH_INVALIDATION_INVALID",
                "cache status and invalidations do not match the prior record",
            )
        if prior["state"] == "corrupt" and (
            "CACHE_CORRUPT_RECOVERED" not in diagnostic_codes
        ):
            raise ConformanceError(
                f"{prefix}_HASH_RECOVERY_DIAGNOSTIC_MISSING",
                "corrupt cache recovery requires a stable diagnostic",
            )
    elif domain == "starlark":
        module_error = _starlark_module_error(options, staged_files)
        if module_error is not None:
            code, path = module_error
            matching = [
                diagnostic
                for diagnostic in result["diagnostics"]
                if diagnostic["code"] == code and diagnostic.get("path") == path
            ]
            if outcome != "error" or not matching:
                raise ConformanceError(
                    f"{prefix}_STARLARK_MODULE_ERROR_INVALID",
                    f"Starlark module resolution must report {code}: {path}",
                )
            return
        if outcome != "ok":
            return
        target_ids: set[tuple[str, str]] = set()
        for target in payload["targets"]:
            identity = (target["rule"], target["name"])
            if identity in target_ids:
                raise ConformanceError(
                    f"{prefix}_STARLARK_TARGET_DUPLICATE",
                    f"duplicate Starlark target: {identity}",
                )
            target_ids.add(identity)
            rendered = [
                _render_display_command(command) for command in target["commands"]
            ]
            if rendered != target["rendered_commands"]:
                raise ConformanceError(
                    f"{prefix}_STARLARK_RENDER_MISMATCH",
                    "rendered commands do not match structured commands",
                )
            if (
                target["command_source"] == "legacy_fallback"
                and options["legacy_fallback"] == "error"
            ):
                raise ConformanceError(
                    f"{prefix}_STARLARK_FALLBACK_FORBIDDEN",
                    "legacy fallback was used under the error policy",
                )
    elif domain == "sharding":
        shard_count = options["shard_count"]
        scheduled = options["scheduled_packages"]
        produced_count = 1 if not scheduled else min(shard_count, len(scheduled))
        index = options.get("shard_index")
        invalid_code = None
        if shard_count < 1:
            invalid_code = "SHARD_COUNT_INVALID"
        elif index is not None and (index < 0 or index >= produced_count):
            invalid_code = "SHARD_INDEX_INVALID"
        if invalid_code is not None:
            if outcome != "error" or diagnostic_codes != [invalid_code]:
                raise ConformanceError(
                    f"{prefix}_SHARD_ERROR_INVALID",
                    f"invalid shard input must report {invalid_code}",
                )
        elif outcome == "ok":
            expected = _expected_shards(options)
            actual = sorted(payload["shards"], key=lambda item: item["index"])
            normalized = []
            for shard in actual:
                copy_value = dict(shard)
                for key in (
                    "assigned_packages",
                    "package_names",
                    "toolchains",
                ):
                    copy_value[key] = sorted(copy_value[key])
                normalized.append(copy_value)
            if normalized != expected:
                raise ConformanceError(
                    f"{prefix}_SHARD_MISMATCH",
                    "shard result does not match the closed balancing oracle",
                )
    elif domain == "validation":
        codes = payload.get("diagnostic_codes", [])
        valid = payload.get("valid")
        if set(options["checks"]) == {"build_file_presence"}:
            expected_codes = sorted(
                {
                    {
                        "missing": "BUILD_FILE_MISSING",
                        "empty": "BUILD_FILE_EMPTY",
                    }[package["build_file_state"]]
                    for package in options["packages"]
                    if package["build_file_state"] != "present"
                }
            )
            if sorted(codes) != expected_codes:
                raise ConformanceError(
                    f"{prefix}_VALIDATION_INCONSISTENT",
                    "build-file diagnostics do not match the normalized snapshot",
                )
        consistent = (
            outcome == "ok" and valid is True and not codes and not diagnostic_codes
        ) or (
            outcome == "error"
            and valid is False
            and bool(codes)
            and set(codes) == set(diagnostic_codes)
        )
        if not consistent:
            raise ConformanceError(
                f"{prefix}_VALIDATION_INCONSISTENT",
                "validation outcome, valid flag, and diagnostics disagree",
            )
    elif domain == "toolchain_detection":
        packages = {package["name"]: package for package in options["packages"]}
        selected = options["scheduled_packages"]
        selected_names = packages if selected is None else selected
        unsupported = any(
            packages[name]["language"] not in LANGUAGE_TOOLCHAINS
            for name in selected_names
        ) or any(
            toolchain not in TOOLCHAINS for toolchain in options["forced_toolchains"]
        )
        if unsupported:
            if outcome != "error" or "TOOLCHAIN_UNSUPPORTED" not in diagnostic_codes:
                raise ConformanceError(
                    f"{prefix}_TOOLCHAIN_ERROR_INVALID",
                    "unsupported toolchains require a stable error",
                )
        elif outcome != "ok" or payload["toolchains"] != _expected_toolchains(options):
            raise ConformanceError(
                f"{prefix}_TOOLCHAIN_MISMATCH",
                "toolchain map does not match the selected packages",
            )
    elif domain == "cli":
        expected_exit = {
            "success": 0,
            "package_failure": 1,
            "validation_failure": 1,
            "invalid_usage": 2,
            "unsafe_input": 2,
        }[options["condition"]]
        expected_outcome = "ok" if expected_exit == 0 else "error"
        if payload["exit_code"] != expected_exit or outcome != expected_outcome:
            raise ConformanceError(
                f"{prefix}_CLI_EXIT_MISMATCH",
                "CLI condition, outcome, and exit code disagree",
            )


def _sort_json_objects(value: Any) -> Any:
    if isinstance(value, dict):
        return {key: _sort_json_objects(value[key]) for key in sorted(value)}
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
    elif domain == "diff_selection":
        for key in (
            "changed_packages",
            "affected_packages",
            "prerequisite_packages",
        ):
            if key in payload:
                payload[key].sort()
    elif domain == "hashing_cache":
        if "invalidated_packages" in payload:
            payload["invalidated_packages"].sort()
    elif domain == "starlark" and "targets" in payload:
        payload["targets"].sort(key=lambda target: (target["rule"], target["name"]))
        for target in payload["targets"]:
            target["srcs"].sort()
            target["deps"].sort()
    elif domain == "sharding" and "shards" in payload:
        payload["shards"].sort(key=lambda shard: shard["index"])
        for shard in payload["shards"]:
            shard["assigned_packages"].sort()
            shard["package_names"].sort()
            shard["toolchains"].sort()
    elif domain == "validation" and "diagnostic_codes" in payload:
        payload["diagnostic_codes"].sort()

    return _sort_json_objects(canonical)


def _validate_result_shape(
    case: dict[str, Any],
    result: dict[str, Any],
    *,
    result_schema: dict[str, Any],
    plan_schema: dict[str, Any],
    pure_domain_schema: dict[str, Any],
    code: str,
) -> None:
    _validate_schema(result, result_schema, code)
    pure_code = (
        "RESULT_PURE_SCHEMA_INVALID"
        if code == "RESULT_SCHEMA_INVALID"
        else "EXPECTED_PURE_SCHEMA_INVALID"
    )
    _validate_pure_domain_record(
        case,
        result,
        pure_domain_schema,
        pure_code,
    )
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
    diagnostic_identities = [
        (
            item["code"],
            item.get("path", ""),
            item.get("package", ""),
            json.dumps(item.get("details", {}), sort_keys=True),
        )
        for item in result["diagnostics"]
    ]
    if len(diagnostic_identities) != len(set(diagnostic_identities)):
        raise ConformanceError(
            "RESULT_DIAGNOSTIC_DUPLICATE",
            "result contains a duplicate diagnostic identity",
        )
    if (
        result["outcome"] in {"error", "unsupported", "skipped"}
        and not result["diagnostics"]
    ):
        raise ConformanceError(
            "RESULT_DIAGNOSTIC_MISSING",
            f"{result['outcome']} requires a diagnostic",
        )
    if result["outcome"] == "ok" and result["domain"] == "discovery":
        names = [package["name"] for package in payload["packages"]]
        if len(names) != len(set(names)):
            raise ConformanceError(
                "RESULT_PACKAGE_NAME_DUPLICATE",
                "discovery result contains a duplicate package name",
            )
    if result["outcome"] == "ok" and result["domain"] == "graph":
        flattened = [package for level in payload["levels"] for package in level]
        if len(flattened) != len(set(flattened)):
            raise ConformanceError(
                "RESULT_GRAPH_LEVEL_DUPLICATE",
                "a graph package appears in more than one level",
            )
        level_packages = set(flattened)
        edge_packages = {package for edge in payload["edges"] for package in edge}
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
    pure_domain_schema: dict[str, Any] | None = None,
) -> dict[str, Any]:
    reject_execution_intent(case)
    reject_unmodeled_domain(case)
    result_schema = result_schema or load_document(
        DEFAULT_FIXTURE_ROOT / "result.schema.json"
    )
    plan_schema = plan_schema or load_document(
        REPO_ROOT / "code" / "specs" / "schemas" / "build-plan-v1.schema.json"
    )
    pure_domain_schema = pure_domain_schema or load_document(
        DEFAULT_FIXTURE_ROOT / "pure-domains.schema.json"
    )
    _validate_result_shape(
        case,
        actual,
        result_schema=result_schema,
        plan_schema=plan_schema,
        pure_domain_schema=pure_domain_schema,
        code="RESULT_SCHEMA_INVALID",
    )
    staged_files = preflight_workspace(case)
    _validate_pure_case_semantics(case, staged_files)
    _validate_pure_result_semantics(
        case,
        actual,
        staged_files,
        "RESULT",
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
    pure_domain_schema: dict[str, Any] | None = None,
) -> list[WorkspaceFile]:
    reject_execution_intent(case)
    _validate_schema(case, case_schema, "CASE_SCHEMA_INVALID")
    reject_unmodeled_domain(case)
    _validate_case_identity(case)
    _validate_input_paths(case)
    staged_files = preflight_workspace(case)
    pure_domain_schema = pure_domain_schema or load_document(
        DEFAULT_FIXTURE_ROOT / "pure-domains.schema.json"
    )
    _validate_pure_case_semantics(case, staged_files)
    _validate_result_shape(
        case,
        case["expected"],
        result_schema=result_schema,
        plan_schema=plan_schema,
        pure_domain_schema=pure_domain_schema,
        code="EXPECTED_SCHEMA_INVALID",
    )
    _validate_pure_result_semantics(
        case,
        case["expected"],
        staged_files,
        "EXPECTED",
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
            item["adapter_status"] == "ready" for item in implementations
        ),
    }


def validate_corpus(
    fixture_root: Path = DEFAULT_FIXTURE_ROOT,
) -> dict[str, Any]:
    fixture_root = fixture_root.resolve()
    case_schema = load_document(DEFAULT_FIXTURE_ROOT / "schema.json")
    result_schema = load_document(DEFAULT_FIXTURE_ROOT / "result.schema.json")
    pure_domain_schema = load_document(
        DEFAULT_FIXTURE_ROOT / "pure-domains.schema.json"
    )
    manifest_schema = load_document(
        DEFAULT_FIXTURE_ROOT / "implementations.schema.json"
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
    validated_files = 0
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
            pure_domain_schema=pure_domain_schema,
        )
        case_ids.add(case["id"])
        domains.add(case["domain"])
        validated_files += len(staged_files)

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
        "validated_file_count": validated_files,
        "domains": sorted(domains),
        "status": "valid",
    }


def validate_result_files(case_path: Path, result_path: Path) -> dict[str, Any]:
    case = load_document(case_path)
    reject_execution_intent(case)
    case_schema = load_document(DEFAULT_FIXTURE_ROOT / "schema.json")
    result_schema = load_document(DEFAULT_FIXTURE_ROOT / "result.schema.json")
    pure_domain_schema = load_document(
        DEFAULT_FIXTURE_ROOT / "pure-domains.schema.json"
    )
    plan_schema = load_document(
        REPO_ROOT / "code" / "specs" / "schemas" / "build-plan-v1.schema.json"
    )
    validate_case_document(
        case,
        case_schema=case_schema,
        result_schema=result_schema,
        plan_schema=plan_schema,
        pure_domain_schema=pure_domain_schema,
    )
    output_limit = min(case["limits"]["output_bytes"], MAX_RESULT_BYTES)
    result = load_document(result_path, max_bytes=output_limit)
    return assert_result_matches(
        case,
        result,
        result_schema=result_schema,
        plan_schema=plan_schema,
        pure_domain_schema=pure_domain_schema,
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate build-tool conformance fixtures and results."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    corpus_parser = subparsers.add_parser(
        "validate-corpus",
        help="Validate the non-execution corpus without filesystem staging.",
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
    exit_code = 0
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
                "outcome": canonical["outcome"],
                "status": "matched",
                "conformance_status": (
                    "non-passing"
                    if canonical["outcome"] in {"unsupported", "skipped"}
                    else "pass"
                ),
            }
            if output["conformance_status"] == "non-passing":
                exit_code = 1
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
        return 1 if error.code.startswith("RESULT_") else 2

    print(json.dumps(output, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
