"""
resolver.py -- Dependency Resolution from Package Metadata
==========================================================

This module reads package metadata files (pyproject.toml for Python, .gemspec
for Ruby, go.mod for Go, package.json for TypeScript, Cargo.toml for Rust,
Package.swift for Swift) and extracts internal dependencies. It builds a
directed graph where edges represent "A depends on B".

Dependency mapping conventions
------------------------------

Each language ecosystem uses a different naming convention for packages in this
monorepo:

- **Python**: Package names in pyproject.toml use the ``coding-adventures-``
  prefix with hyphens. For example, ``coding-adventures-logic-gates`` maps to
  the package ``python/logic-gates``.

- **Ruby**: Gem names in .gemspec use the ``coding_adventures_`` prefix with
  underscores. For example, ``coding_adventures_logic_gates`` maps to
  ``ruby/logic_gates``.

- **Go**: Module paths in go.mod include the repo path. We map module paths
  to ``go/X`` based on the last path component.

- **TypeScript**: package.json uses ``@coding-adventures/`` scoped npm names.
  ``@coding-adventures/logic-gates`` maps to ``typescript/logic-gates``.

- **Rust**: Cargo.toml uses path-based local dependencies.
  The crate name (key before ``=``) maps to ``rust/crate-name``.

- **Swift**: Package.swift uses ``.package(path: "../dep-name")`` relative
  path references. The directory name maps to ``swift/dep-name``.

External dependencies (those not matching the monorepo prefix) are silently
skipped.
"""

from __future__ import annotations

import re
from pathlib import Path

from build_tool.discovery import Package

# We import DirectedGraph at the type level. At runtime, we build our own
# lightweight graph since we don't want to require the directed-graph package
# as a hard dependency. Instead we ship a minimal DirectedGraph implementation
# inline for the build tool.


class DirectedGraph:
    """A minimal directed graph for dependency resolution.

    This is a stripped-down version of the full DirectedGraph in the
    directed-graph package. We only need node/edge storage, topological
    sort, independent groups, transitive closure, and transitive dependents.
    """

    def __init__(self) -> None:
        self._forward: dict[str, set[str]] = {}
        self._reverse: dict[str, set[str]] = {}

    def add_node(self, node: str) -> None:
        if node not in self._forward:
            self._forward[node] = set()
            self._reverse[node] = set()

    def add_edge(self, from_node: str, to_node: str) -> None:
        self.add_node(from_node)
        self.add_node(to_node)
        self._forward[from_node].add(to_node)
        self._reverse[to_node].add(from_node)

    def has_node(self, node: str) -> bool:
        return node in self._forward

    def nodes(self) -> list[str]:
        return list(self._forward.keys())

    def successors(self, node: str) -> list[str]:
        return list(self._forward.get(node, set()))

    def predecessors(self, node: str) -> list[str]:
        return list(self._reverse.get(node, set()))

    def transitive_closure(self, node: str) -> set[str]:
        """All nodes reachable from ``node`` (not including ``node`` itself)."""
        if node not in self._forward:
            return set()
        visited: set[str] = set()
        stack = list(self._forward[node])
        visited.update(stack)
        while stack:
            current = stack.pop()
            for successor in self._forward.get(current, set()):
                if successor not in visited:
                    visited.add(successor)
                    stack.append(successor)
        return visited

    def transitive_dependents(self, node: str) -> set[str]:
        """All nodes that transitively depend on ``node``."""
        if node not in self._reverse:
            return set()
        visited: set[str] = set()
        stack = list(self._reverse[node])
        visited.update(stack)
        while stack:
            current = stack.pop()
            for predecessor in self._reverse.get(current, set()):
                if predecessor not in visited:
                    visited.add(predecessor)
                    stack.append(predecessor)
        return visited

    def edges(self) -> list[tuple[str, str]]:
        """Return all directed edges as (from, to) tuples."""
        result: list[tuple[str, str]] = []
        for node, successors in self._forward.items():
            for succ in successors:
                result.append((node, succ))
        return result

    def affected_nodes(self, changed: set[str]) -> set[str]:
        """Return all nodes in ``changed`` plus all downstream packages.

        In this graph, edges flow dep -> pkg (a dependency must be built before
        the packages that use it). When a dep changes, every package that
        (transitively) depends on it also needs rebuilding — those are reachable
        via forward traversal (transitive_closure).

        Args:
            changed: Package names whose source files changed.

        Returns:
            The changed set plus all packages that transitively use them.
        """
        result: set[str] = set(changed)
        for name in changed:
            result |= self.transitive_closure(name)
        return result

    def independent_groups(self) -> list[list[str]]:
        """Partition nodes into parallel execution levels (Kahn's algorithm)."""
        in_degree: dict[str, int] = {
            node: len(preds) for node, preds in self._reverse.items()
        }
        current_level = sorted(
            node for node, degree in in_degree.items() if degree == 0
        )
        groups: list[list[str]] = []
        processed = 0

        while current_level:
            groups.append(current_level)
            processed += len(current_level)
            next_level_set: set[str] = set()
            for node in current_level:
                for successor in self._forward[node]:
                    in_degree[successor] -= 1
                    if in_degree[successor] == 0:
                        next_level_set.add(successor)
            current_level = sorted(next_level_set)

        if processed != len(self._forward):
            raise RuntimeError("Dependency graph contains a cycle")

        return groups


