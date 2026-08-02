#!/usr/bin/env python3
"""Validate the ADJ curriculum manifest against repository evidence."""

from __future__ import annotations

import argparse
import json
import re
from pathlib import Path, PurePosixPath
from typing import Any

import adj_stdlib_report

DEFAULT_MANIFEST = Path("code/specs/data/adj-stdlib-coverage/manifest.json")
DEFAULT_SCHEMA = Path("code/specs/data/adj-stdlib-coverage/manifest.schema.json")
ID_RE = re.compile(r"^[a-z0-9]+(?:[._-][a-z0-9]+)*$")
SHA256_RE = re.compile(r"^[0-9a-f]{64}$")
BANDS = {"K-2", "3-5", "6-8", "9-12", "pre-clinical", "clinical", "licensure"}
COMPETENCIES = {
    "recall",
    "compute",
    "classify",
    "explain",
    "infer",
    "decide",
    "optimize",
    "interpret",
}
IMPLEMENTATION = {"missing", "partial", "present"}
PROVENANCE = {"missing", "source_labeled", "byte_pinned", "fully_verified"}
TESTS = {"missing", "partial", "present"}
BENCHMARKS = {"missing", "pilot", "held_out"}
CROSSWALKS = {"unmapped", "partial", "mapped"}
ROOT_STATUSES = {"declared", "indexed", "byte_pinned"}


def _load_json(path: Path) -> Any:
    return json.loads(path.read_text(encoding="utf-8"))


def _nonempty_string(value: Any) -> bool:
    return isinstance(value, str) and bool(value.strip())


def _string_list(value: Any) -> bool:
    return isinstance(value, list) and all(_nonempty_string(item) for item in value)


def _safe_repo_path(value: str) -> bool:
    path = PurePosixPath(value)
    return (
        not path.is_absolute()
        and ".." not in path.parts
        and value == path.as_posix()
        and value.startswith("code/")
    )


def discover_library_evidence(root: Path) -> dict[str, dict[str, Any]]:
    report = adj_stdlib_report.build_report(root)
    return {row["path"]: row for row in report["libraries"]}


def _validate_root(item: Any, index: int, errors: list[str]) -> str | None:
    prefix = f"coverage_roots[{index}]"
    if not isinstance(item, dict):
        errors.append(f"{prefix} must be an object")
        return None
    root_id = item.get("id")
    if not _nonempty_string(root_id) or not ID_RE.fullmatch(root_id):
        errors.append(f"{prefix}.id must be a stable lowercase identifier")
        root_id = None
    for field in ("title", "version", "locator"):
        if not _nonempty_string(item.get(field)):
            errors.append(f"{prefix}.{field} must be a non-empty string")
    status = item.get("status")
    if status not in ROOT_STATUSES:
        errors.append(f"{prefix}.status must be one of {sorted(ROOT_STATUSES)}")
    cas_hash = item.get("cas_hash")
    if cas_hash is not None and (
        not isinstance(cas_hash, str) or not SHA256_RE.fullmatch(cas_hash)
    ):
        errors.append(f"{prefix}.cas_hash must be null or a lowercase SHA-256")
    if status == "byte_pinned" and cas_hash is None:
        errors.append(f"{prefix} is byte_pinned but has no cas_hash")
    return root_id


