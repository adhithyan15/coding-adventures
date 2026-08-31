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
from collections.abc import Sequence
from dataclasses import dataclass
from functools import lru_cache
from itertools import pairwise
from pathlib import Path
from typing import Any

import tracked_artifact_unicode17 as tracked_unicode

REPO_ROOT = Path(__file__).resolve().parents[2]
DEFAULT_FIXTURE_ROOT = REPO_ROOT / "code" / "specs" / "fixtures" / "build-tool-v1"
MAX_SAFE_INTEGER = 9_007_199_254_740_991
MAX_DOCUMENT_BYTES = 2_000_000
MAX_REPOSITORY_SOURCE_DOCUMENT_BYTES = 10_000_000
MAX_RESULT_BYTES = 16_777_216
MAX_JSON_DEPTH = 64
MAX_WORKSPACE_FILES = 4096
MAX_WORKSPACE_BYTES = 268_435_456
MAX_SOURCE_INPUT_SELECTORS = 4096
MAX_REPOSITORY_SOURCE_SCOPES = 8192
MAX_REPOSITORY_SOURCE_AUTHORIZATIONS = 32768
SOURCE_INPUT_REGISTRY_DOMAIN = (
    b"coding-adventures/build-tool-language-source-input-registry/v1\0"
)
REPOSITORY_SOURCE_INPUT_BOUNDARY_DOMAIN = (
    b"coding-adventures/build-tool-repository-source-input-boundary/v1\0"
)
RESERVED_ADAPTER_FLAGS = ("--conformance", "--workspace-root", "--output")
CLI_MAX_ARGUMENTS = 64
CLI_MAX_ARGUMENT_CHARACTERS = 256
CLI_MAX_ARGUMENT_BYTES = 4096
CLI_LANGUAGES = (
    "all",
    "c",
    "cpp",
    "csharp",
    "dart",
    "dotnet",
    "elixir",
    "fsharp",
    "go",
    "haskell",
    "java",
    "kotlin",
    "lua",
    "mosaic",
    "ocaml",
    "perl",
    "python",
    "ruby",
    "rust",
    "starlark",
    "swift",
    "twig",
    "typescript",
    "wasm",
)
EXECUTION_CAPABILITIES = {"execution", "trusted_execution"}
PURE_DOMAINS = {
    "cli",
    "diff_selection",
    "hashing_cache",
    "sharding",
    "source_collection",
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
    "source_collection": {"source_collection"},
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
MAX_TOOLCHAIN_BUILD_BYTES = 65_536
MAX_TOOLCHAIN_BUILD_LINES = 4_096
MAX_TOOLCHAIN_SNAPSHOT_BYTES = 1_048_576
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
ORPHAN_SCAN_ROOT = "code"
ORPHAN_LEDGER_PATH = "code/BUILD-EXEMPTIONS"
ORPHAN_BUILD_NAMES = (
    "BUILD",
    "BUILD_windows",
    "BUILD_mac",
    "BUILD_linux",
    "BUILD_mac_and_linux",
)
ORPHAN_SKIP_COMPONENTS = frozenset(
    {
        ".git",
        "target",
        "node_modules",
        "vendor",
        ".venv",
        "_build",
        "deps",
        ".build",
        "dist-newstyle",
        ".cargo",
    }
)
SOURCE_COLLECTION_SKIP_COMPONENTS = frozenset(
    {
        ".git",
        ".hg",
        ".svn",
        ".venv",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".stack-work",
        "__pycache__",
        "node_modules",
        "vendor",
        "dist",
        "dist-newstyle",
        "_build",
        "build",
        "target",
        ".claude",
        "Pods",
        ".gradle",
        ".dart_tool",
        "gradle-build",
        "deps",
        ".build",
        ".cargo",
        "cover",
    }
)
SOURCE_INPUT_SELECTOR_FIELDS = (
    "recursive_suffixes",
    "recursive_exact_basenames",
    "root_exact_basenames",
    "root_variable_suffixes",
    "root_exact_relative_paths",
)
TRACKED_ARTIFACT_COMPONENT_IDENTITY = "node_modules"
TRACKED_ARTIFACT_REDACTED_PATH = "repository"
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


def load_case_document(path: Path) -> dict[str, Any]:
    """Load one case under the narrow larger bound needed by exact source bytes."""

    max_bytes = (
        MAX_REPOSITORY_SOURCE_DOCUMENT_BYTES
        if path.name.startswith("source-collection-repository-")
        else MAX_DOCUMENT_BYTES
    )
    return load_document(path, max_bytes=max_bytes)


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
        if segment in {"", ".", ".."}:
            return "path contains an empty or dot segment"
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


def _utf8_sorted(values: Sequence[str]) -> list[str]:
    return sorted(values, key=lambda value: value.encode("utf-8"))


def source_input_registry_digest(registry: dict[str, Any]) -> str:
    """Return the versioned digest that pins a source-input registry snapshot."""

    encoded = json.dumps(
        registry,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    framed = SOURCE_INPUT_REGISTRY_DOMAIN + len(encoded).to_bytes(8, "big") + encoded
    return hashlib.sha256(framed).hexdigest()


def repository_source_input_boundary_digest(boundary: dict[str, Any]) -> str:
    """Return the versioned digest of an exact repository-input boundary."""

    encoded = json.dumps(
        boundary,
        ensure_ascii=False,
        separators=(",", ":"),
        sort_keys=True,
    ).encode("utf-8")
    framed = (
        REPOSITORY_SOURCE_INPUT_BOUNDARY_DOMAIN
        + len(encoded).to_bytes(8, "big")
        + encoded
    )
    return hashlib.sha256(framed).hexdigest()


def _source_input_text_error(value: str) -> str | None:
    if value != unicodedata.normalize("NFC", value):
        return "value is not NFC-normalized"
    if any(unicodedata.category(character).startswith("C") for character in value):
        return "value contains a control, format, surrogate, or private-use character"
    return None


def _repository_source_sensitive_path(path: str) -> bool:
    """Reject credential, secret, signing, and machine-local exact inputs."""

    components = [component.casefold() for component in path.split("/")]
    basename = components[-1]
    blocked_names = {
        ".env",
        ".envrc",
        ".git-credentials",
        ".netrc",
        ".npmrc",
        ".pypirc",
        "credentials",
        "credentials.json",
        "credentials.toml",
        "id_ed25519",
        "id_rsa",
        "key.properties",
        "local.properties",
        "secrets.json",
        "secrets.toml",
        "signing.properties",
    }
    blocked_components = {".aws", ".azure", ".gnupg", ".ssh", "secrets"}
    blocked_suffixes = (".jks", ".key", ".keystore", ".p12", ".pem", ".pfx")
    sensitive_word = re.compile(
        r"(?:^|[._-])(credential|password|private[-_]?key|secret|signing|token)(?:[._-]|$)"
    )
    machine_local_word = re.compile(r"(?:^|[._-])local(?:[._-]|$)")
    return (
        basename in blocked_names
        or basename.startswith(".env.")
        or any(component in blocked_components for component in components)
        or basename.endswith(blocked_suffixes)
        or sensitive_word.search(basename) is not None
        or machine_local_word.search(basename) is not None
    )


def _source_input_selector_error(value: str, field: str) -> str | None:
    if error := _source_input_text_error(value):
        return error
    if field in {"recursive_suffixes", "root_variable_suffixes", "suffixes"}:
        if not re.fullmatch(r"\.[A-Za-z0-9][A-Za-z0-9._+-]*", value):
            return "suffix is not portable"
        return None
    if field == "root_exact_relative_paths":
        return portable_path_error(value)
    if "/" in value or "\\" in value:
        return "basename contains a separator"
    return portable_path_error(value)


def _validate_source_input_registry(
    registry: dict[str, Any],
    schema: dict[str, Any],
) -> dict[str, int]:
    """Validate the closed, canonical registry before any case can consume it."""

    _validate_schema(registry, schema, "SOURCE_INPUT_REGISTRY_SCHEMA_INVALID")
    languages = registry.get("languages", [])
    if not isinstance(languages, list):
        raise TypeError("validated registry languages must be an array")

    names = [
        entry.get("language")
        for entry in languages
        if isinstance(entry, dict) and isinstance(entry.get("language"), str)
    ]
    if len(names) != len(set(names)):
        raise ConformanceError(
            "SOURCE_INPUT_LANGUAGE_DUPLICATE",
            "source-input registry language keys must be unique",
        )
    expected_languages = set(CLI_LANGUAGES) - {"all"}
    if set(names) != expected_languages:
        raise ConformanceError(
            "SOURCE_INPUT_LANGUAGE_SET",
            "source-input registry must cover every canonical CLI language exactly once",
        )
    if names != _utf8_sorted(names):
        raise ConformanceError(
            "SOURCE_INPUT_NOT_CANONICAL",
            "source-input registry languages are not in UTF-8 byte order",
        )

    for entry in languages:
        for field in SOURCE_INPUT_SELECTOR_FIELDS:
            for value in entry.get(field, []):
                if isinstance(value, str) and (
                    error := _source_input_selector_error(value, field)
                ):
                    raise ConformanceError(
                        "SOURCE_INPUT_PATH_UNSAFE",
                        f"unsafe {entry.get('language')} {field} selector {value!r}: {error}",
                    )
        for scoped in entry.get("scoped_inputs", []):
            if not isinstance(scoped, dict):
                continue
            if not scoped.get("owner") or not scoped.get("reason"):
                raise ConformanceError(
                    "SOURCE_INPUT_SCOPE_CLASSIFICATION_INVALID",
                    "scoped source inputs require a durable owner and reason",
                )
            prefix = scoped.get("path_prefix")
            if isinstance(prefix, str) and (error := portable_path_error(prefix)):
                raise ConformanceError(
                    "SOURCE_INPUT_PATH_UNSAFE",
                    f"unsafe scoped source-input prefix {prefix!r}: {error}",
                )

    universal = registry["universal_inputs"]
    universal_expected = {
        "build_filenames": _utf8_sorted(list(ORPHAN_BUILD_NAMES)),
        "generated_directory_components": _utf8_sorted(
            list(SOURCE_COLLECTION_SKIP_COMPONENTS)
        ),
        "root_exact_basenames": ["required_capabilities.json"],
    }
    selector_count = 0

    def enforce_selector_limit() -> None:
        if selector_count > MAX_SOURCE_INPUT_SELECTORS:
            raise ConformanceError(
                "SOURCE_INPUT_SELECTOR_LIMIT",
                "source-input registry exceeds the aggregate selector limit",
            )

    for field, values in universal.items():
        if values != _utf8_sorted(values) or len(values) != len(set(values)):
            raise ConformanceError(
                "SOURCE_INPUT_NOT_CANONICAL",
                f"universal source-input field {field} is not unique UTF-8 byte order",
            )
        if field in universal_expected and values != universal_expected[field]:
            raise ConformanceError(
                "SOURCE_INPUT_UNIVERSAL_DRIFT",
                f"universal source-input field {field} drifted from the v1 contract",
            )
        for value in values:
            if error := _source_input_selector_error(
                value,
                "recursive_exact_basenames",
            ):
                raise ConformanceError(
                    "SOURCE_INPUT_PATH_UNSAFE",
                    f"unsafe universal source-input selector {value!r}: {error}",
                )
        selector_count += len(values)
        enforce_selector_limit()

    for entry in languages:
        language = entry["language"]
        alias_values: dict[str, frozenset[str]] = {}
        seen_alias_groups: set[tuple[str, ...]] = set()
        encoded_alias_groups = [
            json.dumps(group, ensure_ascii=False, separators=(",", ":")).encode("utf-8")
            for group in entry["case_alias_groups"]
        ]
        if encoded_alias_groups != sorted(encoded_alias_groups):
            raise ConformanceError(
                "SOURCE_INPUT_NOT_CANONICAL",
                f"{language} case-alias groups are not in UTF-8 byte order",
            )
        for group in entry["case_alias_groups"]:
            if group != _utf8_sorted(group):
                raise ConformanceError(
                    "SOURCE_INPUT_NOT_CANONICAL",
                    f"{language} case-alias group is not in UTF-8 byte order",
                )
            identities = {value.casefold() for value in group}
            if len(identities) != 1:
                raise ConformanceError(
                    "SOURCE_INPUT_ALIAS_INVALID",
                    f"{language} case-alias group does not identify one portable basename",
                )
            identity = tuple(group)
            if identity in seen_alias_groups:
                raise ConformanceError(
                    "SOURCE_INPUT_ALIAS_INVALID",
                    f"{language} repeats a case-alias group",
                )
            seen_alias_groups.add(identity)
            folded_identity = next(iter(identities))
            if folded_identity in alias_values:
                raise ConformanceError(
                    "SOURCE_INPUT_ALIAS_INVALID",
                    f"{language} repeats a normalized case-alias identity",
                )
            alias_values[folded_identity] = frozenset(group)
            selector_count += len(group)
            enforce_selector_limit()

        normalized_roles: dict[str, str] = {}
        normalized_values: dict[str, set[str]] = {}
        for value in universal["build_filenames"]:
            normalized_roles[value.casefold()] = "universal_build_filenames"
        for value in universal["root_exact_basenames"]:
            normalized_roles[value.casefold()] = "universal_root_exact_basenames"
        for field in SOURCE_INPUT_SELECTOR_FIELDS:
            values = entry[field]
            if values != _utf8_sorted(values) or len(values) != len(set(values)):
                raise ConformanceError(
                    "SOURCE_INPUT_NOT_CANONICAL",
                    f"{language} {field} is not unique UTF-8 byte order",
                )
            selector_count += len(values)
            enforce_selector_limit()
            for value in values:
                identity = unicodedata.normalize("NFC", value).casefold()
                prior = normalized_roles.get(identity)
                if prior is not None and prior != field:
                    raise ConformanceError(
                        "SOURCE_INPUT_SELECTOR_COLLISION",
                        f"{language} selector {value!r} collides across {prior} and {field}",
                    )
                if prior == field:
                    observed = normalized_values.get(identity, set()) | {value}
                    allowed = alias_values.get(identity)
                    if allowed is None or not observed.issubset(allowed):
                        raise ConformanceError(
                            "SOURCE_INPUT_SELECTOR_COLLISION",
                            f"{language} selector {value!r} collides after case folding",
                        )
                normalized_roles[identity] = field
                normalized_values.setdefault(identity, set()).add(value)

        for identity, allowed in alias_values.items():
            if normalized_values.get(identity) != set(allowed):
                raise ConformanceError(
                    "SOURCE_INPUT_ALIAS_INVALID",
                    f"{language} case-alias group contains an undeclared selector",
                )

        paths = entry["root_exact_relative_paths"]
        for path in paths:
            if any(
                component in SOURCE_COLLECTION_SKIP_COMPONENTS
                for component in path.split("/")
            ):
                raise ConformanceError(
                    "SOURCE_INPUT_PATH_UNSAFE",
                    f"{language} exact path enters a generated component: {path}",
                )
        folded_paths = [path.casefold() for path in paths]
        for index, path in enumerate(folded_paths):
            if any(
                path.startswith(f"{other}/") or other.startswith(f"{path}/")
                for other in folded_paths[index + 1 :]
            ):
                raise ConformanceError(
                    "SOURCE_INPUT_SELECTOR_COLLISION",
                    f"{language} exact relative paths have a prefix collision",
                )

        scoped_ids = [item["id"] for item in entry["scoped_inputs"]]
        if scoped_ids != _utf8_sorted(scoped_ids) or len(scoped_ids) != len(
            set(scoped_ids)
        ):
            raise ConformanceError(
                "SOURCE_INPUT_NOT_CANONICAL",
                f"{language} scoped inputs are not unique id order",
            )
        for scoped in entry["scoped_inputs"]:
            prefix = scoped.get("path_prefix")
            if prefix is not None:
                if error := _source_input_text_error(prefix):
                    raise ConformanceError(
                        "SOURCE_INPUT_PATH_UNSAFE",
                        f"unsafe scoped prefix {prefix!r}: {error}",
                    )
                if any(
                    component in SOURCE_COLLECTION_SKIP_COMPONENTS
                    for component in prefix.split("/")
                ):
                    raise ConformanceError(
                        "SOURCE_INPUT_PATH_UNSAFE",
                        f"{language} scoped input enters generated component {prefix}",
                    )
            for field in ("suffixes", "exact_basenames"):
                values = scoped[field]
                if values != _utf8_sorted(values) or len(values) != len(set(values)):
                    raise ConformanceError(
                        "SOURCE_INPUT_NOT_CANONICAL",
                        f"{language} scoped input {scoped['id']} {field} is not canonical",
                    )
                for value in values:
                    if error := _source_input_selector_error(value, field):
                        raise ConformanceError(
                            "SOURCE_INPUT_PATH_UNSAFE",
                            f"unsafe scoped selector {value!r}: {error}",
                        )
                selector_count += len(values)
                enforce_selector_limit()
            if any(
                basename.endswith(suffix)
                for basename in scoped["exact_basenames"]
                for suffix in scoped["suffixes"]
            ):
                raise ConformanceError(
                    "SOURCE_INPUT_SELECTOR_COLLISION",
                    f"{language} scoped input {scoped['id']} has redundant exact and suffix selectors",
                )

        scoped_inputs = entry["scoped_inputs"]
        for index, scoped in enumerate(scoped_inputs):
            scoped_prefix = scoped.get("path_prefix", "")
            scoped_selectors = {
                value.casefold()
                for field in ("suffixes", "exact_basenames")
                for value in scoped[field]
            }
            if scoped["scope"] == "root":
                for value in scoped["exact_basenames"]:
                    prior = normalized_roles.get(value.casefold())
                    if prior is not None:
                        raise ConformanceError(
                            "SOURCE_INPUT_SELECTOR_COLLISION",
                            f"{language} root scoped selector {value!r} collides with {prior}",
                        )
            for other in scoped_inputs[index + 1 :]:
                other_prefix = other.get("path_prefix", "")
                other_selectors = {
                    value.casefold()
                    for field in ("suffixes", "exact_basenames")
                    for value in other[field]
                }
                if not scoped_selectors.intersection(other_selectors):
                    continue
                same_scope = scoped["scope"] == other["scope"] == "root"
                overlapping_subtrees = scoped["scope"] == other[
                    "scope"
                ] == "subtree" and (
                    scoped_prefix == other_prefix
                    or scoped_prefix.startswith(f"{other_prefix}/")
                    or other_prefix.startswith(f"{scoped_prefix}/")
                )
                if same_scope or overlapping_subtrees:
                    raise ConformanceError(
                        "SOURCE_INPUT_SELECTOR_COLLISION",
                        f"{language} scoped inputs {scoped['id']} and {other['id']} overlap",
                    )

        selectors: list[tuple[str, str, str, str, str]] = []

        def add_selectors(
            target: list[tuple[str, str, str, str, str]],
            role: str,
            scope: str,
            prefix: str,
            matcher: str,
            values: list[str],
        ) -> None:
            target.extend((role, scope, prefix, matcher, value) for value in values)

        add_selectors(
            selectors,
            "universal_build_filenames",
            "any",
            "",
            "basename",
            universal["build_filenames"],
        )
        add_selectors(
            selectors,
            "universal_root_exact_basenames",
            "root",
            "",
            "basename",
            universal["root_exact_basenames"],
        )
        add_selectors(
            selectors,
            "recursive_suffixes",
            "any",
            "",
            "suffix",
            entry["recursive_suffixes"],
        )
        add_selectors(
            selectors,
            "recursive_exact_basenames",
            "any",
            "",
            "basename",
            entry["recursive_exact_basenames"],
        )
        add_selectors(
            selectors,
            "root_exact_basenames",
            "root",
            "",
            "basename",
            entry["root_exact_basenames"],
        )
        add_selectors(
            selectors,
            "root_variable_suffixes",
            "root",
            "",
            "suffix",
            entry["root_variable_suffixes"],
        )
        for path in entry["root_exact_relative_paths"]:
            selectors.append(
                (
                    "root_exact_relative_paths",
                    "exact",
                    path,
                    "basename",
                    posixpath.basename(path),
                )
            )
        for scoped in scoped_inputs:
            scoped_role = f"scoped_inputs:{scoped['id']}"
            scope = scoped["scope"]
            prefix = scoped.get("path_prefix", "")
            add_selectors(
                selectors,
                scoped_role,
                scope,
                prefix,
                "suffix",
                scoped["suffixes"],
            )
            add_selectors(
                selectors,
                scoped_role,
                scope,
                prefix,
                "basename",
                scoped["exact_basenames"],
            )

        def path_in_scope(path: str, scope: str, prefix: str) -> bool:
            folded_path = path.casefold()
            folded_prefix = prefix.casefold()
            if scope == "any":
                return True
            if scope == "root":
                return "/" not in path
            if scope == "subtree":
                return folded_path.startswith(f"{folded_prefix}/")
            return folded_path == folded_prefix

        def scopes_overlap(
            left_scope: str,
            left_prefix: str,
            right_scope: str,
            right_prefix: str,
        ) -> bool:
            if left_scope == "exact":
                return path_in_scope(left_prefix, right_scope, right_prefix)
            if right_scope == "exact":
                return path_in_scope(right_prefix, left_scope, left_prefix)
            if left_scope == "any" or right_scope == "any":
                return True
            if left_scope == right_scope == "root":
                return True
            if "root" in {left_scope, right_scope}:
                return False
            left = left_prefix.casefold()
            right = right_prefix.casefold()
            return (
                left == right
                or left.startswith(f"{right}/")
                or right.startswith(f"{left}/")
            )

        def matchers_overlap(
            left_matcher: str,
            left_value: str,
            right_matcher: str,
            right_value: str,
        ) -> bool:
            left = left_value
            right = right_value
            if left_matcher == right_matcher == "basename":
                return left == right
            if left_matcher == right_matcher == "suffix":
                return left.endswith(right) or right.endswith(left)
            if left_matcher == "basename":
                return left.endswith(right)
            return right.endswith(left)

        for index, left in enumerate(selectors):
            left_role, left_scope, left_prefix, left_matcher, left_value = left
            for right in selectors[index + 1 :]:
                (
                    right_role,
                    right_scope,
                    right_prefix,
                    right_matcher,
                    right_value,
                ) = right
                if left_role == right_role:
                    continue
                if not scopes_overlap(
                    left_scope,
                    left_prefix,
                    right_scope,
                    right_prefix,
                ):
                    continue
                if matchers_overlap(
                    left_matcher,
                    left_value,
                    right_matcher,
                    right_value,
                ):
                    raise ConformanceError(
                        "SOURCE_INPUT_SELECTOR_COLLISION",
                        f"{language} selectors {left_value!r} and {right_value!r} "
                        f"overlap across {left_role} and {right_role}",
                    )

    if selector_count > MAX_SOURCE_INPUT_SELECTORS:
        raise ConformanceError(
            "SOURCE_INPUT_SELECTOR_LIMIT",
            "source-input registry exceeds the bounded selector budget",
        )
    return {"language_count": len(languages), "selector_count": selector_count}


def _validate_repository_source_input_boundary(
    boundary: dict[str, Any],
    schema: dict[str, Any],
    source_input_registry: dict[str, Any],
) -> dict[str, int]:
    """Validate exact repository inputs without granting ambient path authority.

    Package source collection is intentionally rooted at a single package.
    This second registry lists the small set of tracked inputs that cannot be
    expressed there: language-workspace files, exact files below a pruned
    generated component, and exact cross-package inputs. Every selector is a
    complete repository-relative path; there is no suffix, glob, or directory-
    wide fallback.
    """

    _validate_schema(
        boundary,
        schema,
        "REPOSITORY_SOURCE_BOUNDARY_SCHEMA_INVALID",
    )
    if boundary[
        "language_source_input_registry_sha256"
    ] != source_input_registry_digest(source_input_registry):
        raise ConformanceError(
            "REPOSITORY_SOURCE_REGISTRY_DIGEST_MISMATCH",
            "repository boundary does not pin the validated language source-input registry",
        )

    registered_languages = {
        entry["language"] for entry in source_input_registry["languages"]
    }
    generated_components = set(
        source_input_registry["universal_inputs"]["generated_directory_components"]
    )
    boundaries = boundary["boundaries"]
    ids = [entry["id"] for entry in boundaries]
    if ids != _utf8_sorted(ids) or len(ids) != len(set(ids)):
        raise ConformanceError(
            "REPOSITORY_SOURCE_NOT_CANONICAL",
            "repository source boundaries are not in unique UTF-8 id order",
        )

    global_input_identities: dict[str, str] = {}
    global_root_identities: dict[str, str] = {}
    global_input_paths: list[str] = []
    registrations: dict[str, list[tuple[str, dict[str, Any]]]] = {}
    input_count = 0
    scope_count = 0
    authorization_count = 0

    def validate_consumer_root(root: str, *, descendant: bool) -> None:
        if portable_path_error(root) or _source_input_text_error(root):
            raise ConformanceError(
                "REPOSITORY_SOURCE_PATH_UNSAFE",
                f"repository source applicability root is not portable NFC: {root!r}",
            )
        components = root.split("/")
        if (
            len(components) < 3
            or components[0] != "code"
            or components[1] not in {"packages", "programs"}
            or components[2] not in registered_languages
        ):
            raise ConformanceError(
                "REPOSITORY_SOURCE_SCOPE_INVALID",
                f"repository source applicability root is outside a canonical consumer lane: {root}",
            )
        if descendant and len(components) != 3:
            raise ConformanceError(
                "REPOSITORY_SOURCE_SCOPE_INVALID",
                f"descendant boundary must name an exact consumer lane root: {root}",
            )
        if not descendant and len(components) == 3:
            raise ConformanceError(
                "REPOSITORY_SOURCE_SCOPE_INVALID",
                f"exact boundary must name a package or program below its lane root: {root}",
            )

    def applicability_matches(applies_to: dict[str, Any], root: str) -> bool:
        if root in applies_to["exact_roots"]:
            return True
        return any(
            root.startswith(f"{ancestor}/") and root not in applies_to["excluded_roots"]
            for ancestor in applies_to["descendant_roots"]
        )

    def applicability_overlaps(left: dict[str, Any], right: dict[str, Any]) -> bool:
        if set(left["exact_roots"]) & set(right["exact_roots"]):
            return True
        if any(applicability_matches(right, root) for root in left["exact_roots"]):
            return True
        if any(applicability_matches(left, root) for root in right["exact_roots"]):
            return True
        return bool(set(left["descendant_roots"]) & set(right["descendant_roots"]))

    for entry in boundaries:
        input_origin = entry["input_origin"]
        if input_origin != "repository" and input_origin not in registered_languages:
            raise ConformanceError(
                "REPOSITORY_SOURCE_LANGUAGE_UNKNOWN",
                f"repository source boundary input origin is not registered: {input_origin}",
            )
        if (
            not entry["owner"]
            or not entry["reason"]
            or _source_input_text_error(entry["owner"])
            or _source_input_text_error(entry["reason"])
        ):
            raise ConformanceError(
                "REPOSITORY_SOURCE_CLASSIFICATION_INVALID",
                "repository source boundaries require a portable durable owner and reason",
            )

        applies_to = entry["applies_to"]
        exact_roots = applies_to["exact_roots"]
        descendant_roots = applies_to["descendant_roots"]
        excluded_roots = applies_to["excluded_roots"]
        for field, roots in (
            ("exact_roots", exact_roots),
            ("descendant_roots", descendant_roots),
            ("excluded_roots", excluded_roots),
        ):
            if roots != _utf8_sorted(roots) or len(
                {root.casefold() for root in roots}
            ) != len(roots):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_NOT_CANONICAL",
                    f"repository source {field} are not in unique UTF-8 order: {entry['id']}",
                )
            for root in roots:
                identity = root.casefold()
                prior = global_root_identities.get(identity)
                if prior is not None and prior != root:
                    raise ConformanceError(
                        "REPOSITORY_SOURCE_SCOPE_COLLISION",
                        f"repository source applicability root collides by platform identity: {root}",
                    )
                global_root_identities[identity] = root
        for root in exact_roots:
            validate_consumer_root(root, descendant=False)
        for root in descendant_roots:
            validate_consumer_root(root, descendant=True)
        for root in excluded_roots:
            validate_consumer_root(root, descendant=False)
            if not any(
                root.startswith(f"{ancestor}/") for ancestor in descendant_roots
            ):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_SCOPE_INVALID",
                    f"excluded root is not below a declared descendant root: {root}",
                )
        if set(exact_roots) & set(excluded_roots):
            raise ConformanceError(
                "REPOSITORY_SOURCE_SCOPE_COLLISION",
                "repository source exact and excluded roots cannot overlap",
            )
        for root in exact_roots:
            if any(
                root.startswith(f"{ancestor}/") and root not in excluded_roots
                for ancestor in descendant_roots
            ):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_SCOPE_COLLISION",
                    f"exact root is redundant with a descendant root: {root}",
                )
        scope_count += len(exact_roots) + len(descendant_roots)

        inputs = entry["inputs"]
        paths = [item["path"] for item in inputs]
        local_identities: dict[str, str] = {}
        for item in inputs:
            path = item["path"]
            if portable_path_error(path) or _source_input_text_error(path):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_PATH_UNSAFE",
                    f"repository source input is not portable NFC: {path!r}",
                )
            if _repository_source_sensitive_path(path):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_SENSITIVE_PATH",
                    "repository source boundaries cannot authorize credentials, secrets, signing material, or machine-local configuration",
                )
            identity = path.casefold()
            prior = global_input_identities.get(identity)
            if identity in local_identities or (prior is not None and prior != path):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_INPUT_COLLISION",
                    f"repository source input collides by platform identity: {path}",
                )
            local_identities[identity] = path
            global_input_identities[identity] = path

            path_components = path.split("/")
            if len(path_components) < 2 or path_components[0] != "code":
                raise ConformanceError(
                    "REPOSITORY_SOURCE_ROLE_INVALID",
                    f"repository source input is outside the canonical code tree: {path}",
                )

            implementation_lane_path = (
                len(path_components) >= 4
                and path_components[1] in {"packages", "programs"}
                and path_components[2] in registered_languages
            )
            if (
                input_origin == "repository"
                and path_components[1] in {"packages", "programs"}
            ) or (
                input_origin != "repository"
                and (not implementation_lane_path or path_components[2] != input_origin)
            ):
                raise ConformanceError(
                    "REPOSITORY_SOURCE_ROLE_INVALID",
                    f"repository source input does not belong to its declared origin: {path}",
                )

            generated_in_path = [
                component
                for component in path_components
                if component in generated_components
            ]
            if item["role"] == "shared_ancestor":
                input_workspace_root = f"code/packages/{input_origin}"
                if (
                    input_origin == "repository"
                    or len(path_components) < 4
                    or path_components[2] != input_origin
                    or path.rpartition("/")[0] != input_workspace_root
                    or generated_in_path
                    or any(root != input_workspace_root for root in descendant_roots)
                    or any(
                        not root.startswith(f"{input_workspace_root}/")
                        for root in exact_roots
                    )
                ):
                    raise ConformanceError(
                        "REPOSITORY_SOURCE_ROLE_INVALID",
                        f"shared input is not an ancestor of every consumer in its input workspace: {path}",
                    )
            elif item["role"] == "generated_pruning_exception":
                declared_component = item["generated_component"]
                if (
                    input_origin == "repository"
                    or len(path_components) < 4
                    or path_components[2] != input_origin
                    or declared_component not in generated_components
                    or generated_in_path != [declared_component]
                    or any(not path.startswith(f"{root}/") for root in exact_roots)
                    or any(not path.startswith(f"{root}/") for root in descendant_roots)
                ):
                    raise ConformanceError(
                        "REPOSITORY_SOURCE_ROLE_INVALID",
                        f"pruning exception is not contained by every scope with one generated component: {path}",
                    )
            else:
                if (
                    descendant_roots
                    or not exact_roots
                    or any(
                        path == root or path.startswith(f"{root}/")
                        for root in exact_roots
                    )
                ):
                    raise ConformanceError(
                        "REPOSITORY_SOURCE_ROLE_INVALID",
                        f"cross-package input is not outside every exact consumer root: {path}",
                    )
            input_count += 1
            registrations.setdefault(identity, []).append((entry["id"], applies_to))

        if paths != _utf8_sorted(paths):
            raise ConformanceError(
                "REPOSITORY_SOURCE_NOT_CANONICAL",
                f"repository source inputs are not in UTF-8 path order: {entry['id']}",
            )

        global_input_paths.extend(paths)
        authorization_count += len(inputs) * (len(exact_roots) + len(descendant_roots))

    if scope_count > MAX_REPOSITORY_SOURCE_SCOPES:
        raise ConformanceError(
            "REPOSITORY_SOURCE_SCOPE_LIMIT",
            "repository source boundaries exceed the bounded scope budget",
        )
    if authorization_count > MAX_REPOSITORY_SOURCE_AUTHORIZATIONS:
        raise ConformanceError(
            "REPOSITORY_SOURCE_AUTHORIZATION_LIMIT",
            "repository source boundaries exceed the bounded authorization budget",
        )

    folded_paths = sorted({path.casefold() for path in global_input_paths})
    for path, other in pairwise(folded_paths):
        if other.startswith(f"{path}/"):
            raise ConformanceError(
                "REPOSITORY_SOURCE_INPUT_COLLISION",
                "repository source input files cannot be path prefixes",
            )

    for path, path_registrations in registrations.items():
        for index, (left_owner, left_scope) in enumerate(path_registrations):
            for right_owner, right_scope in path_registrations[index + 1 :]:
                if applicability_overlaps(left_scope, right_scope):
                    raise ConformanceError(
                        "REPOSITORY_SOURCE_SCOPE_COLLISION",
                        f"repository source input is authorized twice for overlapping scopes: {path} ({left_owner}, {right_owner})",
                    )

    return {
        "boundary_count": len(boundaries),
        "input_count": input_count,
        "scope_count": scope_count,
        "authorization_count": authorization_count,
    }


