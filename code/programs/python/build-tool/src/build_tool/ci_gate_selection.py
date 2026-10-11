"""Pure, bounded CI-gate selection over an in-memory registry.

File loading, Git diff acquisition, and publishing Actions outputs belong to
front-door adapters. This module only makes the portable selection decision.
"""

from __future__ import annotations

import unicodedata
from dataclasses import dataclass

from build_tool.glob_match import match_path, validate_pattern

MATCH_WORK_LIMIT = 50_000_000
MATCH_LIMIT_ERROR = "CI_GATE_MATCH_LIMIT_EXCEEDED"
_MACHINERY_FILES = frozenset(
    {".github/workflows/ci.yml", "code/specs/data/ci-gates.json"}
)
_MACHINERY_PREFIXES = (
    "code/programs/go/build-tool/internal/cigates/",
    "code/programs/go/build-tool/internal/globmatch/",
    "code/programs/go/build-tool/internal/gitdiff/",
    "code/programs/go/build-tool/main.go",
)
_RESERVED = frozenset(
    {"CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"}
    | {f"{prefix}{digit}" for prefix in ("COM", "LPT") for digit in "123456789¹²³"}
)


@dataclass(frozen=True, slots=True)
class Gate:
    id: str
    scope: str
    description: str
    packages: tuple[str, ...]
    paths: tuple[str, ...]


@dataclass(frozen=True, slots=True)
class Registry:
    schema_version: int
    gates: tuple[Gate, ...]


@dataclass(frozen=True, slots=True)
class SelectionInput:
    registry: Registry
    affected_packages: tuple[str, ...] | None
    changed_files: tuple[str, ...] | None
    force: bool


@dataclass(frozen=True, slots=True)
class GateDecision:
    id: str
    required: bool
    output_name: str


@dataclass(frozen=True, slots=True)
class SelectionResult:
    gates: tuple[GateDecision, ...]
    error_code: str = ""


def evaluate(selection: SelectionInput) -> SelectionResult:
    """Return every gate in ID order or one non-partial match-work error."""
    gates = _validate_registry(selection.registry)
    changed = selection.changed_files
    run_all = (
        selection.force
        or selection.affected_packages is None
        or changed is None
        or _touches_machinery(changed)
    )
    if run_all:
        return _decisions(gates, frozenset(gate.id for gate in gates))

    assert changed is not None
    assert selection.affected_packages is not None
    candidates = _preflight(gates, changed)
    if candidates is None:
        return SelectionResult((), MATCH_LIMIT_ERROR)

    affected = frozenset(selection.affected_packages)
    pattern_matches: dict[str, bool] = {}

    def pattern_fires(pattern: str) -> bool:
        if pattern not in pattern_matches:
            pattern_matches[pattern] = any(
                match_path(pattern, file) for file in candidates[pattern]
            )
        return pattern_matches[pattern]

    required = frozenset(
        gate.id
        for gate in gates
        if any(package in affected for package in gate.packages)
        or any(pattern_fires(pattern) for pattern in gate.paths)
    )
    return _decisions(gates, required)


def _decisions(gates: tuple[Gate, ...], required: frozenset[str]) -> SelectionResult:
    return SelectionResult(
        tuple(
            GateDecision(gate.id, gate.id in required, _output_name(gate.id))
            for gate in gates
        )
    )


def _output_name(gate_id: str) -> str:
    return "run_" + gate_id.replace("-", "_")


def _validate_registry(registry: Registry) -> tuple[Gate, ...]:
    if registry.schema_version != 1:
        raise ValueError("unsupported CI-gate registry schema")
    if not 1 <= len(registry.gates) <= 128:
        raise ValueError("CI-gate registry must declare 1 to 128 gates")
    owners: dict[str, str] = {}
    for gate in registry.gates:
        if not 1 <= len(gate.id) <= 80 or any(
            char not in "abcdefghijklmnopqrstuvwxyz0123456789-_" for char in gate.id
        ):
            raise ValueError(f"invalid CI-gate id {gate.id!r}")
        output = _output_name(gate.id)
        if output in owners:
            raise ValueError(
                f"gates {owners[output]!r} and {gate.id!r} share output name {output!r}"
            )
        owners[output] = gate.id
        if gate.scope not in ("", "job", "step"):
            raise ValueError(f"gate {gate.id!r} has invalid scope")
        if not gate.description.strip() or len(gate.description) > 240:
            raise ValueError(f"gate {gate.id!r} has invalid description")
        if not gate.packages and not gate.paths:
            raise ValueError(f"gate {gate.id!r} has neither packages nor paths")
        if len(gate.packages) > 4096 or len(gate.paths) > 4096:
            raise ValueError(f"gate {gate.id!r} has too many entries")
        if len(set(gate.packages)) != len(gate.packages):
            raise ValueError(f"gate {gate.id!r} repeats a package")
        if len(set(gate.paths)) != len(gate.paths):
            raise ValueError(f"gate {gate.id!r} repeats a path glob")
        for package in gate.packages:
            if not _valid_package(package):
                raise ValueError(f"gate {gate.id!r} has invalid package {package!r}")
        for pattern in gate.paths:
            _validate_glob(pattern)
    return tuple(sorted(registry.gates, key=lambda gate: gate.id))