def _validate_status(
    objective: dict[str, Any],
    prefix: str,
    evidence: list[dict[str, Any]],
    errors: list[str],
) -> None:
    status = objective.get("status")
    if not isinstance(status, dict):
        errors.append(f"{prefix}.status must be an object")
        return
    allowed = {
        "implementation": IMPLEMENTATION,
        "provenance": PROVENANCE,
        "tests": TESTS,
        "benchmark": BENCHMARKS,
        "crosswalk": CROSSWALKS,
    }
    for field, values in allowed.items():
        if status.get(field) not in values:
            errors.append(f"{prefix}.status.{field} must be one of {sorted(values)}")

    libraries = objective.get("libraries", [])
    if status.get("implementation") == "present" and not libraries:
        errors.append(f"{prefix} is implemented but names no libraries")
    if status.get("tests") == "present" and any(
        not row.get("test_reference") for row in evidence
    ):
        errors.append(f"{prefix} claims tests present for an unreferenced library")
    if status.get("provenance") in {
        "source_labeled",
        "byte_pinned",
        "fully_verified",
    } and any(not row.get("source_envelope") for row in evidence):
        errors.append(f"{prefix} claims sourced provenance without complete envelopes")
    if status.get("provenance") in {"byte_pinned", "fully_verified"} and any(
        not row.get("pinned_quote") for row in evidence
    ):
        errors.append(f"{prefix} claims byte pins that library evidence does not support")
    source_hashes = objective.get("source_cas_hashes", [])
    if status.get("provenance") == "fully_verified" and not source_hashes:
        errors.append(f"{prefix} is fully_verified but has no source_cas_hashes")
    if status.get("benchmark") == "held_out" and not objective.get("benchmark_paths"):
        errors.append(f"{prefix} claims a held-out benchmark but names no benchmark_paths")
    if status.get("crosswalk") == "mapped" and not objective.get("standards"):
        errors.append(f"{prefix} claims a mapped crosswalk but names no standards")


def _find_cycles(prerequisites: dict[str, list[str]]) -> list[str]:
    colors: dict[str, int] = {}
    stack: list[str] = []
    cycles: list[str] = []

    def visit(node: str) -> None:
        colors[node] = 1
        stack.append(node)
        for dependency in prerequisites.get(node, []):
            if dependency not in prerequisites:
                continue
            if colors.get(dependency, 0) == 0:
                visit(dependency)
            elif colors.get(dependency) == 1:
                start = stack.index(dependency)
                cycles.append(" -> ".join([*stack[start:], dependency]))
        stack.pop()
        colors[node] = 2

    for node in prerequisites:
        if colors.get(node, 0) == 0:
            visit(node)
    return sorted(set(cycles))