@lru_cache(maxsize=1)
def _default_source_input_registry() -> dict[str, Any]:
    schema = load_document(
        DEFAULT_FIXTURE_ROOT / "language-source-input-registry.schema.json"
    )
    registry = load_document(
        DEFAULT_FIXTURE_ROOT / "language-source-input-registry.json"
    )
    _validate_source_input_registry(registry, schema)
    return registry


@lru_cache(maxsize=1)
def _default_repository_source_input_boundary() -> dict[str, Any]:
    schema = load_document(
        DEFAULT_FIXTURE_ROOT / "repository-source-input-boundary.schema.json"
    )
    boundary = load_document(
        DEFAULT_FIXTURE_ROOT / "repository-source-input-boundary.json"
    )
    _validate_repository_source_input_boundary(
        boundary,
        schema,
        _default_source_input_registry(),
    )
    return boundary


def _cli_argument_is_unsafe(argument: str) -> bool:
    if argument.startswith("@"):
        return True
    if not argument.startswith("--") and re.match(
        r"^[A-Za-z_][A-Za-z0-9_]*=", argument
    ):
        return True
    if any(ord(character) < 32 for character in argument):
        return True
    return any(
        marker in argument
        for marker in (
            ";",
            "&",
            "|",
            "<",
            ">",
            "`",
            "$",
            "%",
            "!",
            "(",
            ")",
            "^",
        )
    )