# ---------------------------------------------------------------------------
# Python dependency parsing
# ---------------------------------------------------------------------------

# We need a minimal TOML parser since we can't assume tomllib is available
# on all Python 3.12 installs (it is in stdlib from 3.11, but let's be safe
# and use it via the tomllib module which IS in 3.11+ stdlib).

try:
    import tomllib
except ImportError:
    # Python < 3.11 fallback (shouldn't happen with >=3.12 requirement)
    import tomli as tomllib  # type: ignore[no-redef]


def _parse_python_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Python package's pyproject.toml.

    Reads the ``[project] dependencies`` list and maps entries with the
    ``coding-adventures-`` prefix to their package names.

    Args:
        package: The Python package to inspect.
        known_names: Mapping from pypi-style name to package name.

    Returns:
        List of internal dependency package names.
    """
    pyproject = package.path / "pyproject.toml"
    if not pyproject.exists():
        return []

    with open(pyproject, "rb") as f:
        data = tomllib.load(f)

    deps_list = data.get("project", {}).get("dependencies", [])
    internal_deps: list[str] = []

    for dep_str in deps_list:
        # Strip version specifiers: "coding-adventures-logic-gates>=0.1" -> "coding-adventures-logic-gates"
        dep_name = re.split(r"[>=<!\s;]", dep_str)[0].strip().lower()
        if dep_name in known_names:
            internal_deps.append(known_names[dep_name])

    return internal_deps


# ---------------------------------------------------------------------------
# Ruby dependency parsing
# ---------------------------------------------------------------------------


def _parse_ruby_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Ruby package's .gemspec file.

    Looks for lines matching ``spec.add_dependency "coding_adventures_X"``
    and maps them to package names.
    """
    gemspec_files = list(package.path.glob("*.gemspec"))
    if not gemspec_files:
        return []

    gemspec = gemspec_files[0]
    text = gemspec.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    # Match: spec.add_dependency "coding_adventures_something"
    pattern = re.compile(r'spec\.add_dependency\s+"([^"]+)"')
    for match in pattern.finditer(text):
        gem_name = match.group(1).strip().lower()
        if gem_name in known_names:
            internal_deps.append(known_names[gem_name])

    return internal_deps


# ---------------------------------------------------------------------------
# Go dependency parsing
# ---------------------------------------------------------------------------


def _parse_go_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Go package's go.mod file.

    Looks for ``require`` lines and maps module paths to package names.
    """
    go_mod = package.path / "go.mod"
    if not go_mod.exists():
        return []

    text = go_mod.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    # Match require blocks and single require lines
    # Single: require github.com/user/repo/pkg v1.0.0
    # Block:  require (\n\tgithub.com/user/repo/pkg v1.0.0\n)
    in_require_block = False
    for line in text.splitlines():
        stripped = line.strip()

        if stripped == "require (":
            in_require_block = True
            continue
        if stripped == ")":
            in_require_block = False
            continue

        if in_require_block or stripped.startswith("require "):
            # Extract module path
            parts = stripped.replace("require ", "").strip().split()
            if parts:
                module_path = parts[0].lower()
                if module_path in known_names:
                    internal_deps.append(known_names[module_path])

    return internal_deps


# ---------------------------------------------------------------------------
# Elixir dependency parsing
# ---------------------------------------------------------------------------

def _parse_elixir_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from an Elixir mix.exs file."""
    mix_exs = package.path / "mix.exs"
    if not mix_exs.exists():
        return []

    text = mix_exs.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    pattern = re.compile(r'\{:(coding_adventures_[a-z0-9_]+)')
    for line in text.splitlines():
        for match in pattern.finditer(line):
            app_name = match.group(1).strip().lower()
            if app_name in known_names:
                internal_deps.append(known_names[app_name])

    return internal_deps


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


def _dependency_scope(language: str) -> str:
    """Map a package language to the dependency namespace it can resolve."""
    if language in {"csharp", "fsharp", "dotnet"}:
        return "dotnet"
    if language == "wasm":
        return "wasm"
    return language


def _in_dependency_scope(package_language: str, scope: str) -> bool:
    """Whether a package contributes names to the requested dependency scope."""
    if scope == "dotnet":
        return package_language in {"csharp", "fsharp", "dotnet"}
    if scope == "wasm":
        return package_language in {"wasm", "rust"}
    return package_language == scope


