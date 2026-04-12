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
# Lua dependency parsing
# ---------------------------------------------------------------------------


def _parse_lua_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Lua package's .rockspec file.

    LuaRocks rockspec files declare dependencies in a Lua table::

        dependencies = {
            "lua >= 5.4",
            "coding-adventures-logic-gates >= 0.1.0",
        }

    We scan for quoted strings inside the ``dependencies`` block that start
    with ``coding-adventures-`` and map them to internal package names.

    Args:
        package: The Lua package to inspect.
        known_names: Mapping from rockspec-style name to package name.

    Returns:
        List of internal dependency package names.
    """
    rockspec_files = list(package.path.glob("*.rockspec"))
    if not rockspec_files:
        return []

    text = rockspec_files[0].read_text(encoding="utf-8")
    internal_deps: list[str] = []

    # Find the dependencies = { ... } block and extract quoted strings.
    in_deps = False
    for line in text.splitlines():
        stripped = line.strip()

        if not in_deps:
            if "dependencies" in stripped and "=" in stripped and "{" in stripped:
                in_deps = True
                # Single-line case: dependencies = { "foo", "bar" }
                if "}" in stripped:
                    _extract_lua_deps(stripped, known_names, internal_deps)
                    break
                _extract_lua_deps(stripped, known_names, internal_deps)
            continue

        # Inside the dependencies block.
        if "}" in stripped:
            _extract_lua_deps(stripped, known_names, internal_deps)
            break
        _extract_lua_deps(stripped, known_names, internal_deps)

    return internal_deps


def _extract_lua_deps(
    line: str, known_names: dict[str, str], deps: list[str]
) -> None:
    """Extract Lua dependency names from a line, stripping version specifiers."""
    for match in re.finditer(r'"([^"]+)"', line):
        dep_str = match.group(1)
        # Strip version specifiers: "coding-adventures-foo >= 0.1" -> "coding-adventures-foo"
        dep_name = re.split(r"[>=<!\s~]", dep_str)[0].strip().lower()
        if dep_name in known_names:
            deps.append(known_names[dep_name])


# ---------------------------------------------------------------------------
# TypeScript dependency parsing
# ---------------------------------------------------------------------------


def _parse_typescript_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a TypeScript package's package.json.

    TypeScript packages declare dependencies in package.json::

        "dependencies": {
            "@coding-adventures/logic-gates": "file:../logic-gates"
        }

    We scan both ``dependencies`` and ``devDependencies`` blocks for keys
    matching the ``@coding-adventures/`` prefix (or bare name fallback) and
    map them to internal package names.

    Args:
        package: The TypeScript package to inspect.
        known_names: Mapping from npm name to package name.

    Returns:
        List of internal dependency package names.
    """
    package_json = package.path / "package.json"
    if not package_json.exists():
        return []

    text = package_json.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    in_deps = False
    key_re = re.compile(r'"([^"]+)"\s*:')
    for line in text.splitlines():
        stripped = line.strip()

        if not in_deps:
            if ('"dependencies"' in stripped or '"devDependencies"' in stripped) and "{" in stripped:
                in_deps = True
            continue

        if "}" in stripped:
            in_deps = False
            continue

        for match in key_re.finditer(stripped):
            dep_name = match.group(1).strip().lower()
            if dep_name in known_names:
                internal_deps.append(known_names[dep_name])

    return internal_deps


# ---------------------------------------------------------------------------
# Rust dependency parsing
# ---------------------------------------------------------------------------


def _parse_rust_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Rust package's Cargo.toml.

    Rust Cargo.toml declares workspace-local dependencies with path references::

        [dependencies]
        logic-gates = { path = "../logic-gates" }

    We look for lines in the ``[dependencies]`` section that contain
    ``path =`` and extract the crate name (the key before ``=``). We then
    look up that name in the known names mapping.

    Args:
        package: The Rust package to inspect.
        known_names: Mapping from crate name to package name.

    Returns:
        List of internal dependency package names.
    """
    cargo_toml = package.path / "Cargo.toml"
    if not cargo_toml.exists():
        return []

    text = cargo_toml.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    in_deps = False
    for line in text.splitlines():
        stripped = line.strip()

        # Detect section headers like [dependencies] or [dev-dependencies].
        if stripped.startswith("["):
            in_deps = stripped == "[dependencies]"
            continue

        if not in_deps:
            continue

        # Look for lines like: logic-gates = { path = "../logic-gates" }
        if "path" in stripped and "=" in stripped:
            parts = stripped.split("=", 1)
            if len(parts) >= 2:
                crate_name = parts[0].strip().lower()
                if crate_name in known_names:
                    internal_deps.append(known_names[crate_name])

    return internal_deps


def _parse_dotnet_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from .NET project references."""
    project_files = list(package.path.glob("*.csproj")) + list(package.path.glob("*.fsproj"))
    if not project_files:
        return []

    internal_deps: list[str] = []
    pattern = re.compile(r'<ProjectReference\s+Include\s*=\s*"\.\.[\\/]+([^/\\"]+)[\\/][^"]*"')

    for project_file in project_files:
        text = project_file.read_text(encoding="utf-8")
        for match in pattern.finditer(text):
            dep_dir = match.group(1).strip().lower()
            if "/" in dep_dir or "\\" in dep_dir or dep_dir == "..":
                continue
            if dep_dir in known_names:
                internal_deps.append(known_names[dep_dir])

    return internal_deps