def _cli_git_ref_is_valid(value: str) -> bool:
    base, separator, ancestry = value.partition("~")
    segments = base.split("/")
    return bool(
        len(value) <= 128
        and re.fullmatch(r"[A-Za-z0-9][A-Za-z0-9._/-]*", base)
        and (not separator or bool(re.fullmatch(r"0|[1-9][0-9]*", ancestry)))
        and "~" not in ancestry
        and "//" not in base
        and ".." not in base
        and not base.endswith(("/", ".lock"))
        and not any(
            segment in {"", ".", ".."}
            or segment.startswith(".")
            or segment.endswith((".", ".lock"))
            for segment in segments
        )
    )


def _parse_cli_argv(argv: list[Any]) -> tuple[dict[str, Any] | None, str | None]:
    """Parse the inert portable CLI grammar without consulting host state."""

    if len(argv) > CLI_MAX_ARGUMENTS:
        return None, "CLI_ARGUMENT_LIMIT"
    if any(
        not isinstance(argument, str)
        or not argument
        or len(argument) > CLI_MAX_ARGUMENT_CHARACTERS
        for argument in argv
    ):
        return None, "CLI_ARGUMENT_LIMIT"
    try:
        encoded_bytes = sum(len(argument.encode("utf-8")) for argument in argv)
    except UnicodeEncodeError:
        return None, "CLI_ARGUMENT_UNSAFE"
    if encoded_bytes > CLI_MAX_ARGUMENT_BYTES:
        return None, "CLI_ARGUMENT_LIMIT"

    for argument in argv:
        if any(
            argument == flag or argument.startswith(f"{flag}=")
            for flag in RESERVED_ADAPTER_FLAGS
        ):
            return None, "CLI_ARGUMENT_RESERVED"
    if any(_cli_argument_is_unsafe(argument) for argument in argv):
        return None, "CLI_ARGUMENT_UNSAFE"

    parsed: dict[str, Any] = {
        "cache_file": ".build-cache.json",
        "clippy": False,
        "detect_languages": False,
        "diff_base": "origin/main",
        "dry_run": False,
        "emit_plan": None,
        "emit_shard_matrix": False,
        "force": False,
        "jobs": None,
        "language": "all",
        "plan_file": None,
        "root": None,
        "shard_count": None,
        "shard_index": None,
        "validate_build_files": True,
    }
    boolean_flags = {
        "--clippy": ("clippy", True),
        "--detect-languages": ("detect_languages", True),
        "--dry-run": ("dry_run", True),
        "--emit-shard-matrix": ("emit_shard_matrix", True),
        "--force": ("force", True),
        "--no-validate-build-files": ("validate_build_files", False),
        "--validate-build-files": ("validate_build_files", True),
    }
    value_flags = {
        "--cache-file": "cache_file",
        "--diff-base": "diff_base",
        "--emit-plan": "emit_plan",
        "--jobs": "jobs",
        "--language": "language",
        "--plan-file": "plan_file",
        "--root": "root",
        "--shard-count": "shard_count",
        "--shard-index": "shard_index",
    }
    seen: set[str] = set()
    index = 0
    while index < len(argv):
        argument = argv[index]
        flag, separator, attached = argument.partition("=")
        if flag in boolean_flags:
            field, value = boolean_flags[flag]
            if separator or field in seen:
                return None, "CLI_USAGE_INVALID"
            seen.add(field)
            parsed[field] = value
            index += 1
            continue
        option_field = value_flags.get(flag)
        if option_field is None or option_field in seen:
            return None, "CLI_USAGE_INVALID"
        if separator:
            value = attached
        else:
            index += 1
            if index >= len(argv) or argv[index].startswith("--"):
                return None, "CLI_USAGE_INVALID"
            value = argv[index]
        if not value:
            return None, "CLI_USAGE_INVALID"
        seen.add(option_field)

        if option_field in {"jobs", "shard_count", "shard_index"}:
            if not re.fullmatch(r"0|[1-9][0-9]*", value):
                return None, "CLI_USAGE_INVALID"
            integer = int(value)
            minimum = 0 if option_field == "shard_index" else 1
            maximum = 255 if option_field == "shard_index" else 256
            if not minimum <= integer <= maximum:
                return None, "CLI_USAGE_INVALID"
            parsed[option_field] = integer
        elif option_field == "language":
            if value not in CLI_LANGUAGES:
                return None, "CLI_USAGE_INVALID"
            parsed[option_field] = value
        elif option_field == "diff_base":
            if not _cli_git_ref_is_valid(value):
                return None, "CLI_USAGE_INVALID"
            parsed[option_field] = value
        else:
            if option_field == "root" and value == ".":
                parsed[option_field] = value
            elif portable_path_error(value) is not None:
                return None, "CLI_PATH_UNSAFE"
            else:
                parsed[option_field] = value
        index += 1

    if parsed["emit_plan"] is not None and parsed["plan_file"] is not None:
        return None, "CLI_USAGE_INVALID"
    if parsed["shard_count"] is not None and parsed["emit_plan"] is None:
        return None, "CLI_USAGE_INVALID"
    if parsed["shard_index"] is not None and parsed["plan_file"] is None:
        return None, "CLI_USAGE_INVALID"
    if parsed["emit_shard_matrix"] and (
        parsed["emit_plan"] is None or parsed["shard_count"] is None
    ):
        return None, "CLI_USAGE_INVALID"
    return parsed, None


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
        mode = case["input"]["options"]["mode"]
        if mode == "replace_existing" and "plan_v1_write" not in capabilities:
            raise ConformanceError(
                "CASE_CAPABILITY_MISSING",
                "replace_existing plan case requires plan_v1_write",
            )
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