def _valid_package(package: str) -> bool:
    if not 1 <= len(package) <= 240:
        return False
    parts = package.split("/")
    return len(parts) >= 2 and all(
        part
        and part[0] in "abcdefghijklmnopqrstuvwxyz0123456789"
        and all(char in "abcdefghijklmnopqrstuvwxyz0123456789._-" for char in part[1:])
        for part in parts
    )


def _validate_glob(pattern: str) -> None:
    if (
        not 1 <= len(pattern) <= 512
        or unicodedata.normalize("NFC", pattern) != pattern
        or pattern.startswith("/")
        or "\\" in pattern
        or "//" in pattern
        or (
            len(pattern) >= 2
            and pattern[0].isascii()
            and pattern[0].isalpha()
            and pattern[1] == ":"
        )
        or any(
            ord(char) < 32 or 0xD800 <= ord(char) <= 0xDFFF or char in '<>:"|?'
            for char in pattern
        )
    ):
        raise ValueError(f"invalid portable CI-gate glob {pattern!r}")
    for segment in pattern.split("/"):
        if (
            segment in ("", ".", "..")
            or segment.endswith((".", " "))
            or (
                not any(char in "*[]{}" for char in segment)
                and segment.split(".", 1)[0].upper() in _RESERVED
            )
        ):
            raise ValueError(f"invalid portable CI-gate glob {pattern!r}")
    try:
        validate_pattern(pattern)
    except ValueError as exc:
        raise ValueError(f"invalid portable CI-gate glob {pattern!r}") from exc


def _touches_machinery(changed: tuple[str, ...]) -> bool:
    return any(
        file in _MACHINERY_FILES
        or any(
            file == prefix or file.startswith(prefix) for prefix in _MACHINERY_PREFIXES
        )
        for file in changed
    )


def _segments(path: str) -> tuple[str, ...]:
    return tuple(segment for segment in path.rstrip("/").split("/") if segment)


def _literal_bounds(pattern: str) -> tuple[tuple[str, ...], tuple[str, ...], bool, int]:
    segments = _segments(pattern)
    wildcards = [
        index
        for index, segment in enumerate(segments)
        if any(char in "*?[]" for char in segment)
    ]
    if wildcards:
        prefix = segments[: wildcards[0]]
        suffix = segments[wildcards[-1] + 1 :]
        exact = False
    else:
        prefix, suffix, exact = segments, (), True
    work = sum(len(segment) + 1 for segment in (*prefix, *suffix))
    return prefix, suffix, exact, work


def _could_match(
    path: tuple[str, ...], prefix: tuple[str, ...], suffix: tuple[str, ...], exact: bool
) -> bool:
    if exact:
        return path == prefix
    return (
        len(path) >= len(prefix) + len(suffix)
        and path[: len(prefix)] == prefix
        and (not suffix or path[-len(suffix) :] == suffix)
    )


def _preflight(
    gates: tuple[Gate, ...], changed: tuple[str, ...]
) -> dict[str, tuple[str, ...]] | None:
    patterns = sorted({pattern for gate in gates for pattern in gate.paths})
    if not patterns:
        return {}
    # Every pair costs at least two units. Avoid a large intermediate table.
    if len(changed) > (MATCH_WORK_LIMIT // 2) // len(patterns):
        return None
    path_segments = tuple(_segments(file) for file in changed)
    remaining = MATCH_WORK_LIMIT
    candidates: dict[str, tuple[str, ...]] = {}
    for pattern in patterns:
        prefix, suffix, exact, literal_work = _literal_bounds(pattern)
        factor = len(pattern) + 1
        possible: list[str] = []
        for file, segments in zip(changed, path_segments, strict=True):
            if literal_work > remaining:
                return None
            remaining -= literal_work
            if not _could_match(segments, prefix, suffix, exact):
                continue
            file_factor = len(file) + 1
            if file_factor > remaining // factor:
                return None
            remaining -= factor * file_factor
            possible.append(file)
        candidates[pattern] = tuple(possible)
    return candidates