# ---------------------------------------------------------------------------
# Swift dependency parsing
# ---------------------------------------------------------------------------

# Matches: .package(path: "../dep-name")
_SWIFT_DEP_RE = re.compile(r'\.package\s*\(\s*path\s*:\s*"\.\./([^"]+)"')


def _parse_swift_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Swift Package.swift file.

    Swift Package Manager uses relative path references for local (monorepo)
    dependencies. The declaration always appears on a single line::

        .package(path: "../logic-gates"),

    We scan for this pattern and map the directory name back to our internal
    package name. External dependencies (declared with ``url:``) are silently
    skipped because they don't match the ``path: "../"`` prefix.

    Args:
        package: The Swift package to inspect.
        known_names: Mapping from directory name to package name.

    Returns:
        List of internal dependency package names.
    """
    manifest = package.path / "Package.swift"
    if not manifest.exists():
        return []

    text = manifest.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        match = _SWIFT_DEP_RE.search(stripped)
        if match:
            dep_dir = match.group(1).lower()
            # Guard against path traversal: reject any segment containing
            # a path separator or additional ".." components.
            if "/" in dep_dir or "\\" in dep_dir or dep_dir == "..":
                continue
            if dep_dir in known_names:
                internal_deps.append(known_names[dep_dir])

    return internal_deps


# ---------------------------------------------------------------------------
# Perl dependency parsing
# ---------------------------------------------------------------------------


def _parse_perl_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Perl package's cpanfile.

    A cpanfile declares dependencies with one ``requires`` per line::

        requires 'coding-adventures-logic-gates';
        requires 'coding-adventures-bitset', '>= 0.01';

        on 'test' => sub {
            requires 'Test2::V0';
        };

    We scan for lines matching ``requires 'coding-adventures-...'`` and map
    them to internal package names. External deps are silently skipped.

    Args:
        package: The Perl package to inspect.
        known_names: Mapping from CPAN dist name to package name.

    Returns:
        List of internal package names this package depends on.
    """
    cpanfile = package.path / "cpanfile"
    if not cpanfile.exists():
        return []

    text = cpanfile.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    pattern = re.compile(r"""requires\s+['"](coding-adventures-[^'"]+)['"]""")

    for line in text.splitlines():
        stripped = line.strip()
        # Skip blank lines and comments.
        if not stripped or stripped.startswith("#"):
            continue

        match = pattern.search(stripped)
        if match:
            dep_name = match.group(1).lower()
            if dep_name in known_names:
                internal_deps.append(known_names[dep_name])

    return internal_deps


# ---------------------------------------------------------------------------
# Haskell dependency parsing
# ---------------------------------------------------------------------------


def _parse_haskell_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Haskell package's .cabal file."""
    cabal_files = list(package.path.glob("*.cabal"))
    if not cabal_files:
        return []

    text = cabal_files[0].read_text(encoding="utf-8")
    internal_deps: list[str] = []

    pattern = re.compile(r"coding-adventures-([a-z0-9-]+)")
    for match in pattern.finditer(text):
        dep_name = f"coding-adventures-{match.group(1).lower()}"
        if dep_name in known_names and known_names[dep_name] != package.name:
            internal_deps.append(known_names[dep_name])

    return internal_deps


# ---------------------------------------------------------------------------
# Gradle (Java / Kotlin) dependency parsing
# ---------------------------------------------------------------------------

# Regex for Gradle composite build includes: includeBuild("../logic-gates")
_GRADLE_INCLUDE_BUILD_RE = re.compile(r'includeBuild\s*\(\s*"\.\.\/([^"]+)"\s*\)')