def _validate_plan_semantics(
    plan: dict[str, Any],
    *,
    prefix: str = "RESULT",
) -> None:
    packages = plan.get("packages", [])
    package_names: set[str] = set()
    for package in packages:
        name = package["name"]
        if name in package_names:
            raise ConformanceError(
                f"{prefix}_PLAN_PACKAGE_DUPLICATE",
                f"duplicate plan package: {name}",
            )
        package_names.add(name)
        if error := portable_path_error(package["rel_path"]):
            raise ConformanceError(
                f"{prefix}_PLAN_PATH_UNSAFE",
                f"unsafe plan rel_path for {name}: {error}",
            )
    for edge in plan.get("dependency_edges", []):
        if edge[0] not in package_names or edge[1] not in package_names:
            raise ConformanceError(
                f"{prefix}_PLAN_EDGE_UNKNOWN",
                f"plan edge references an unknown package: {edge}",
            )
    affected = plan.get("affected_packages")
    if isinstance(affected, list):
        for name in affected:
            if name not in package_names:
                raise ConformanceError(
                    f"{prefix}_PLAN_AFFECTED_UNKNOWN",
                    f"affected package is not declared: {name}",
                )


def _validate_case_plan_inputs(
    case: dict[str, Any],
    plan_schema: dict[str, Any],
) -> None:
    """Validate both complete plans required by repeated replacement."""
    if case["domain"] != "plan":
        return
    options = case["input"]["options"]
    if options["mode"] != "replace_existing":
        return
    for key, prefix in (("existing_plan", "CASE_EXISTING"), ("plan", "CASE")):
        plan = options[key]
        _validate_schema(plan, plan_schema, f"{prefix}_PLAN_SCHEMA_INVALID")
        _validate_plan_semantics(plan, prefix=prefix)


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