def _read_cargo_package_name(package: Package) -> str | None:
    """Read the Cargo package name from Cargo.toml when available."""
    cargo_toml = package.path / "Cargo.toml"
    if not cargo_toml.exists():
        return None

    text = cargo_toml.read_text(encoding="utf-8")
    match = re.search(r'(?m)^\s*name\s*=\s*"([^"]+)"', text)
    if not match:
        return None
    return match.group(1).strip().lower()


def _build_known_names(packages: list[Package], language: str = "") -> dict[str, str]:
    """Build a mapping from ecosystem-specific dependency names to package names.

    For Python:     "coding-adventures-logic-gates" -> "python/logic-gates"
    For Ruby:       "coding_adventures_logic_gates" -> "ruby/logic_gates"
    For Go:         module paths -> "go/module-name"
    For TypeScript: "@coding-adventures/logic-gates" -> "typescript/logic-gates"
    For Rust:       "logic-gates" (crate name) -> "rust/logic-gates"
    For Swift:      "logic-gates" (dir name) -> "swift/logic-gates"

    Library packages take priority over programs when the same ecosystem name
    maps to both. This prevents a program that depends on its own library from
    resolving the dep to itself and creating a self-loop.
    """
    known: dict[str, str] = {}

    def _set_known(key: str, value: str, pkg_path: Path) -> None:
        """Insert key→value, letting library packages overwrite programs."""
        if key not in known:
            known[key] = value
            return
        # Key already set. Allow overwrite only if the current pkg is a
        # library (not a program) — i.e., when the existing entry came from
        # a program and we now have the definitive library entry.
        if "/programs/" not in str(pkg_path).replace("\\", "/"):
            known[key] = value

    for pkg in packages:
        if language and not _in_dependency_scope(pkg.language, language):
            continue
        if pkg.language == "python":
            # Convert package dir name to pypi name: "logic-gates" -> "coding-adventures-logic-gates"
            pypi_name = f"coding-adventures-{pkg.path.name}".lower()
            _set_known(pypi_name, pkg.name, pkg.path)

        elif pkg.language == "ruby":
            # Convert package dir name to gem name: "logic_gates" -> "coding_adventures_logic_gates"
            gem_name = f"coding_adventures_{pkg.path.name}".lower()
            _set_known(gem_name, pkg.name, pkg.path)

        elif pkg.language == "go":
            # For Go, read the module path from go.mod. Go module paths are
            # unique across packages and programs, so the standard map write
            # is safe here.
            go_mod = pkg.path / "go.mod"
            if go_mod.exists():
                text = go_mod.read_text(encoding="utf-8")
                for line in text.splitlines():
                    if line.startswith("module "):
                        module_path = line.split(None, 1)[1].strip().lower()
                        known[module_path] = pkg.name
                        break

        elif pkg.language == "elixir":
            app_name = f"coding_adventures_{pkg.path.name.replace('-', '_')}".lower()
            known[app_name] = pkg.name

    return known


def resolve_dependencies(packages: list[Package]) -> DirectedGraph:
    """Parse package metadata to discover dependencies and build a graph.

    The graph contains all discovered packages as nodes. Edges represent
    "A depends on B" (A -> B means A needs B built first).

    External dependencies (not found among the discovered packages) are
    silently skipped.

    Args:
        packages: List of discovered packages.

    Returns:
        A DirectedGraph with dependency edges.
    """
    graph = DirectedGraph()

    # First, add all packages as nodes.
    for pkg in packages:
        graph.add_node(pkg.name)

    # Build dependency name mappings keyed by the scope each package language uses.
    known_names_by_scope: dict[str, dict[str, str]] = {}
    for pkg in packages:
        scope = _dependency_scope(pkg.language)
        if scope not in known_names_by_scope:
            known_names_by_scope[scope] = _build_known_names(packages, scope)

    # Parse dependencies for each package.
    for pkg in packages:
        known_names = known_names_by_scope[_dependency_scope(pkg.language)]
        if pkg.language == "python":
            deps = _parse_python_deps(pkg, known_names)
        elif pkg.language == "ruby":
            deps = _parse_ruby_deps(pkg, known_names)
        elif pkg.language == "go":
            deps = _parse_go_deps(pkg, known_names)
        elif pkg.language == "elixir":
            deps = _parse_elixir_deps(pkg, known_names)
        else:
            deps = []

        for dep_name in deps:
            # Edge direction: dep -> pkg means "dep must be built before pkg".
            # This makes independent_groups() produce the correct build order:
            # nodes with zero in-degree (no dependencies) come first.
            graph.add_edge(dep_name, pkg.name)

    return graph
