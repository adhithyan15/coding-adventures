"""Pure, bounded graph and diff-selection behavior.

The operations consume only caller-owned immutable values. They do not read a
checkout, invoke Git, inspect the environment, launch processes, or use the
network. JSON and fixture loading intentionally stay outside this boundary.
"""

from __future__ import annotations

import hashlib
import json
import re
from collections import deque
from dataclasses import dataclass
from typing import Literal

from build_tool import tracked_artifact_unicode17 as unicode17
from build_tool.glob_match import match_path, validate_pattern

MAX_PACKAGES = 4_096
MAX_EDGES = 16_384
MAX_SOURCE_GLOBS = 256
MAX_PATH_SCALARS = 512
MAX_PACKAGE_NAME_SCALARS = 240
MAX_DIFF_SELECTION_MATCH_WORK = 50_000_000

BUILD_FRONTS = frozenset(
    {"BUILD", "BUILD_windows", "BUILD_mac", "BUILD_linux", "BUILD_mac_and_linux"}
)
_BOUNDARY_DOMAIN = b"coding-adventures/build-tool-repository-source-input-boundary/v1\0"
_PACKAGE_NAME = re.compile(r"^[a-z0-9][a-z0-9._-]*(/[a-z0-9][a-z0-9._-]*)+$")
_DIGEST = re.compile(r"^[0-9a-f]{64}$")
_DRIVE_PREFIX = re.compile(r"^[A-Za-z]:")
_WINDOWS_RESERVED = frozenset(
    {
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
)

SourceMode = Literal["package_prefix", "strict_globs"]
UnknownPathPolicy = Literal["all", "error"]


@dataclass(frozen=True, slots=True, order=True)
class Edge:
    """One canonical ``[prerequisite, dependent]`` edge."""

    prerequisite: str
    dependent: str


@dataclass(frozen=True, slots=True)
class GraphInput:
    packages: tuple[str, ...]
    edges: tuple[Edge, ...]


@dataclass(frozen=True, slots=True)
class GraphResult:
    edges: tuple[Edge, ...]
    levels: tuple[tuple[str, ...], ...]
    error_code: str = ""


@dataclass(frozen=True, slots=True)
class PackageSpec:
    name: str
    rel_path: str
    source_mode: SourceMode
    source_globs: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class AppliesTo:
    exact_roots: tuple[str, ...]
    descendant_roots: tuple[str, ...]
    excluded_roots: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class BoundaryInput:
    path: str
    role: str
    generated_component: str = ""


@dataclass(frozen=True, slots=True)
class BoundaryRule:
    id: str
    input_origin: str
    applies_to: AppliesTo
    inputs: tuple[BoundaryInput, ...]
    reason: str
    owner: str


@dataclass(frozen=True, slots=True)
class RepositoryBoundary:
    schema_version: int
    language_source_input_registry_sha256: str
    boundaries: tuple[BoundaryRule, ...]

    def digest(self) -> str:
        """Return the versioned digest of this exact inert boundary."""

        document = {
            "schema_version": self.schema_version,
            "language_source_input_registry_sha256": (
                self.language_source_input_registry_sha256
            ),
            "boundaries": [_boundary_rule_document(rule) for rule in self.boundaries],
        }
        encoded = json.dumps(
            document,
            ensure_ascii=False,
            separators=(",", ":"),
            sort_keys=True,
        ).encode("utf-8")
        framed = _BOUNDARY_DOMAIN + len(encoded).to_bytes(8, "big") + encoded
        return hashlib.sha256(framed).hexdigest()


@dataclass(frozen=True, slots=True)
class DiffSelectionInput:
    packages: tuple[PackageSpec, ...]
    edges: tuple[Edge, ...]
    forced_packages: tuple[str, ...]
    unknown_path_policy: UnknownPathPolicy
    changed_paths: tuple[str, ...]
    boundary_sha256: str = ""
    boundary: RepositoryBoundary | None = None


@dataclass(frozen=True, slots=True)
class DiffSelectionResult:
    changed_packages: tuple[str, ...]
    affected_packages: tuple[str, ...]
    prerequisite_packages: tuple[str, ...]
    error_code: str = ""


@dataclass(frozen=True, slots=True)
class _ValidatedGraph:
    names: frozenset[str]
    edges: tuple[Edge, ...]
    dependents: dict[str, tuple[str, ...]]
    prerequisites: dict[str, tuple[str, ...]]


def evaluate_graph(input_value: GraphInput) -> GraphResult:
    """Evaluate canonical edges and deterministic prerequisite-first levels."""

    graph = _validate_graph(input_value.packages, input_value.edges)
    levels = _levels(graph)
    if levels is None:
        return GraphResult((), (), "GRAPH_CYCLE")
    return GraphResult(graph.edges, levels)


def evaluate_diff_selection(input_value: DiffSelectionInput) -> DiffSelectionResult:
    """Evaluate changed, dependent-closed, and prerequisite-only package sets."""

    names = tuple(package.name for package in input_value.packages)
    graph = _validate_graph(names, input_value.edges)
    if _levels(graph) is None:
        raise ValueError("DIFF_EDGE_CYCLE")

    packages = _validate_diff_input(input_value, graph)
    boundary_consumers = _boundary_reverse_index(input_value, packages)

    remaining_work = MAX_DIFF_SELECTION_MATCH_WORK
    for package in input_value.packages:
        if package.source_mode != "strict_globs":
            continue
        pattern_factor = sum(len(pattern) + 1 for pattern in package.source_globs)
        for path in input_value.changed_paths:
            if not _inside(path, package.rel_path):
                continue
            relative = _relative(path, package.rel_path)
            if _basename(relative) in BUILD_FRONTS:
                continue
            path_factor = len(relative) + 1
            if pattern_factor and pattern_factor > remaining_work // path_factor:
                return _diff_error("DIFF_MATCH_LIMIT_EXCEEDED")
            remaining_work -= pattern_factor * path_factor

    changed = set(input_value.forced_packages)
    unknown = False
    for path in input_value.changed_paths:
        consumers = boundary_consumers.get(path, frozenset())
        changed.update(consumers)
        known = bool(consumers)
        for package in input_value.packages:
            if not _inside(path, package.rel_path):
                continue
            known = True
            relative = _relative(path, package.rel_path)
            if (
                package.source_mode == "package_prefix"
                or _basename(relative) in BUILD_FRONTS
                or any(
                    match_path(pattern, relative) for pattern in package.source_globs
                )
            ):
                changed.add(package.name)
        if not known:
            unknown = True

    if unknown:
        if input_value.unknown_path_policy == "error":
            return _diff_error("DIFF_UNKNOWN_PATH")
        changed = set(packages)

    affected = _closure(changed, graph.dependents)
    prerequisite_closure = _closure(affected, graph.prerequisites)
    prerequisite_closure.difference_update(affected)
    return DiffSelectionResult(
        tuple(sorted(changed)),
        tuple(sorted(affected)),
        tuple(sorted(prerequisite_closure)),
    )


def _validate_graph(
    packages: tuple[str, ...], edges: tuple[Edge, ...]
) -> _ValidatedGraph:
    _require(len(packages) <= MAX_PACKAGES, "GRAPH_PACKAGE_LIMIT_EXCEEDED")
    _require(len(edges) <= MAX_EDGES, "GRAPH_EDGE_LIMIT_EXCEEDED")
    names: set[str] = set()
    for name in packages:
        _require(_valid_package_name(name), "GRAPH_PACKAGE_INVALID")
        _require(name not in names, "GRAPH_PACKAGE_DUPLICATE")
        names.add(name)

    unique_edges: set[Edge] = set()
    dependents: dict[str, list[str]] = {name: [] for name in names}
    prerequisites: dict[str, list[str]] = {name: [] for name in names}
    for edge in edges:
        _require(
            edge.prerequisite in names and edge.dependent in names,
            "GRAPH_EDGE_UNKNOWN",
        )
        _require(edge.prerequisite != edge.dependent, "GRAPH_EDGE_SELF")
        _require(edge not in unique_edges, "GRAPH_EDGE_DUPLICATE")
        unique_edges.add(edge)
        dependents[edge.prerequisite].append(edge.dependent)
        prerequisites[edge.dependent].append(edge.prerequisite)

    return _ValidatedGraph(
        frozenset(names),
        tuple(sorted(edges)),
        {name: tuple(sorted(values)) for name, values in dependents.items()},
        {name: tuple(sorted(values)) for name, values in prerequisites.items()},
    )


def _levels(graph: _ValidatedGraph) -> tuple[tuple[str, ...], ...] | None:
    indegree = {name: 0 for name in graph.names}
    for edge in graph.edges:
        indegree[edge.dependent] += 1
    ready = sorted(name for name, degree in indegree.items() if degree == 0)
    levels: list[tuple[str, ...]] = []
    visited = 0
    while ready:
        level = tuple(ready)
        levels.append(level)
        next_ready: list[str] = []
        for name in level:
            visited += 1
            for dependent in graph.dependents[name]:
                indegree[dependent] -= 1
                if indegree[dependent] == 0:
                    next_ready.append(dependent)
        ready = sorted(next_ready)
    return tuple(levels) if visited == len(graph.names) else None


def _validate_diff_input(
    input_value: DiffSelectionInput, graph: _ValidatedGraph
) -> dict[str, PackageSpec]:
    packages: dict[str, PackageSpec] = {}
    root_identities: list[str] = []
    for package in input_value.packages:
        _require(_valid_package_name(package.name), "DIFF_PACKAGE_INVALID")
        _require(_valid_portable_path(package.rel_path), "DIFF_PATH_INVALID")
        root_identity = unicode17.casefold(unicode17.nfc(package.rel_path))
        for prior in root_identities:
            _require(
                root_identity != prior
                and not root_identity.startswith(prior + "/")
                and not prior.startswith(root_identity + "/"),
                "DIFF_PATH_INVALID",
            )
        root_identities.append(root_identity)
        _require(
            package.source_mode in {"package_prefix", "strict_globs"},
            "DIFF_SOURCE_MODE_INVALID",
        )
        _require(
            len(package.source_globs) <= MAX_SOURCE_GLOBS
            and len(set(package.source_globs)) == len(package.source_globs),
            "DIFF_GLOB_INVALID",
        )
        for pattern in package.source_globs:
            _require(_valid_portable_glob(pattern), "DIFF_GLOB_INVALID")
            try:
                validate_pattern(pattern)
            except ValueError as error:
                raise ValueError("DIFF_GLOB_INVALID") from error
        _require(
            package.source_mode == "strict_globs" or not package.source_globs,
            "DIFF_GLOB_INVALID",
        )
        _require(package.name not in packages, "DIFF_PACKAGE_DUPLICATE")
        packages[package.name] = package

    _require(set(packages) == set(graph.names), "DIFF_PACKAGE_INVALID")
    _require(input_value.unknown_path_policy in {"all", "error"}, "DIFF_POLICY_INVALID")
    _require(
        len(input_value.forced_packages) <= MAX_PACKAGES
        and len(set(input_value.forced_packages)) == len(input_value.forced_packages),
        "DIFF_FORCED_PACKAGE_INVALID",
    )
    _require(
        len(input_value.changed_paths) <= MAX_PACKAGES
        and len(set(input_value.changed_paths)) == len(input_value.changed_paths),
        "DIFF_PATH_INVALID",
    )
    for path in input_value.changed_paths:
        _require(_valid_portable_path(path), "DIFF_PATH_INVALID")
    for forced in input_value.forced_packages:
        _require(forced in packages, "DIFF_FORCED_PACKAGE_UNKNOWN")
    return packages


def _boundary_reverse_index(
    input_value: DiffSelectionInput, packages: dict[str, PackageSpec]
) -> dict[str, frozenset[str]]:
    if not input_value.boundary_sha256:
        _require(input_value.boundary is None, "DIFF_BOUNDARY_DIGEST_MISMATCH")
        return {}
    _require(
        bool(_DIGEST.fullmatch(input_value.boundary_sha256)),
        "DIFF_BOUNDARY_DIGEST_MISMATCH",
    )
    boundary = input_value.boundary
    _require(
        boundary is not None and boundary.digest() == input_value.boundary_sha256,
        "DIFF_BOUNDARY_DIGEST_MISMATCH",
    )
    if boundary is None:
        raise ValueError("DIFF_BOUNDARY_DIGEST_MISMATCH")

    mutable: dict[str, set[str]] = {}
    for package in packages.values():
        for rule in boundary.boundaries:
            if not _applies(rule.applies_to, package.rel_path):
                continue
            for boundary_input in rule.inputs:
                mutable.setdefault(boundary_input.path, set()).add(package.name)
    return {path: frozenset(values) for path, values in mutable.items()}


def _boundary_rule_document(rule: BoundaryRule) -> dict[str, object]:
    return {
        "id": rule.id,
        "input_origin": rule.input_origin,
        "applies_to": {
            "exact_roots": list(rule.applies_to.exact_roots),
            "descendant_roots": list(rule.applies_to.descendant_roots),
            "excluded_roots": list(rule.applies_to.excluded_roots),
        },
        "inputs": [_boundary_input_document(value) for value in rule.inputs],
        "reason": rule.reason,
        "owner": rule.owner,
    }


def _boundary_input_document(value: BoundaryInput) -> dict[str, str]:
    result = {"path": value.path, "role": value.role}
    if value.generated_component:
        result["generated_component"] = value.generated_component
    return result


def _closure(seeds: set[str], adjacency: dict[str, tuple[str, ...]]) -> set[str]:
    result = set(seeds)
    pending = deque(seeds)
    while pending:
        for next_name in adjacency[pending.popleft()]:
            if next_name not in result:
                result.add(next_name)
                pending.append(next_name)
    return result


def _applies(applies_to: AppliesTo, root: str) -> bool:
    if root in applies_to.exact_roots:
        return True
    if root in applies_to.excluded_roots:
        return False
    return any(root.startswith(parent + "/") for parent in applies_to.descendant_roots)


def _inside(path: str, root: str) -> bool:
    return path == root or path.startswith(root + "/")


def _relative(path: str, root: str) -> str:
    return "" if path == root else path[len(root) + 1 :]


def _basename(path: str) -> str:
    return path.rsplit("/", 1)[-1]


def _valid_package_name(value: str) -> bool:
    return len(value) <= MAX_PACKAGE_NAME_SCALARS and bool(
        _PACKAGE_NAME.fullmatch(value)
    )


def _valid_portable_path(value: str) -> bool:
    if (
        not value
        or len(value) > MAX_PATH_SCALARS
        or unicode17.nfc(value) != value
        or value.startswith("/")
        or _DRIVE_PREFIX.match(value)
        or "\\" in value
        or "//" in value
        or any(ord(character) < 32 or character in '<>:"|?*' for character in value)
    ):
        return False
    return all(
        segment not in {"", ".", ".."}
        and not segment.endswith((" ", "."))
        and not _reserved(segment)
        for segment in value.split("/")
    )


def _valid_portable_glob(value: str) -> bool:
    if (
        not value
        or len(value) > MAX_PATH_SCALARS
        or unicode17.nfc(value) != value
        or value.startswith("/")
        or _DRIVE_PREFIX.match(value)
        or "\\" in value
        or "//" in value
        or any(ord(character) < 32 or character in '<>:"|?' for character in value)
    ):
        return False
    for segment in value.split("/"):
        if segment in {"", ".", ".."} or segment.endswith((" ", ".")):
            return False
        if not any(character in segment for character in "*[]{}") and _reserved(
            segment
        ):
            return False
    return True


def _reserved(segment: str) -> bool:
    base = segment.split(".", 1)[0]
    return unicode17.full_uppercase(base) in _WINDOWS_RESERVED


def _diff_error(code: str) -> DiffSelectionResult:
    return DiffSelectionResult((), (), (), code)


def _require(condition: bool, code: str) -> None:
    if not condition:
        raise ValueError(code)