def _path_identity(path: str) -> str:
    return unicodedata.normalize("NFC", path).casefold()


def _is_orphan_artifact_path(path: str) -> bool:
    return any(component in ORPHAN_SKIP_COMPONENTS for component in path.split("/"))


def _is_under_orphan_scan_root(path: str) -> bool:
    return path == ORPHAN_SCAN_ROOT or path.startswith(f"{ORPHAN_SCAN_ROOT}/")


def _validate_orphan_snapshot(snapshot: dict[str, Any]) -> None:
    directories = snapshot["directories"]
    manifests = snapshot["manifests"]
    build_files = snapshot["build_files"]
    exemptions = snapshot["exemptions"]

    if directories != sorted(directories):
        raise ConformanceError(
            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
            "orphan directories must be sorted",
        )
    normalized_directories: set[str] = set()
    for path in directories:
        if error := portable_path_error(path):
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                f"unsafe orphan directory {path!r}: {error}",
            )
        if not _is_under_orphan_scan_root(path):
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                f"orphan directory is outside {ORPHAN_SCAN_ROOT}/: {path}",
            )
        identity = _path_identity(path)
        if identity in normalized_directories:
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                f"duplicate normalized orphan directory: {path}",
            )
        normalized_directories.add(identity)

    for records, label in ((manifests, "manifest"), (build_files, "BUILD")):
        paths = [record["path"] for record in records]
        if paths != sorted(paths):
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                f"orphan {label} paths must be sorted",
            )
        normalized: set[str] = set()
        for path in paths:
            if error := portable_path_error(path):
                raise ConformanceError(
                    "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                    f"unsafe orphan {label} path {path!r}: {error}",
                )
            if not _is_under_orphan_scan_root(path):
                raise ConformanceError(
                    "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                    f"orphan {label} path is outside {ORPHAN_SCAN_ROOT}/: {path}",
                )
            identity = _path_identity(path)
            if identity in normalized:
                raise ConformanceError(
                    "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                    f"duplicate normalized orphan {label} path: {path}",
                )
            normalized.add(identity)

    for manifest in manifests:
        if manifest["path"] not in set(directories):
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                f"orphan manifest directory is not declared: {manifest['path']}",
            )
    for build_file in build_files:
        if posixpath.basename(build_file["path"]) not in ORPHAN_BUILD_NAMES:
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                f"unrecognized orphan BUILD filename: {build_file['path']}",
            )

    lines = [entry["line"] for entry in exemptions]
    if lines != sorted(lines) or len(lines) != len(set(lines)):
        raise ConformanceError(
            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
            "orphan exemption lines must be strictly increasing and unique",
        )


def _normalize_tracked_artifact_path(path: str) -> tuple[str | None, str | None]:
    normalized = path.replace("\\", "/")
    if not normalized:
        return None, "EMPTY"
    if len(normalized) > 512:
        return None, "TOO_LONG"
    if normalized != tracked_unicode.nfc(normalized):
        return None, "NON_NFC"
    if normalized.startswith("/"):
        return None, "ABSOLUTE"
    if re.match(r"^[A-Za-z]:", normalized):
        return None, "DRIVE_QUALIFIED"
    if any(not segment for segment in normalized.split("/")):
        return None, "EMPTY_SEGMENT"
    if any(ord(character) < 32 or character in '<>:"|?*' for character in normalized):
        return None, "UNSAFE_CHARACTER"
    for segment in normalized.split("/"):
        if segment in {".", ".."}:
            return None, "DOT_SEGMENT"
        if segment.endswith((" ", ".")):
            return None, "TRAILING_DOT_OR_SPACE"
        basename = tracked_unicode.full_uppercase(segment.split(".", 1)[0])
        if basename in WINDOWS_RESERVED_BASENAMES:
            return None, "RESERVED_BASENAME"
    return normalized, None