def _parse_gradle_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Extract internal dependencies from a Gradle settings.gradle.kts file.

    Both Java and Kotlin packages use Gradle as their build system. In this
    monorepo, sibling package dependencies are declared as composite builds
    in ``settings.gradle.kts``::

        includeBuild("../logic-gates")
        includeBuild("../transistors")

    We scan for ``includeBuild("../...")`` entries and map the directory name
    back to our internal package name. Only ``"../"`` prefixed entries are
    considered (local monorepo siblings).

    Args:
        package: The Java or Kotlin package to inspect.
        known_names: Mapping from directory name to package name.

    Returns:
        List of internal dependency package names.
    """
    settings_file = package.path / "settings.gradle.kts"
    if not settings_file.exists():
        return []

    text = settings_file.read_text(encoding="utf-8")
    internal_deps: list[str] = []

    for line in text.splitlines():
        stripped = line.strip()
        if not stripped or stripped.startswith("//"):
            continue
        match = _GRADLE_INCLUDE_BUILD_RE.search(stripped)
        if match:
            dep_dir = match.group(1).lower()
            # Guard against path traversal.
            if "/" in dep_dir or "\\" in dep_dir or dep_dir == "..":
                continue
            if dep_dir in known_names:
                internal_deps.append(known_names[dep_dir])

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

        elif pkg.language == "typescript":
            # Convert dir name to npm scoped name: "logic-gates" -> "@coding-adventures/logic-gates"
            npm_name = f"@coding-adventures/{pkg.path.name}".lower()
            _set_known(npm_name, pkg.name, pkg.path)
            _set_known(pkg.path.name.lower(), pkg.name, pkg.path)

            # Also read the actual "name" field from package.json for accuracy.
            package_json = pkg.path / "package.json"
            if package_json.exists():
                name_match = re.search(r'"name"\s*:\s*"([^"]+)"', package_json.read_text(encoding="utf-8"))
                if name_match:
                    _set_known(name_match.group(1).strip().lower(), pkg.name, pkg.path)

        elif pkg.language in ("rust", "wasm"):
            # Rust crate names use the directory name directly (kebab-case).
            crate_name = pkg.path.name.lower()
            _set_known(crate_name, pkg.name, pkg.path)
            cargo_name = _read_cargo_package_name(pkg)
            if cargo_name is not None:
                _set_known(cargo_name, pkg.name, pkg.path)

        elif pkg.language == "elixir":
            # Elixir mix names replace hyphens with underscores.
            base_name = pkg.path.name.replace("-", "_").lower()
            app_name = f"coding_adventures_{base_name}"
            _set_known(app_name, pkg.name, pkg.path)
            _set_known(base_name, pkg.name, pkg.path)

            # Also read the actual app name from mix.exs for accuracy.
            mix_exs = pkg.path / "mix.exs"
            if mix_exs.exists():
                app_match = re.search(r"app:\s*:([a-z0-9_]+)", mix_exs.read_text(encoding="utf-8"))
                if app_match:
                    _set_known(app_match.group(1).strip().lower(), pkg.name, pkg.path)

        elif pkg.language == "lua":
            # Lua rockspec names use hyphens: "logic_gates" dir → "coding-adventures-logic-gates"
            rockspec_name = f"coding-adventures-{pkg.path.name.replace('_', '-')}".lower()
            _set_known(rockspec_name, pkg.name, pkg.path)

        elif pkg.language == "perl":
            # Perl CPAN dist names use hyphens: "logic-gates" → "coding-adventures-logic-gates"
            # This matches the Python convention exactly.
            cpan_name = f"coding-adventures-{pkg.path.name}".lower()
            _set_known(cpan_name, pkg.name, pkg.path)

        elif pkg.language == "swift":
            # Swift SPM package names are the kebab-case directory name.
            # .package(path: "../logic-gates") references the directory name directly.
            dir_base = pkg.path.name.lower()
            _set_known(dir_base, pkg.name, pkg.path)

        elif pkg.language == "haskell":
            # Haskell Cabal package names use hyphens: "logic-gates" → "coding-adventures-logic-gates"
            cabal_name = f"coding-adventures-{pkg.path.name}".lower()
            _set_known(cabal_name, pkg.name, pkg.path)

        elif pkg.language in ("java", "kotlin", "csharp", "fsharp", "dotnet"):
            # Java and Kotlin packages use Gradle composite builds. Dependencies
            # are referenced by directory name in settings.gradle.kts via
            # includeBuild("../dep-name"). .NET packages likewise use sibling
            # directory names in ProjectReference paths. The directory name maps directly.
            dir_base = pkg.path.name.lower()
            _set_known(dir_base, pkg.name, pkg.path)

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
        elif pkg.language == "typescript":
            deps = _parse_typescript_deps(pkg, known_names)
        elif pkg.language in {"rust", "wasm"}:
            deps = _parse_rust_deps(pkg, known_names)
        elif pkg.language == "elixir":
            deps = _parse_elixir_deps(pkg, known_names)
        elif pkg.language == "lua":
            deps = _parse_lua_deps(pkg, known_names)
        elif pkg.language == "perl":
            deps = _parse_perl_deps(pkg, known_names)
        elif pkg.language == "swift":
            deps = _parse_swift_deps(pkg, known_names)
        elif pkg.language == "haskell":
            deps = _parse_haskell_deps(pkg, known_names)
        elif pkg.language in ("java", "kotlin"):
            deps = _parse_gradle_deps(pkg, known_names)
        elif pkg.language in {"csharp", "fsharp", "dotnet"}:
            deps = _parse_dotnet_deps(pkg, known_names)
        else:
            deps = []

        for dep_name in deps:
            # Edge direction: dep -> pkg means "dep must be built before pkg".
            # This makes independent_groups() produce the correct build order:
            # nodes with zero in-degree (no dependencies) come first.
            graph.add_edge(dep_name, pkg.name)

    return graph
