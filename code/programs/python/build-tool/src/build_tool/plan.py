"""
plan.py -- Build Plan Serialization and Deserialization
========================================================

A build plan captures the results of the build tool's discovery, dependency
resolution, and change detection steps as a JSON file. This enables CI to
compute the plan once in a fast "detect" job and share it across build jobs
on multiple platforms -- eliminating redundant computation.

Why build plans?
----------------

In a CI pipeline, the build tool runs three expensive steps before building:

1. **Discovery** -- walk the filesystem to find all BUILD files.
2. **Resolution** -- parse dependency metadata (pyproject.toml, go.mod, etc.).
3. **Change detection** -- run ``git diff`` to find affected packages.

These steps are identical across platforms (macOS, Linux, Windows). Running
them once and sharing the result via a JSON plan file saves CI minutes.

Schema versioning
-----------------

The plan uses a simple integer version scheme (``schema_version`` field).
Readers MUST reject plans with a version higher than what they support,
falling back to the normal discovery flow. Writers always stamp the current
version.

This approach is intentionally conservative: if a newer build tool writes a
plan with features the current tool doesn't understand, we'd rather re-do
discovery than silently ignore new fields.

Path conventions
----------------

All paths in the plan use forward slashes (``/``) regardless of platform.
On write, we normalize paths to forward slashes. On read, consumers can
convert back to platform-native separators if needed.

JSON structure
--------------

The plan file looks like this::

    {
      "schema_version": 1,
      "diff_base": "origin/main",
      "force": false,
      "affected_packages": ["python/logic-gates", "python/vm"],
      "packages": [
        {
          "name": "python/logic-gates",
          "rel_path": "code/packages/python/logic-gates",
          "language": "python",
          "build_commands": ["uv pip install -e .", "pytest"],
          "is_starlark": true,
          "declared_srcs": ["src/**/*.py"],
          "declared_deps": []
        }
      ],
      "dependency_edges": [["python/logic-gates", "python/vm"]],
      "languages_needed": {"python": true, "go": false}
    }

Semantics of ``affected_packages``:

- ``null`` (JSON null / Python ``None``): rebuild all packages. This happens
  in ``--force`` mode or when git diff is unavailable.
- ``[]`` (empty list): nothing changed, build nothing.
- ``["a", "b"]`` (non-empty list): only these packages need building.
"""

from __future__ import annotations

import contextlib
import json
from dataclasses import dataclass, field
from pathlib import Path

# The schema version that this implementation reads and writes.
# Plans with a higher version are rejected -- we'd rather fall back to
# normal discovery than silently ignore unknown fields.
CURRENT_SCHEMA_VERSION = 1


# =========================================================================
# Data Classes
# =========================================================================
#
# These mirror the Go implementation's structs in internal/plan/plan.go.
# The field names and JSON keys are identical for interoperability.


@dataclass
class PackageEntry:
    """A single package in the build plan.

    Attributes:
        name: Qualified package name, e.g. "python/logic-gates".
        rel_path: Repo-root-relative path, always using forward slashes.
        language: The package's programming language.
        build_commands: Shell commands to execute for building/testing.
        is_starlark: Whether the BUILD file uses Starlark syntax.
        declared_srcs: Glob patterns from the Starlark srcs field.
        declared_deps: Qualified names from the Starlark deps field.
    """

    name: str
    rel_path: str
    language: str
    build_commands: list[str]
    is_starlark: bool = False
    declared_srcs: list[str] = field(default_factory=list)
    declared_deps: list[str] = field(default_factory=list)


@dataclass
class BuildPlan:
    """The top-level structure serialized to JSON.

    Attributes:
        schema_version: Identifies the plan format.
        diff_base: Git ref used for change detection (informational).
        force: Whether ``--force`` was set.
        affected_packages: Packages needing building, or None for "all".
        packages: ALL discovered packages (not just affected ones).
        dependency_edges: Directed edges ``(from, to)`` where from must
            be built before to.
        languages_needed: Map of language name -> whether its toolchain
            is needed for this build.
    """

    schema_version: int
    diff_base: str
    force: bool
    affected_packages: list[str] | None
    packages: list[PackageEntry]
    dependency_edges: list[tuple[str, str]]
    languages_needed: dict[str, bool]


# =========================================================================
# Serialization
# =========================================================================