def _validate_tracked_artifact_snapshot(snapshot: dict[str, Any]) -> None:
    if snapshot["unicode_version"] != tracked_unicode.UNICODE_VERSION:
        raise ConformanceError(
            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
            "tracked artifact Unicode version does not match the pinned runtime",
        )
    ordinals = [entry["ordinal"] for entry in snapshot["entries"]]
    if ordinals != sorted(ordinals) or len(ordinals) != len(set(ordinals)):
        raise ConformanceError(
            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
            "tracked artifact ordinals must be strictly increasing and unique",
        )


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
    source_input_registry: dict[str, Any] | None = None,
    repository_source_input_boundary: dict[str, Any] | None = None,
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
    elif domain == "source_collection":
        registry = source_input_registry or _default_source_input_registry()
        if options["language"] not in {
            entry["language"] for entry in registry["languages"]
        }:
            raise ConformanceError(
                "CASE_SOURCE_LANGUAGE_UNKNOWN",
                f"source-collection language is not registered: {options['language']}",
            )
        repository_mode = options["mode"] == "repository_boundary"
        if repository_mode:
            boundary = (
                repository_source_input_boundary
                or _default_repository_source_input_boundary()
            )
            if options["boundary_sha256"] != repository_source_input_boundary_digest(
                boundary
            ):
                raise ConformanceError(
                    "CASE_REPOSITORY_SOURCE_BOUNDARY_DIGEST_MISMATCH",
                    "repository source-collection case does not pin the validated boundary",
                )
            package_root = options["package_root"]
            if portable_path_error(package_root) or _source_input_text_error(
                package_root
            ):
                raise ConformanceError(
                    "CASE_REPOSITORY_SOURCE_ROOT_UNSAFE",
                    f"repository source package root is not portable NFC: {package_root!r}",
                )
            root_components = package_root.split("/")
            if (
                len(root_components) < 4
                or root_components[0] != "code"
                or root_components[1] not in {"packages", "programs"}
                or root_components[2] != options["language"]
            ):
                raise ConformanceError(
                    "CASE_REPOSITORY_SOURCE_ROOT_LANGUAGE_MISMATCH",
                    "repository source package root does not belong to the declared consumer language",
                )
        elif options["registry_sha256"] != source_input_registry_digest(registry):
            raise ConformanceError(
                "CASE_SOURCE_REGISTRY_DIGEST_MISMATCH",
                "source-collection case does not pin the validated registry snapshot",
            )
        candidate_paths: dict[str, tuple[str, str]] = {}
        for candidate in options["candidates"]:
            path = candidate["path"]
            if portable_path_error(path) or _source_input_text_error(path):
                raise ConformanceError(
                    "CASE_SOURCE_PATH_UNSAFE",
                    f"source candidate path is not portable NFC: {path!r}",
                )
            identity = path.casefold()
            if identity in candidate_paths:
                raise ConformanceError(
                    "CASE_SOURCE_CANDIDATE_DUPLICATE",
                    f"duplicate platform-identity source candidate path: {path}",
                )
            for other_path, other_kind in candidate_paths.values():
                folded_other = other_path.casefold()
                path_below_other = identity.startswith(f"{folded_other}/")
                other_below_path = folded_other.startswith(f"{identity}/")
                if (path_below_other and other_kind == "file") or (
                    other_below_path and candidate["kind"] == "file"
                ):
                    raise ConformanceError(
                        "CASE_SOURCE_CANDIDATE_COLLISION",
                        f"source candidate file paths cannot be prefixes: {other_path!r}, {path!r}",
                    )
            candidate_paths[identity] = (path, candidate["kind"])
        if not repository_mode:
            for pattern in options["declared_srcs"]:
                if error := portable_glob_error(pattern):
                    raise ConformanceError(
                        "CASE_SOURCE_GLOB_UNSAFE",
                        f"unsafe declared source glob {pattern!r}: {error}",
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
        checks = set(options["checks"])
        lua_windows_sibling_parity = "lua_windows_sibling_parity" in checks
        starlark_declarations = "starlark_declarations" in checks
        if "orphan_crate_coverage" in checks:
            _validate_orphan_snapshot(options["orphan_snapshot"])
        if "tracked_artifact_absence" in checks:
            _validate_tracked_artifact_snapshot(options["tracked_artifact_snapshot"])
        _validate_unique_paths(
            [package["rel_path"] for package in by_name.values()],
            "CASE_NESTED_PATH_UNSAFE",
        )
        _validate_known_edges(options["dependency_edges"], set(by_name))
        for package in by_name.values():
            for pattern in package["declared_srcs"]:
                if not starlark_declarations and (
                    error := portable_glob_error(pattern)
                ):
                    raise ConformanceError(
                        "CASE_NESTED_GLOB_UNSAFE",
                        f"unsafe declared source {pattern!r}: {error}",
                    )
            for field in (
                "identity_candidates",
                "manifest_candidates",
                "validation_paths",
            ):
                if field in package and package[field] != sorted(package[field]):
                    raise ConformanceError(
                        "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                        f"{field} must be sorted",
                    )
            reference_fields = ["build_references"]
            if not starlark_declarations:
                reference_fields.append("declared_deps")
            if lua_windows_sibling_parity:
                reference_fields.extend(
                    (
                        "canonical_lua_sibling_installs",
                        "windows_lua_sibling_installs",
                    )
                )
                if package["language"] != "lua":
                    raise ConformanceError(
                        "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                        "lua_windows_sibling_parity accepts only Lua packages",
                    )
                for field in (
                    "canonical_lua_sibling_installs",
                    "windows_lua_sibling_installs",
                ):
                    if package[field] != sorted(package[field]):
                        raise ConformanceError(
                            "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                            f"{field} must be sorted",
                        )
                if (
                    package["build_file_state"] != "present"
                    and package["canonical_lua_sibling_installs"]
                ):
                    raise ConformanceError(
                        "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                        "a non-present canonical BUILD cannot declare sibling installs",
                    )
                if (
                    package["windows_build_file_state"] != "present"
                    and package["windows_lua_sibling_installs"]
                ):
                    raise ConformanceError(
                        "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                        "a non-present BUILD_windows cannot declare sibling installs",
                    )
            for field in reference_fields:
                for name in package[field]:
                    if name not in by_name:
                        raise ConformanceError(
                            "CASE_PACKAGE_REFERENCE_UNKNOWN",
                            f"{field} references an unknown package: {name}",
                        )
    elif domain == "toolchain_detection":
        by_name = _package_index(options["packages"])
        selected = options["scheduled_packages"]
        if options["force_full"] and selected is not None:
            raise ConformanceError(
                "CASE_TOOLCHAIN_FORCE_SELECTION_INVALID",
                "force_full requires a null scheduled_packages selection",
            )
        snapshot_bytes = 0
        for package in options["packages"]:
            for content in package["build_files"].values():
                content_bytes = len(content.encode("utf-8"))
                snapshot_bytes += content_bytes
                if (
                    content_bytes > MAX_TOOLCHAIN_BUILD_BYTES
                    or content.count("\n") + 1 > MAX_TOOLCHAIN_BUILD_LINES
                ):
                    raise ConformanceError(
                        "CASE_TOOLCHAIN_SNAPSHOT_LIMIT_EXCEEDED",
                        "a BUILD snapshot exceeds the per-file byte or line ceiling",
                    )
        if snapshot_bytes > MAX_TOOLCHAIN_SNAPSHOT_BYTES:
            raise ConformanceError(
                "CASE_TOOLCHAIN_SNAPSHOT_LIMIT_EXCEEDED",
                "the aggregate BUILD snapshot exceeds the byte ceiling",
            )
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


def _source_input_entry(registry: dict[str, Any], language: str) -> dict[str, Any]:
    for entry in registry["languages"]:
        if entry["language"] == language:
            return entry
    raise ConformanceError(
        "CASE_SOURCE_LANGUAGE_UNKNOWN",
        f"source-collection language is not registered: {language}",
    )


def _scoped_source_input_matches(rule: dict[str, Any], path: str) -> bool:
    basename = posixpath.basename(path)
    if rule["scope"] == "root":
        if "/" in path:
            return False
    else:
        prefix = rule["path_prefix"]
        if not path.startswith(f"{prefix}/"):
            return False
    return basename in set(rule["exact_basenames"]) or any(
        basename.endswith(suffix) for suffix in rule["suffixes"]
    )


def _expected_source_collection(
    options: dict[str, Any],
    source_input_registry: dict[str, Any] | None = None,
) -> list[dict[str, str]]:
    registry = source_input_registry or _default_source_input_registry()
    if options["registry_sha256"] != source_input_registry_digest(registry):
        raise ConformanceError(
            "CASE_SOURCE_REGISTRY_DIGEST_MISMATCH",
            "source-collection case does not pin the validated registry snapshot",
        )
    language_inputs = _source_input_entry(registry, options["language"])
    universal = registry["universal_inputs"]
    link_roots = {
        candidate["path"]
        for candidate in options["candidates"]
        if candidate["kind"] != "file"
    }
    declared_srcs = options["declared_srcs"]
    files: list[dict[str, str]] = []

    for candidate in options["candidates"]:
        if candidate["kind"] != "file":
            continue
        path = candidate["path"]
        if any(path == root or path.startswith(f"{root}/") for root in link_roots):
            continue
        if any(
            component in set(universal["generated_directory_components"])
            for component in path.split("/")
        ):
            continue

        basename = posixpath.basename(path)
        is_root = "/" not in path
        included = basename in set(universal["build_filenames"])
        if is_root and basename in set(universal["root_exact_basenames"]):
            included = True
        if is_root and basename in set(language_inputs["root_exact_basenames"]):
            included = True
        if is_root and any(
            basename.endswith(suffix)
            for suffix in language_inputs["root_variable_suffixes"]
        ):
            included = True
        if path in set(language_inputs["root_exact_relative_paths"]):
            included = True

        if options["mode"] == "extension":
            if basename in set(language_inputs["recursive_exact_basenames"]):
                included = True
            if any(
                basename.endswith(suffix)
                for suffix in language_inputs["recursive_suffixes"]
            ):
                included = True
            scoped_matches = [
                rule
                for rule in language_inputs["scoped_inputs"]
                if _scoped_source_input_matches(rule, path)
            ]
            if scoped_matches:
                included = True
        elif not included:
            included = any(
                _portable_glob_matches(pattern, path) for pattern in declared_srcs
            )
        if included:
            files.append(
                {
                    "path": path,
                    "digest": hashlib.sha256(
                        bytes.fromhex(candidate["content_hex"])
                    ).hexdigest(),
                }
            )

    return sorted(files, key=lambda item: item["path"])


def _repository_boundary_applies(
    boundary: dict[str, Any],
    package_root: str,
) -> bool:
    applies_to = boundary["applies_to"]
    if package_root in applies_to["exact_roots"]:
        return True
    return any(
        package_root.startswith(f"{root}/")
        and package_root not in applies_to["excluded_roots"]
        for root in applies_to["descendant_roots"]
    )


def _expected_repository_source_collection(
    options: dict[str, Any],
    repository_source_input_boundary: dict[str, Any] | None = None,
) -> list[dict[str, str]]:
    """Select exact tracked repository inputs for one package snapshot."""

    boundary = (
        repository_source_input_boundary or _default_repository_source_input_boundary()
    )
    if options["boundary_sha256"] != repository_source_input_boundary_digest(boundary):
        raise ConformanceError(
            "CASE_REPOSITORY_SOURCE_BOUNDARY_DIGEST_MISMATCH",
            "repository source-collection case does not pin the validated boundary",
        )

    allowed_paths = {
        item["path"]
        for entry in boundary["boundaries"]
        if _repository_boundary_applies(
            entry,
            options["package_root"],
        )
        for item in entry["inputs"]
    }
    files = [
        {
            "path": candidate["path"],
            "digest": hashlib.sha256(
                bytes.fromhex(candidate["content_hex"])
            ).hexdigest(),
        }
        for candidate in options["candidates"]
        if candidate["kind"] == "file"
        and candidate["tracked"]
        and candidate["path"] in allowed_paths
    ]
    return sorted(files, key=lambda item: item["path"])


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
    pending = [(options["entrypoint"], 0, False)]
    visited: set[str] = set()
    active: set[str] = set()
    limits = options["evaluation_limits"]
    while pending:
        module, depth, exiting = pending.pop()
        if exiting:
            active.remove(module)
            continue
        if module in active:
            return "STARLARK_LOAD_CYCLE", module
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
        active.add(module)
        pending.append((module, depth, True))
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
            pending.append((resolved, depth + 1, False))
    return None


def _selected_toolchain_build_content(package: dict[str, Any], platform: str) -> str:
    build_files = package["build_files"]
    candidates = {
        "darwin": ("BUILD_mac", "BUILD_mac_and_linux", "BUILD"),
        "linux": ("BUILD_linux", "BUILD_mac_and_linux", "BUILD"),
        "windows": ("BUILD_windows", "BUILD"),
    }[platform]
    for filename in candidates:
        if filename in build_files:
            return build_files[filename]
    raise AssertionError("toolchain package schema requires generic BUILD")


def _extra_toolchain_declarations(raw_content: str) -> list[str]:
    prefix = "# needs-toolchain:"
    declarations: list[str] = []
    seen: set[str] = set()
    raw_lines = raw_content.split("\n")
    for index, raw_line in enumerate(raw_lines):
        # Splitting on LF preserves the byte immediately before the terminator.
        # Remove CR only when this segment really was LF-terminated; a final
        # lone CR and a CR before trailing spaces remain grammar content.
        if index < len(raw_lines) - 1 and raw_line.endswith("\r"):
            raw_line = raw_line[:-1]
        line = raw_line.lstrip(" \t").rstrip(" \t")
        if not line.startswith(prefix):
            continue
        suffix = line[len(prefix) :]
        if not suffix or suffix[0] not in " \t":
            continue
        name = suffix.strip(" \t")
        if name not in TOOLCHAINS or name in seen:
            continue
        seen.add(name)
        declarations.append(name)
    return declarations


def _expected_toolchains(
    options: dict[str, Any],
) -> dict[str, bool]:
    if options["force_full"]:
        return {toolchain: True for toolchain in TOOLCHAINS}
    by_name = {package["name"]: package for package in options["packages"]}
    selected = options["scheduled_packages"]
    selected_names = sorted(by_name) if selected is None else selected
    enabled: set[str] = set()
    for name in selected_names:
        package = by_name[name]
        enabled.add(_toolchain_for_language(package["language"]))
        selected_content = _selected_toolchain_build_content(
            package, options["platform"]
        )
        enabled.update(_extra_toolchain_declarations(selected_content))
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


def _expected_orphan_validation(
    options: dict[str, Any],
) -> tuple[list[dict[str, Any]], int]:
    snapshot = options["orphan_snapshot"]
    manifests = [
        manifest
        for manifest in snapshot["manifests"]
        if not _is_orphan_artifact_path(manifest["path"])
    ]
    build_files = snapshot["build_files"]
    directories = set(snapshot["directories"])
    manifest_by_path = {manifest["path"]: manifest for manifest in manifests}
    build_name_rank = {name: index for index, name in enumerate(ORPHAN_BUILD_NAMES)}

    def covering_builds(manifest_path: str, state: str) -> list[dict[str, Any]]:
        candidates = []
        for build_file in build_files:
            if build_file["state"] != state:
                continue
            parent = posixpath.dirname(build_file["path"])
            if manifest_path != parent and not manifest_path.startswith(f"{parent}/"):
                continue
            if not _is_under_orphan_scan_root(parent):
                continue
            candidates.append(build_file)
        return sorted(
            candidates,
            key=lambda item: (
                -len(posixpath.dirname(item["path"]).split("/")),
                build_name_rank[posixpath.basename(item["path"])],
                item["path"],
            ),
        )

    coverage: dict[str, dict[str, Any] | None] = {}
    empty_builds: dict[str, dict[str, Any] | None] = {}
    for manifest in manifests:
        runnable = covering_builds(manifest["path"], "runnable")
        empty = covering_builds(manifest["path"], "empty")
        coverage[manifest["path"]] = runnable[0] if runnable else None
        empty_builds[manifest["path"]] = empty[0] if empty else None

    diagnostics: list[dict[str, Any]] = []
    seen_exemption_paths: dict[str, int] = {}
    valid_exemptions: list[dict[str, Any]] = []
    for exemption in snapshot["exemptions"]:
        path = exemption["path"]
        exemption_identity: str | None = None
        path_problem: str | None = None
        if portable_path_error(path) is not None:
            path_problem = "PATH_UNSAFE"
        else:
            exemption_identity = _path_identity(path)
            if not _is_under_orphan_scan_root(path):
                path_problem = "PATH_OUTSIDE_SCAN"
            elif _is_orphan_artifact_path(path):
                path_problem = "PATH_ARTIFACT"

        duplicate = False
        if exemption_identity is not None:
            duplicate = exemption_identity in seen_exemption_paths
            if not duplicate:
                seen_exemption_paths[exemption_identity] = exemption["line"]

        problem: str | None
        if exemption["kind"] not in {"EXCLUDED", "PENDING"}:
            problem = "UNKNOWN_KIND"
        elif not exemption["reason"].strip():
            problem = "REASON_MISSING"
        elif duplicate:
            problem = "DUPLICATE_PATH"
        elif path_problem is not None:
            problem = path_problem
        else:
            problem = None

        if problem is not None:
            diagnostics.append(
                {
                    "code": "ORPHAN_EXEMPTION_INVALID",
                    "severity": "error",
                    "path": ORPHAN_LEDGER_PATH,
                    "details": {
                        "line": exemption["line"],
                        "problem": problem,
                    },
                }
            )
            continue
        if exemption_identity is None:
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                "valid orphan exemption path has no normalized identity",
            )
        valid_exemptions.append(exemption)

    active_exemptions: dict[str, dict[str, Any]] = {}
    pending_exemption_count = 0
    for exemption in valid_exemptions:
        exemption_path = exemption["path"]
        stale_problem: str | None = None
        if exemption_path not in directories:
            stale_problem = "MISSING_DIRECTORY"
        elif exemption_path not in manifest_by_path:
            stale_problem = "NO_MANIFEST"
        elif coverage[exemption_path] is not None:
            stale_problem = "COVERED"
        if stale_problem is not None:
            diagnostics.append(
                {
                    "code": "ORPHAN_EXEMPTION_STALE",
                    "severity": "error",
                    "path": ORPHAN_LEDGER_PATH,
                    "details": {
                        "entry_path": exemption["path"],
                        "kind": exemption["kind"],
                        "line": exemption["line"],
                        "problem": stale_problem,
                    },
                }
            )
            continue
        active_exemptions[exemption_path] = exemption
        if exemption["kind"] == "PENDING":
            pending_exemption_count += 1

    for manifest in manifests:
        manifest_path = manifest["path"]
        if coverage[manifest_path] is not None or manifest_path in active_exemptions:
            continue
        empty_build = empty_builds[manifest_path]
        if empty_build is None:
            diagnostics.append(
                {
                    "code": "ORPHAN_CRATE_UNLISTED",
                    "severity": "error",
                    "path": manifest["path"],
                    "details": {"manifest_kind": manifest["kind"]},
                }
            )
        else:
            diagnostics.append(
                {
                    "code": "ORPHAN_CRATE_EMPTY_BUILD",
                    "severity": "error",
                    "path": manifest["path"],
                    "details": {
                        "build_path": empty_build["path"],
                        "manifest_kind": manifest["kind"],
                    },
                }
            )

    diagnostics.sort(
        key=lambda item: (
            item["code"],
            item.get("path", ""),
            item.get("package", ""),
            json.dumps(item.get("details", {}), sort_keys=True),
        )
    )
    return diagnostics, pending_exemption_count


