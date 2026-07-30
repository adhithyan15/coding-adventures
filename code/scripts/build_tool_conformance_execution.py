#!/usr/bin/env python3
"""Validate trusted-execution policy without executing fixture code.

This module is the process-free policy layer. It deliberately imports no
process API, creates no workspace, and has no host-execution fallback. Platform
backends land separately. The Linux OCI identity schema is checked here, but
its process-owning capability preflight is never imported. Until later
authority and execution tranches land, ``run-case`` can only return a stable
non-passing result after authority checks succeed.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
import stat
import struct
import sys
import unicodedata
from collections.abc import Iterable, Sequence
from pathlib import Path
from typing import Any

import build_tool_conformance as bootstrap

DEFAULT_FIXTURE_ROOT = bootstrap.DEFAULT_FIXTURE_ROOT
DEFAULT_EXECUTION_CASE_ROOT = DEFAULT_FIXTURE_ROOT / "execution-cases"
MAX_EXECUTION_CASE_BYTES = bootstrap.MAX_DOCUMENT_BYTES
SHA256_PATTERN = "0123456789abcdef"
PLATFORM_BACKEND_KINDS = {
    "darwin": "macos_isolated",
    "linux": "linux_oci",
    "windows": "windows_appcontainer",
}


def _read_raw_regular(
    path: Path,
    *,
    max_bytes: int = MAX_EXECUTION_CASE_BYTES,
) -> bytes:
    """Read a bounded regular file without following its final link."""

    try:
        with bootstrap._open_regular_no_follow(path) as source:
            if not stat.S_ISREG(os.fstat(source.fileno()).st_mode):
                raise bootstrap.ConformanceError(
                    "EXECUTION_CORPUS_FILE_INVALID",
                    f"execution corpus member is not a regular file: {path}",
                )
            raw = source.read(max_bytes + 1)
    except bootstrap.ConformanceError:
        raise
    except (OSError, ValueError) as error:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_READ_FAILED",
            f"could not read execution corpus member: {path}",
        ) from error
    if len(raw) > max_bytes:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_FILE_TOO_LARGE",
            f"execution corpus member exceeds {max_bytes} bytes: {path}",
        )
    return raw


def framed_corpus_digest(entries: Iterable[tuple[str, bytes]]) -> str:
    """Hash sorted portable paths and exact bytes with length framing."""

    digest = hashlib.sha256()
    previous_path: str | None = None
    for relative_path, raw in sorted(entries, key=lambda item: item[0]):
        if error := bootstrap.portable_path_error(relative_path):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_PATH_UNSAFE",
                f"unsafe execution corpus path {relative_path!r}: {error}",
            )
        if relative_path != unicodedata.normalize("NFC", relative_path):
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_PATH_UNSAFE",
                f"execution corpus path is not NFC-normalized: {relative_path}",
            )
        if relative_path == previous_path:
            raise bootstrap.ConformanceError(
                "EXECUTION_CORPUS_PATH_DUPLICATE",
                f"duplicate execution corpus path: {relative_path}",
            )
        previous_path = relative_path
        path_bytes = relative_path.encode("utf-8")
        digest.update(struct.pack(">Q", len(path_bytes)))
        digest.update(path_bytes)
        digest.update(struct.pack(">Q", len(raw)))
        digest.update(raw)
    return digest.hexdigest()


def _execution_corpus_entries(case_root: Path) -> list[tuple[str, bytes]]:
    """Open one stable bounded snapshot of direct execution-case members."""

    try:
        root_status = case_root.lstat()
    except OSError as error:
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_MISSING",
            f"execution corpus directory does not exist: {case_root}",
        ) from error
    is_reparse = bool(
        getattr(root_status, "st_file_attributes", 0)
        & getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    )
    if (
        not stat.S_ISDIR(root_status.st_mode)
        or stat.S_ISLNK(root_status.st_mode)
        or is_reparse
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_CORPUS_DIRECTORY_INVALID",
            f"execution corpus path is not a regular directory: {case_root}",
        )
    return [
        (path.relative_to(case_root).as_posix(), _read_raw_regular(path))
        for path in sorted(case_root.glob("*.json"), key=lambda item: item.name)
    ]


def execution_corpus_digest(case_root: Path) -> str:
    """Compute the reviewed execution-corpus digest without JSON decoding."""

    return framed_corpus_digest(_execution_corpus_entries(case_root))


def _is_sha256(value: Any) -> bool:
    return (
        isinstance(value, str)
        and len(value) == 64
        and all(character in SHA256_PATTERN for character in value)
    )


def validate_policy_semantics(policy: dict[str, Any]) -> dict[str, int]:
    """Validate cross-field identities that JSON Schema cannot express."""

    backends = policy["backends"]
    platforms = [item["platform"] for item in backends]
    if platforms != sorted(PLATFORM_BACKEND_KINDS):
        raise bootstrap.ConformanceError(
            "EXECUTION_BACKENDS_NOT_CANONICAL",
            "backends must contain darwin, linux, and windows in sorted order",
        )
    for backend in backends:
        expected_kind = PLATFORM_BACKEND_KINDS[backend["platform"]]
        if backend["kind"] != expected_kind:
            raise bootstrap.ConformanceError(
                "EXECUTION_BACKEND_KIND_MISMATCH",
                f"{backend['platform']} requires backend kind {expected_kind}",
            )

    adapter_keys: set[tuple[str, str]] = set()
    for adapter in policy["adapters"]:
        key = (adapter["language"], adapter["platform"])
        if key in adapter_keys:
            raise bootstrap.ConformanceError(
                "EXECUTION_ADAPTER_DUPLICATE",
                f"duplicate execution adapter identity: {key[0]}/{key[1]}",
            )
        adapter_keys.add(key)
        if error := bootstrap.portable_path_error(adapter["executable"]):
            raise bootstrap.ConformanceError(
                "EXECUTION_ADAPTER_PATH_UNSAFE",
                f"unsafe adapter path {adapter['executable']!r}: {error}",
            )
    return {
        "ready_backend_count": sum(item["status"] == "ready" for item in backends),
        "adapter_count": len(policy["adapters"]),
    }


def _execution_options(case: dict[str, Any]) -> dict[str, Any]:
    input_value = case.get("input")
    if not isinstance(input_value, dict):
        return {}
    options = input_value.get("options")
    return options if isinstance(options, dict) else {}


def _validate_execution_graph(
    package_names: set[str],
    edges: list[list[str]],
) -> None:
    outgoing = {name: [] for name in package_names}
    indegree = {name: 0 for name in package_names}
    for prerequisite, dependent in edges:
        if prerequisite not in package_names or dependent not in package_names:
            raise bootstrap.ConformanceError(
                "EXECUTION_EDGE_UNKNOWN",
                "execution dependency edge references an unknown package",
            )
        outgoing[prerequisite].append(dependent)
        indegree[dependent] += 1
    ready = sorted(name for name, count in indegree.items() if count == 0)
    visited = 0
    while ready:
        current = ready.pop(0)
        visited += 1
        for dependent in sorted(outgoing[current]):
            indegree[dependent] -= 1
            if indegree[dependent] == 0:
                ready.append(dependent)
                ready.sort()
    if visited != len(package_names):
        raise bootstrap.ConformanceError(
            "EXECUTION_GRAPH_CYCLE",
            "execution dependency graph contains a cycle",
        )


def validate_execution_semantics(case: dict[str, Any]) -> None:
    """Validate execution case identities and deterministic result ordering."""

    if case.get("domain") != "execution":
        raise bootstrap.ConformanceError(
            "EXECUTION_DOMAIN_INVALID",
            "execution cases require domain=execution",
        )
    input_value = case.get("input")
    if not isinstance(input_value, dict) or input_value.get("operation") != "execution":
        raise bootstrap.ConformanceError(
            "EXECUTION_OPERATION_INVALID",
            "execution cases require input.operation=execution",
        )
    capabilities = case.get("capabilities")
    if not isinstance(capabilities, list) or not {
        "execution",
        "trusted_execution",
    }.issubset(capabilities):
        raise bootstrap.ConformanceError(
            "EXECUTION_CAPABILITY_MISSING",
            "execution cases require execution and trusted_execution",
        )
    expected = case.get("expected")
    if (
        not isinstance(expected, dict)
        or expected.get("case_id") != case.get("id")
        or expected.get("domain") != "execution"
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_IDENTITY_MISMATCH",
            "execution case and expected result identities must match",
        )

    options = _execution_options(case)
    platform_name = options.get("platform")
    platforms = case.get("platforms")
    if not isinstance(platforms, list) or platform_name not in platforms:
        raise bootstrap.ConformanceError(
            "EXECUTION_PLATFORM_MISMATCH",
            "execution options.platform must be listed in top-level platforms",
        )
    limits = case.get("limits")
    process_limit = limits.get("process_count") if isinstance(limits, dict) else None
    jobs = options.get("jobs")
    if (
        not isinstance(jobs, int)
        or not isinstance(process_limit, int)
        or jobs > process_limit
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_JOB_LIMIT",
            "execution jobs cannot exceed the requested process_count limit",
        )

    packages = options.get("packages")
    if not isinstance(packages, list):
        raise bootstrap.ConformanceError(
            "EXECUTION_PACKAGES_INVALID",
            "execution packages must be an array",
        )
    package_names: set[str] = set()
    normalized_paths: set[str] = set()
    for package in packages:
        name = package["name"]
        if name in package_names:
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_DUPLICATE",
                f"duplicate execution package: {name}",
            )
        package_names.add(name)
        rel_path = package["rel_path"]
        if error := bootstrap.portable_path_error(rel_path):
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_PATH_UNSAFE",
                f"unsafe execution package path {rel_path!r}: {error}",
            )
        normalized = unicodedata.normalize("NFC", rel_path).casefold()
        if normalized in normalized_paths:
            raise bootstrap.ConformanceError(
                "EXECUTION_PACKAGE_PATH_DUPLICATE",
                f"duplicate normalized execution package path: {rel_path}",
            )
        normalized_paths.add(normalized)
        if package["resource_locks"] != sorted(package["resource_locks"]):
            raise bootstrap.ConformanceError(
                "EXECUTION_LOCKS_NOT_CANONICAL",
                f"resource locks are not sorted for {name}",
            )

    edges = options.get("dependency_edges")
    if not isinstance(edges, list):
        raise bootstrap.ConformanceError(
            "EXECUTION_EDGES_INVALID",
            "execution dependency_edges must be an array",
        )
    _validate_execution_graph(package_names, edges)

    outcome = expected["outcome"]
    if outcome in {"ok", "error"}:
        result_packages = expected["result"]["packages"]
        result_names = [package["name"] for package in result_packages]
        if result_names != sorted(result_names):
            raise bootstrap.ConformanceError(
                "EXECUTION_RESULT_NOT_CANONICAL",
                "execution result packages must be sorted by name",
            )
        if set(result_names) != package_names:
            raise bootstrap.ConformanceError(
                "EXECUTION_RESULT_PACKAGE_MISMATCH",
                "execution result must classify every input package exactly once",
            )
        package_by_name = {package["name"]: package for package in packages}
        for package_result in result_packages:
            command_results = package_result["commands"]
            indices = [command["index"] for command in command_results]
            if indices != list(range(len(command_results))):
                raise bootstrap.ConformanceError(
                    "EXECUTION_COMMAND_INDEX_INVALID",
                    f"command indices are not canonical for {package_result['name']}",
                )
            command_count = len(package_by_name[package_result["name"]]["commands"])
            if len(command_results) != command_count:
                raise bootstrap.ConformanceError(
                    "EXECUTION_COMMAND_COUNT_MISMATCH",
                    f"command result count differs for {package_result['name']}",
                )


def _load_contract_documents(
    fixture_root: Path,
) -> tuple[
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
    dict[str, Any],
]:
    return (
        bootstrap.load_document(fixture_root / "schema.json"),
        bootstrap.load_document(fixture_root / "result.schema.json"),
        bootstrap.load_document(fixture_root / "execution.schema.json"),
        bootstrap.load_document(fixture_root / "execution-policy.schema.json"),
        bootstrap.load_document(fixture_root / "execution-policy.json"),
    )


def validate_contract(
    fixture_root: Path = DEFAULT_FIXTURE_ROOT,
) -> dict[str, Any]:
    """Validate schemas, policy, digest, and inert execution cases."""

    fixture_root = fixture_root.resolve()
    (
        case_schema,
        result_schema,
        execution_schema,
        policy_schema,
        policy,
    ) = _load_contract_documents(fixture_root)
    linux_oci_schema = bootstrap.load_document(
        fixture_root / "linux-oci-backend.schema.json"
    )
    for schema in (
        case_schema,
        result_schema,
        execution_schema,
        policy_schema,
        linux_oci_schema,
    ):
        bootstrap._schema_errors({}, schema)
    bootstrap._validate_schema(
        policy,
        policy_schema,
        "EXECUTION_POLICY_SCHEMA_INVALID",
    )
    summary = validate_policy_semantics(policy)
    case_root = fixture_root / "execution-cases"
    corpus_entries = _execution_corpus_entries(case_root)
    digest = framed_corpus_digest(corpus_entries)
    if digest != policy["execution_corpus_sha256"]:
        raise bootstrap.ConformanceError(
            "EXECUTION_POLICY_CORPUS_MISMATCH",
            "checked-in execution policy does not match the execution corpus",
        )

    case_ids: set[str] = set()
    for relative_path, raw in corpus_entries:
        case = bootstrap.strict_load_bytes(raw)
        bootstrap._validate_schema(
            case,
            case_schema,
            "EXECUTION_CASE_SCHEMA_INVALID",
        )
        expected = case.get("expected")
        if not isinstance(expected, dict):
            raise bootstrap.ConformanceError(
                "EXECUTION_EXPECTED_INVALID",
                "execution case expected result is missing",
            )
        bootstrap._validate_schema(
            expected,
            result_schema,
            "EXECUTION_RESULT_SCHEMA_INVALID",
        )
        projection = {
            "domain": case.get("domain"),
            "outcome": expected.get("outcome"),
            "input": case.get("input"),
            "result": expected.get("result"),
        }
        bootstrap._validate_schema(
            projection,
            execution_schema,
            "EXECUTION_PROJECTION_SCHEMA_INVALID",
        )
        validate_execution_semantics(case)
        case_id = case["id"]
        if case_id in case_ids:
            raise bootstrap.ConformanceError(
                "EXECUTION_CASE_ID_DUPLICATE",
                f"duplicate execution case id in {relative_path}: {case_id}",
            )
        case_ids.add(case_id)

    return {
        "schema_version": 1,
        "execution_case_count": len(corpus_entries),
        "execution_corpus_sha256": digest,
        "ready_backend_count": summary["ready_backend_count"],
        "adapter_count": summary["adapter_count"],
        "status": "valid",
        "conformance_status": "not-run",
    }


def _platform_name() -> str:
    if sys.platform.startswith("linux"):
        return "linux"
    if sys.platform == "darwin":
        return "darwin"
    if os.name == "nt":
        return "windows"
    return "unsupported"


def _nonpassing_skip(code: str, message: str) -> dict[str, Any]:
    return {
        "status": "skipped",
        "outcome": "skipped",
        "conformance_status": "non-passing",
        "diagnostics": [
            {
                "code": code,
                "severity": "error",
                "message": message,
            }
        ],
    }


def run_case(
    case_path: Path,
    *,
    language: str,
    approved_digest: str,
    allow_trusted_execution: bool,
    fixture_root: Path = DEFAULT_FIXTURE_ROOT,
    platform_name: str | None = None,
) -> dict[str, Any]:
    """Check execution authority and fail closed before any process operation."""

    del case_path  # The policy-only tranche never decodes executable case data.
    if not allow_trusted_execution:
        raise bootstrap.ConformanceError(
            "EXECUTION_AUTHORIZATION_REQUIRED",
            "trusted execution requires --allow-trusted-execution",
        )

    fixture_root = fixture_root.resolve()
    _, _, _, policy_schema, policy = _load_contract_documents(fixture_root)
    bootstrap._validate_schema(
        policy,
        policy_schema,
        "EXECUTION_POLICY_SCHEMA_INVALID",
    )
    validate_policy_semantics(policy)
    actual_digest = execution_corpus_digest(fixture_root / "execution-cases")
    if (
        not _is_sha256(approved_digest)
        or approved_digest != actual_digest
        or approved_digest != policy["execution_corpus_sha256"]
    ):
        raise bootstrap.ConformanceError(
            "EXECUTION_DIGEST_MISMATCH",
            "approved digest, policy digest, and corpus digest must match exactly",
        )

    if not policy["enabled"]:
        return _nonpassing_skip(
            "EXECUTION_POLICY_DISABLED",
            "trusted execution policy is disabled",
        )

    selected_platform = platform_name or _platform_name()
    backend = next(
        (item for item in policy["backends"] if item["platform"] == selected_platform),
        None,
    )
    if backend is None or backend["status"] != "ready":
        return _nonpassing_skip(
            "EXECUTION_BACKEND_UNAVAILABLE",
            f"no enforcing trusted-execution backend for {selected_platform}",
        )

    adapter = next(
        (
            item
            for item in policy["adapters"]
            if item["language"] == language and item["platform"] == selected_platform
        ),
        None,
    )
    if adapter is None:
        return _nonpassing_skip(
            "EXECUTION_ADAPTER_UNAVAILABLE",
            f"no reviewed adapter for {language}/{selected_platform}",
        )
    return _nonpassing_skip(
        "EXECUTION_BACKEND_UNIMPLEMENTED",
        "policy authority is valid, but this tranche contains no process backend",
    )


def _build_parser() -> argparse.ArgumentParser:
    parser = argparse.ArgumentParser(
        description="Validate trusted-execution policy without executing code."
    )
    subparsers = parser.add_subparsers(dest="command", required=True)

    validate_parser = subparsers.add_parser(
        "validate-contract",
        help="Validate execution schemas, policy, digest, and inert cases.",
    )
    validate_parser.add_argument(
        "--fixture-root",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
    )

    run_parser = subparsers.add_parser(
        "run-case",
        help="Check execution authority and return a fail-closed result.",
    )
    run_parser.add_argument("--case", type=Path, required=True)
    run_parser.add_argument("--language", required=True)
    run_parser.add_argument("--approved-corpus-sha256", required=True)
    run_parser.add_argument("--allow-trusted-execution", action="store_true")
    run_parser.add_argument(
        "--fixture-root",
        type=Path,
        default=DEFAULT_FIXTURE_ROOT,
    )
    return parser


def main(argv: Sequence[str] | None = None) -> int:
    parser = _build_parser()
    try:
        arguments = parser.parse_args(argv)
    except SystemExit as error:
        return int(error.code)
    try:
        if arguments.command == "validate-contract":
            output = validate_contract(arguments.fixture_root)
            exit_code = 0
        else:
            output = run_case(
                arguments.case,
                language=arguments.language,
                approved_digest=arguments.approved_corpus_sha256,
                allow_trusted_execution=arguments.allow_trusted_execution,
                fixture_root=arguments.fixture_root,
            )
            exit_code = 1
    except bootstrap.ConformanceError as error:
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
        return 2
    print(json.dumps(output, sort_keys=True))
    return exit_code


if __name__ == "__main__":
    raise SystemExit(main())