def write_plan(bp: BuildPlan, path: str) -> None:
    """Serialize a build plan to a JSON file.

    Always stamps ``schema_version`` to ``CURRENT_SCHEMA_VERSION``
    regardless of what the caller set. This ensures we never accidentally
    write an old version number with new-format data.

    Uses an atomic write strategy: write to a temporary file first, then
    rename. This prevents partial writes if the process is killed mid-write.

    Parameters
    ----------
    bp : BuildPlan
        The plan to serialize.
    path : str
        File path to write to.
    """
    # Force the schema version to current.
    bp.schema_version = CURRENT_SCHEMA_VERSION

    # Build the JSON-serializable dict.
    #
    # We construct this manually rather than using dataclasses.asdict()
    # because we need special handling for affected_packages (None vs [])
    # and dependency_edges (list of tuples -> list of lists).
    data: dict = {
        "schema_version": bp.schema_version,
        "diff_base": bp.diff_base,
        "force": bp.force,
        # None -> JSON null, [] -> JSON [], ["a"] -> JSON ["a"]
        "affected_packages": bp.affected_packages,
        "packages": [
            {
                "name": pe.name,
                "rel_path": pe.rel_path,
                "language": pe.language,
                "build_commands": pe.build_commands,
                "is_starlark": pe.is_starlark,
                "declared_srcs": pe.declared_srcs,
                "declared_deps": pe.declared_deps,
            }
            for pe in bp.packages
        ],
        # Convert tuples to lists for JSON serialization.
        "dependency_edges": [list(edge) for edge in bp.dependency_edges],
        "languages_needed": bp.languages_needed,
    }

    json_str = json.dumps(data, indent=2)

    # Atomic write: write to temp file, then rename.
    tmp_path = path + ".tmp"
    try:
        Path(tmp_path).write_text(json_str, encoding="utf-8")
        Path(tmp_path).rename(path)
    except Exception:
        # Clean up temp file on failure.
        with contextlib.suppress(OSError):
            Path(tmp_path).unlink(missing_ok=True)
        raise


# =========================================================================
# Deserialization
# =========================================================================


def read_plan(path: str) -> BuildPlan:
    """Deserialize a build plan from a JSON file.

    Rejects plans with ``schema_version`` higher than ``CURRENT_SCHEMA_VERSION``.
    This is the forward-compatibility safety valve: if a newer tool writes a
    plan with features we don't understand, we reject it so the caller can
    fall back to normal discovery.

    Parameters
    ----------
    path : str
        File path to read from.

    Returns
    -------
    BuildPlan
        The deserialized plan.

    Raises
    ------
    FileNotFoundError
        If the file does not exist.
    ValueError
        If the JSON is invalid or the schema version is too high.
    """
    try:
        text = Path(path).read_text(encoding="utf-8")
    except FileNotFoundError:
        raise FileNotFoundError(f"Build plan not found: {path}") from None

    try:
        data = json.loads(text)
    except json.JSONDecodeError as exc:
        raise ValueError(f"Invalid JSON in build plan {path}: {exc}") from exc

    # Check schema version BEFORE parsing the rest. If the version is
    # too high, we don't even try to interpret the fields -- they might
    # have different semantics in a newer schema.
    version = data.get("schema_version", 0)
    if version > CURRENT_SCHEMA_VERSION:
        raise ValueError(
            f"Unsupported build plan version {version} "
            f"(this tool supports up to {CURRENT_SCHEMA_VERSION})"
        )

    # Parse packages.
    packages: list[PackageEntry] = []
    for raw_pkg in data.get("packages", []):
        packages.append(
            PackageEntry(
                name=raw_pkg.get("name", ""),
                rel_path=raw_pkg.get("rel_path", ""),
                language=raw_pkg.get("language", ""),
                build_commands=raw_pkg.get("build_commands", []),
                is_starlark=raw_pkg.get("is_starlark", False),
                declared_srcs=raw_pkg.get("declared_srcs", []),
                declared_deps=raw_pkg.get("declared_deps", []),
            )
        )

    # Parse dependency edges. In JSON they're [from, to] arrays.
    edges: list[tuple[str, str]] = []
    for raw_edge in data.get("dependency_edges", []):
        if isinstance(raw_edge, list) and len(raw_edge) == 2:
            edges.append((raw_edge[0], raw_edge[1]))

    # Parse affected_packages. null -> None, [] -> [], [...] -> [...].
    raw_affected = data.get("affected_packages")
    affected: list[str] | None = None
    if raw_affected is not None:
        affected = list(raw_affected)

    return BuildPlan(
        schema_version=version,
        diff_base=data.get("diff_base", ""),
        force=data.get("force", False),
        affected_packages=affected,
        packages=packages,
        dependency_edges=edges,
        languages_needed=data.get("languages_needed", {}),
    )