def _expected_tracked_artifact_validation(
    options: dict[str, Any],
) -> list[dict[str, Any]]:
    diagnostics: list[dict[str, Any]] = []
    for entry in options["tracked_artifact_snapshot"]["entries"]:
        normalized_path, problem = _normalize_tracked_artifact_path(entry["path"])
        details = {
            "ordinal": entry["ordinal"],
            "entry_kind": entry["entry_kind"],
        }
        if problem is not None:
            details["problem"] = problem
            diagnostics.append(
                {
                    "code": "TRACKED_ARTIFACT_PATH_INVALID",
                    "severity": "error",
                    "path": TRACKED_ARTIFACT_REDACTED_PATH,
                    "details": details,
                }
            )
            continue
        if normalized_path is None:
            raise ConformanceError(
                "CASE_VALIDATION_SNAPSHOT_INCONSISTENT",
                "valid tracked artifact path did not normalize",
            )
        if any(
            tracked_unicode.nfkc_casefold(component)
            == TRACKED_ARTIFACT_COMPONENT_IDENTITY
            for component in normalized_path.split("/")
        ):
            diagnostics.append(
                {
                    "code": "TRACKED_ARTIFACT_FORBIDDEN",
                    "severity": "error",
                    "path": normalized_path,
                    "details": details,
                }
            )
    return sorted(
        diagnostics,
        key=lambda item: (
            item["code"],
            item.get("path", ""),
            item.get("package", ""),
            json.dumps(item.get("details", {}), sort_keys=True),
        ),
    )