def validate_manifest(
    root: Path,
    manifest: Any,
    library_evidence: dict[str, dict[str, Any]] | None = None,
) -> list[str]:
    """Return stable validation errors; an empty list means valid."""

    errors: list[str] = []
    if not isinstance(manifest, dict):
        return ["manifest must be a JSON object"]
    if manifest.get("schema_version") != 1:
        errors.append("schema_version must equal 1")
    if not _nonempty_string(manifest.get("manifest_id")):
        errors.append("manifest_id must be a non-empty string")

    roots = manifest.get("coverage_roots")
    if not isinstance(roots, list):
        errors.append("coverage_roots must be an array")
        roots = []
    root_ids: list[str] = []
    for index, item in enumerate(roots):
        root_id = _validate_root(item, index, errors)
        if root_id is not None:
            root_ids.append(root_id)
    duplicates = sorted(root_id for root_id in set(root_ids) if root_ids.count(root_id) > 1)
    errors.extend(f"duplicate coverage root id: {root_id}" for root_id in duplicates)
    root_id_set = set(root_ids)

    objectives = manifest.get("objectives")
    if not isinstance(objectives, list):
        errors.append("objectives must be an array")
        objectives = []
    if library_evidence is None:
        library_evidence = discover_library_evidence(root)

    objective_ids: list[str] = []
    prerequisites: dict[str, list[str]] = {}
    for index, item in enumerate(objectives):
        prefix = f"objectives[{index}]"
        if not isinstance(item, dict):
            errors.append(f"{prefix} must be an object")
            continue
        objective_id = item.get("id")
        if not _nonempty_string(objective_id) or not ID_RE.fullmatch(objective_id):
            errors.append(f"{prefix}.id must be a stable lowercase identifier")
            objective_id = f"<invalid-{index}>"
        objective_ids.append(objective_id)
        for field in ("title", "domain"):
            if not _nonempty_string(item.get(field)):
                errors.append(f"{prefix}.{field} must be a non-empty string")
        if item.get("band") not in BANDS:
            errors.append(f"{prefix}.band must be one of {sorted(BANDS)}")
        if item.get("competency") not in COMPETENCIES:
            errors.append(f"{prefix}.competency must be one of {sorted(COMPETENCIES)}")

        for field in (
            "coverage_roots",
            "prerequisites",
            "libraries",
            "modalities",
            "source_cas_hashes",
            "benchmark_paths",
        ):
            if not _string_list(item.get(field)):
                errors.append(f"{prefix}.{field} must be an array of non-empty strings")

        objective_roots = item.get("coverage_roots", [])
        for root_id in objective_roots if isinstance(objective_roots, list) else []:
            if root_id not in root_id_set:
                errors.append(f"{prefix} references unknown coverage root: {root_id}")
        prereqs = item.get("prerequisites", [])
        prerequisites[objective_id] = prereqs if isinstance(prereqs, list) else []

        paths = item.get("libraries", [])
        evidence: list[dict[str, Any]] = []
        if isinstance(paths, list):
            for path in paths:
                if not isinstance(path, str) or not _safe_repo_path(path):
                    errors.append(f"{prefix} has unsafe library path: {path!r}")
                    continue
                row = library_evidence.get(path)
                if row is None:
                    errors.append(f"{prefix} references unknown library: {path}")
                else:
                    evidence.append(row)

        standards = item.get("standards")
        if not isinstance(standards, list):
            errors.append(f"{prefix}.standards must be an array")
            standards = []
        for standard_index, standard in enumerate(standards):
            standard_prefix = f"{prefix}.standards[{standard_index}]"
            if not isinstance(standard, dict):
                errors.append(f"{standard_prefix} must be an object")
                continue
            if standard.get("root") not in root_id_set:
                errors.append(f"{standard_prefix}.root is unknown")
            if not _nonempty_string(standard.get("objective")):
                errors.append(f"{standard_prefix}.objective must be non-empty")

        for source_hash in item.get("source_cas_hashes", []):
            if not SHA256_RE.fullmatch(source_hash):
                errors.append(f"{prefix} has malformed source CAS hash: {source_hash}")
        for benchmark_path in item.get("benchmark_paths", []):
            if not _safe_repo_path(benchmark_path) or not (root / benchmark_path).is_file():
                errors.append(f"{prefix} references missing benchmark: {benchmark_path}")
        _validate_status(item, prefix, evidence, errors)

    duplicate_objectives = sorted(
        item for item in set(objective_ids) if objective_ids.count(item) > 1
    )
    errors.extend(f"duplicate objective id: {item}" for item in duplicate_objectives)
    objective_id_set = set(objective_ids)
    for objective_id, prereqs in prerequisites.items():
        for prereq in prereqs:
            if prereq not in objective_id_set:
                errors.append(f"{objective_id} references unknown prerequisite: {prereq}")
    errors.extend(f"prerequisite cycle: {cycle}" for cycle in _find_cycles(prerequisites))
    return sorted(set(errors))


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[2])
    parser.add_argument("--manifest", type=Path, default=DEFAULT_MANIFEST)
    parser.add_argument("--schema", type=Path, default=DEFAULT_SCHEMA)
    args = parser.parse_args()

    root = args.root.resolve()
    manifest_path = args.manifest if args.manifest.is_absolute() else root / args.manifest
    schema_path = args.schema if args.schema.is_absolute() else root / args.schema
    manifest = _load_json(manifest_path)
    schema = _load_json(schema_path)
    errors = validate_manifest(root, manifest)
    if schema.get("$id") != "https://coding-adventures.dev/schemas/adj-stdlib-manifest-v1.json":
        errors.append("manifest schema has an unexpected $id")
    result = {
        "valid": not errors,
        "manifest": manifest_path.relative_to(root).as_posix(),
        "objectives": len(manifest.get("objectives", [])),
        "coverage_roots": len(manifest.get("coverage_roots", [])),
        "errors": sorted(errors),
    }
    print(json.dumps(result, indent=2, sort_keys=True))
    return 0 if not errors else 1


if __name__ == "__main__":
    raise SystemExit(main())