def _expected_validation_diagnostics(
    options: dict[str, Any],
) -> list[dict[str, Any]]:
    checks = set(options["checks"])
    packages = _package_index(options["packages"])
    prerequisites: dict[str, set[str]] = {name: set() for name in packages}
    for prerequisite, dependent in options["dependency_edges"]:
        prerequisites[dependent].add(prerequisite)

    closure_cache: dict[str, frozenset[str]] = {}

    def prerequisite_closure(name: str) -> frozenset[str]:
        if name in closure_cache:
            return closure_cache[name]
        result: set[str] = set()
        pending = list(prerequisites[name])
        while pending:
            prerequisite = pending.pop()
            if prerequisite in result:
                continue
            result.add(prerequisite)
            pending.extend(prerequisites[prerequisite])
        closure = frozenset(result)
        closure_cache[name] = closure
        return closure

    def build_path(package: dict[str, Any]) -> str:
        filename = "BUILD_windows" if options["platform"] == "windows" else "BUILD"
        return f"{package['rel_path']}/{filename}"

    diagnostics: list[dict[str, Any]] = []
    if "build_file_presence" in checks:
        for package in packages.values():
            state = package["build_file_state"]
            if state == "present":
                continue
            diagnostics.append(
                {
                    "code": {
                        "missing": "BUILD_FILE_MISSING",
                        "empty": "BUILD_FILE_EMPTY",
                    }[state],
                    "severity": "error",
                    "path": package["rel_path"],
                }
            )
    if "local_dependency_declarations" in checks:
        for name, package in packages.items():
            undeclared = sorted(
                set(package["build_references"])
                - {name}
                - set(prerequisite_closure(name))
            )
            if undeclared:
                diagnostics.append(
                    {
                        "code": "LOCAL_DEPENDENCY_UNDECLARED",
                        "severity": "error",
                        "path": build_path(package),
                        "package": name,
                        "details": {"undeclared_references": undeclared},
                    }
                )
    if "standalone_prerequisites" in checks:
        standalone_languages = {"perl", "python", "typescript"}
        for name, package in packages.items():
            if package["language"] not in standalone_languages:
                continue
            missing = sorted(
                set(prerequisite_closure(name)) - set(package["build_references"])
            )
            if missing:
                diagnostics.append(
                    {
                        "code": "STANDALONE_PREREQUISITE_MISSING",
                        "severity": "error",
                        "path": build_path(package),
                        "package": name,
                        "details": {"missing_prerequisites": missing},
                    }
                )
    if "starlark_declarations" in checks:
        for name, package in packages.items():
            if not package["is_starlark"]:
                continue
            invalid_dependencies = sorted(
                dependency
                for dependency in package["declared_deps"]
                if dependency not in packages
                or dependency not in prerequisite_closure(name)
            )
            invalid_sources = sorted(
                pattern
                for pattern in package["declared_srcs"]
                if portable_glob_error(pattern) is not None
            )
            if invalid_dependencies:
                diagnostics.append(
                    {
                        "code": "STARLARK_DEPENDENCY_INVALID",
                        "severity": "error",
                        "path": build_path(package),
                        "package": name,
                        "details": {
                            "invalid_dependencies": invalid_dependencies,
                        },
                    }
                )
            if invalid_sources:
                diagnostics.append(
                    {
                        "code": "STARLARK_SOURCE_INVALID",
                        "severity": "error",
                        "path": build_path(package),
                        "package": name,
                        "details": {"invalid_sources": invalid_sources},
                    }
                )
    if "identity_uniqueness" in checks:
        for name, package in packages.items():
            candidates = package["identity_candidates"]
            if len(candidates) != 1 or candidates[0] != package["rel_path"]:
                diagnostics.append(
                    {
                        "code": "IDENTITY_AMBIGUOUS",
                        "severity": "error",
                        "path": package["rel_path"],
                        "package": name,
                        "details": {"candidate_roots": candidates},
                    }
                )
    if "manifest_uniqueness" in checks:
        for name, package in packages.items():
            candidates = package["manifest_candidates"]
            if len(candidates) > 1:
                diagnostics.append(
                    {
                        "code": "MANIFEST_AMBIGUOUS",
                        "severity": "error",
                        "path": package["rel_path"],
                        "package": name,
                        "details": {"candidate_manifests": candidates},
                    }
                )
    if "toolchain_support" in checks:
        for name, package in packages.items():
            if package["language"] not in LANGUAGE_TOOLCHAINS:
                diagnostics.append(
                    {
                        "code": "TOOLCHAIN_UNSUPPORTED",
                        "severity": "error",
                        "path": package["rel_path"],
                        "package": name,
                        "details": {"language": package["language"]},
                    }
                )
    if "path_safety" in checks:
        for name, package in packages.items():
            unsafe_paths = sorted(
                path
                for path in package["validation_paths"]
                if portable_path_error(path) is not None
            )
            if unsafe_paths:
                diagnostics.append(
                    {
                        "code": "PATH_UNSAFE",
                        "severity": "error",
                        "path": package["rel_path"],
                        "package": name,
                        "details": {"unsafe_paths": unsafe_paths},
                    }
                )
    if "lua_windows_sibling_parity" in checks:
        for package in packages.values():
            missing = sorted(
                set(package["canonical_lua_sibling_installs"])
                - set(package["windows_lua_sibling_installs"])
            )
            if not missing:
                continue
            diagnostics.append(
                {
                    "code": "STANDALONE_PREREQUISITE_MISSING",
                    "severity": "error",
                    "path": f"{package['rel_path']}/BUILD_windows",
                    "package": package["name"],
                    "details": {
                        "missing_sibling_installs": missing,
                        "windows_build_file_state": package["windows_build_file_state"],
                    },
                }
            )
    if "orphan_crate_coverage" in checks:
        orphan_diagnostics, _ = _expected_orphan_validation(options)
        diagnostics.extend(orphan_diagnostics)
    if "tracked_artifact_absence" in checks:
        diagnostics.extend(_expected_tracked_artifact_validation(options))
    return sorted(
        diagnostics,
        key=lambda item: (
            item["code"],
            item.get("path", ""),
            item.get("package", ""),
            json.dumps(item.get("details", {}), sort_keys=True),
        ),
    )


def _validate_pure_result_semantics(
    case: dict[str, Any],
    result: dict[str, Any],
    staged_files: list[WorkspaceFile],
    prefix: str,
    source_input_registry: dict[str, Any] | None = None,
    repository_source_input_boundary: dict[str, Any] | None = None,
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
    elif domain == "source_collection" and outcome == "ok":
        actual_files = sorted(payload["files"], key=lambda item: item["path"])
        if options["mode"] == "repository_boundary":
            expected_files = _expected_repository_source_collection(
                options,
                repository_source_input_boundary,
            )
        else:
            expected_files = _expected_source_collection(options, source_input_registry)
        if actual_files != expected_files:
            raise ConformanceError(
                f"{prefix}_SOURCE_COLLECTION_MISMATCH",
                "source collection does not match pruning, link, mode, and digest rules",
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
        expected_diagnostics = _expected_validation_diagnostics(options)
        expected_codes = sorted(
            {diagnostic["code"] for diagnostic in expected_diagnostics}
        )
        if "orphan_crate_coverage" in set(options["checks"]):
            _, expected_pending_count = _expected_orphan_validation(options)
            if payload.get("pending_exemption_count") != expected_pending_count:
                raise ConformanceError(
                    f"{prefix}_VALIDATION_INCONSISTENT",
                    "pending exemption count does not match the normalized snapshot",
                )
        diagnostic_projection = [
            {
                key: diagnostic[key]
                for key in ("code", "severity", "path", "package", "details")
                if key in diagnostic
            }
            for diagnostic in result["diagnostics"]
        ]
        diagnostic_projection.sort(
            key=lambda item: (
                item["code"],
                item.get("path", ""),
                item.get("package", ""),
                json.dumps(item.get("details", {}), sort_keys=True),
            )
        )
        if (
            sorted(codes) != expected_codes
            or diagnostic_projection != expected_diagnostics
        ):
            raise ConformanceError(
                f"{prefix}_VALIDATION_INCONSISTENT",
                "validation diagnostics do not match the normalized snapshot",
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
        parsed, parser_diagnostic = _parse_cli_argv(options["argv"])
        expected_exit = {
            "success": 0,
            "package_failure": 1,
            "validation_failure": 1,
        }[options["dispatch_outcome"]]
        cli_expected_diagnostics: list[dict[str, str]] = []
        expected_payload: dict[str, Any]
        if parser_diagnostic is not None:
            expected_exit = 2
            expected_payload = {"exit_code": 2}
            cli_expected_diagnostics = [
                {"code": parser_diagnostic, "severity": "error"}
            ]
        else:
            expected_payload = {"exit_code": expected_exit, "parsed": parsed}
            if options["dispatch_outcome"] == "package_failure":
                cli_expected_diagnostics = [
                    {"code": "CLI_PACKAGE_FAILED", "severity": "error"}
                ]
            elif options["dispatch_outcome"] == "validation_failure":
                cli_expected_diagnostics = [
                    {"code": "CLI_VALIDATION_FAILED", "severity": "error"}
                ]
        expected_outcome = "ok" if expected_exit == 0 else "error"
        if payload["exit_code"] != expected_exit or outcome != expected_outcome:
            raise ConformanceError(
                f"{prefix}_CLI_EXIT_MISMATCH",
                "CLI parse/dispatch outcome and exit code disagree",
            )
        if payload.get("parsed") != expected_payload.get("parsed"):
            raise ConformanceError(
                f"{prefix}_CLI_PARSE_MISMATCH",
                "CLI typed parse result does not match the inert argv oracle",
            )
        if result["diagnostics"] != cli_expected_diagnostics:
            raise ConformanceError(
                f"{prefix}_CLI_DIAGNOSTIC_MISMATCH",
                "CLI diagnostics do not match the inert argv oracle",
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
    elif domain == "source_collection" and "files" in payload:
        payload["files"].sort(key=lambda item: item["path"])
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
    source_input_registry: dict[str, Any] | None = None,
    repository_source_input_boundary: dict[str, Any] | None = None,
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
    _validate_pure_case_semantics(
        case,
        staged_files,
        source_input_registry,
        repository_source_input_boundary,
    )
    _validate_pure_result_semantics(
        case,
        actual,
        staged_files,
        "RESULT",
        source_input_registry,
        repository_source_input_boundary,
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
    source_input_registry: dict[str, Any] | None = None,
    repository_source_input_boundary: dict[str, Any] | None = None,
) -> list[WorkspaceFile]:
    reject_execution_intent(case)
    _validate_schema(case, case_schema, "CASE_SCHEMA_INVALID")
    reject_unmodeled_domain(case)
    _validate_case_identity(case)
    _validate_case_plan_inputs(case, plan_schema)
    _validate_input_paths(case)
    staged_files = preflight_workspace(case)
    pure_domain_schema = pure_domain_schema or load_document(
        DEFAULT_FIXTURE_ROOT / "pure-domains.schema.json"
    )
    _validate_pure_case_semantics(
        case,
        staged_files,
        source_input_registry,
        repository_source_input_boundary,
    )
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
        source_input_registry,
        repository_source_input_boundary,
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
    source_input_registry_schema = load_document(
        DEFAULT_FIXTURE_ROOT / "language-source-input-registry.schema.json"
    )
    source_input_registry = load_document(
        fixture_root / "language-source-input-registry.json"
    )
    repository_source_boundary_schema = load_document(
        DEFAULT_FIXTURE_ROOT / "repository-source-input-boundary.schema.json"
    )
    repository_source_boundary = load_document(
        fixture_root / "repository-source-input-boundary.json"
    )
    plan_schema = load_document(
        REPO_ROOT / "code" / "specs" / "schemas" / "build-plan-v1.schema.json"
    )
    manifest_summary = _validate_manifest(manifest, manifest_schema)
    source_input_summary = _validate_source_input_registry(
        source_input_registry, source_input_registry_schema
    )
    repository_source_summary = _validate_repository_source_input_boundary(
        repository_source_boundary,
        repository_source_boundary_schema,
        source_input_registry,
    )

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
        case = load_case_document(case_path)
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
            source_input_registry=source_input_registry,
            repository_source_input_boundary=repository_source_boundary,
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
        "source_input_language_count": source_input_summary["language_count"],
        "source_input_registry_sha256": source_input_registry_digest(
            source_input_registry
        ),
        "repository_source_boundary_count": repository_source_summary["boundary_count"],
        "repository_source_input_count": repository_source_summary["input_count"],
        "repository_source_boundary_sha256": repository_source_input_boundary_digest(
            repository_source_boundary
        ),
        "conformance_run_count": 0,
        "conformance_status": "not-run",
        "execution_case_count": 0,
        "validated_file_count": validated_files,
        "domains": sorted(domains),
        "status": "valid",
    }


def validate_result_files(case_path: Path, result_path: Path) -> dict[str, Any]:
    case = load_case_document(case_path)
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
