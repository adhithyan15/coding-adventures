"""
resolver.py -- Dependency Resolution from Package Metadata
==========================================================

This module reads package metadata files (pyproject.toml for Python, .gemspec
for Ruby, go.mod for Go, package.json for TypeScript, Cargo.toml for Rust,
Package.swift for Swift, pubspec.yaml for Dart, and root project files for C#
and F#) and extracts internal dependencies. It builds a directed graph where
edges represent "A depends on B".

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

import os
import re
from pathlib import Path

from build_tool.discovery import Package

# We import DirectedGraph at the type level. At runtime, we build our own
# lightweight graph since we don't want to require the directed-graph package
# as a hard dependency. Instead we ship a minimal DirectedGraph implementation
# inline for the build tool.


class MetadataEncodingError(ValueError):
    """A dependency manifest is not valid UTF-8 metadata."""

    code = "METADATA_INVALID_UTF8"

    def __init__(self, package: str, manifest: Path) -> None:
        self.package = package
        self.manifest = manifest.name
        code_index = next(
            (
                index
                for index in range(len(manifest.parts) - 2, -1, -1)
                if manifest.parts[index] == "code"
                and manifest.parts[index + 1] in {"packages", "programs"}
            ),
            None,
        )
        if code_index is None:
            self.path = f"{package}/{manifest.name}"
        else:
            self.path = "/".join(manifest.parts[code_index:])
        super().__init__(
            f"{self.code}: {self.path} for {package} must be encoded as UTF-8"
        )


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
# Dart dependency parsing
# ---------------------------------------------------------------------------


_DART_PACKAGE_IDENTIFIER = re.compile(r"^[a-z][a-z0-9_]*$")


def _read_dart_package_name(package_path: Path) -> str | None:
    """Return one unquoted root ``name`` from a Dart pubspec."""
    pubspec = package_path / "pubspec.yaml"
    if not pubspec.exists():
        return None
    match = re.search(
        r"(?m)^name\s*:\s*([a-z0-9_]+)\s*$",
        pubspec.read_text(encoding="utf-8"),
    )
    return match.group(1).lower() if match else None


def _parse_dart_deps(
    package: Package, known_names: dict[str, str]
) -> list[str]:
    """Read direct dependency keys from the two root pubspec maps.

    A dependency value may be a scalar constraint or a nested source map. The
    nested map is deliberately opaque: following ``path`` or reading target
    manifests would turn metadata resolution into filesystem authority and
    would misclassify source-option keys as packages.
    """
    pubspec = package.path / "pubspec.yaml"
    if not pubspec.exists():
        return []

    dependencies: set[str] = set()
    in_dependency_map = False
    direct_entry_indent: int | None = None

    for raw_line in pubspec.read_text(encoding="utf-8").splitlines():
        stripped = raw_line.strip()
        if not stripped or stripped.startswith("#"):
            continue

        indent = len(raw_line) - len(raw_line.lstrip(" "))
        if indent == 0:
            in_dependency_map = stripped in {
                "dependencies:",
                "dev_dependencies:",
            }
            direct_entry_indent = None
            continue

        if not in_dependency_map:
            continue
        if direct_entry_indent is None:
            direct_entry_indent = indent
        if indent != direct_entry_indent:
            continue

        dependency_name, separator, _ = stripped.partition(":")
        dependency_name = dependency_name.strip().lower()
        if not separator or not _DART_PACKAGE_IDENTIFIER.fullmatch(dependency_name):
            continue
        dependency = known_names.get(dependency_name)
        if dependency is not None and dependency != package.name:
            dependencies.add(dependency)

    return sorted(dependencies)


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

    rockspec = rockspec_files[0]
    try:
        text = rockspec.read_text(encoding="utf-8")
    except UnicodeDecodeError as exc:
        raise MetadataEncodingError(package.name, rockspec) from exc
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


def _find_cabal_file(package_path: Path) -> Path | None:
    """Return the sole root Cabal manifest, rejecting ambiguous packages."""
    try:
        manifests = sorted(
            path
            for path in package_path.iterdir()
            if path.is_file() and path.suffix.lower() == ".cabal"
        )
    except OSError:
        return None
    return manifests[0] if len(manifests) == 1 else None


def _read_cabal_package_name(package_path: Path) -> str | None:
    """Read the declared name from an unambiguous root Cabal manifest."""
    manifest = _find_cabal_file(package_path)
    if manifest is None:
        return None
    match = re.search(
        r"(?mi)^\s*name\s*:\s*([a-z0-9][a-z0-9-]*)\s*$",
        manifest.read_text(encoding="utf-8"),
    )
    return match.group(1).lower() if match else None


def _parse_haskell_deps(package: Package, known_names: dict[str, str]) -> list[str]:
    """Read only ``build-depends`` fields from one root Cabal manifest."""
    manifest = _find_cabal_file(package.path)
    if manifest is None:
        return []

    name_pattern = re.compile(r"^([a-z0-9][a-z0-9-]*)", re.IGNORECASE)
    field_pattern = re.compile(r"^[a-z][a-z0-9-]*\s*:", re.IGNORECASE)
    internal_deps: set[str] = set()
    in_build_depends = False

    for raw_line in manifest.read_text(encoding="utf-8").splitlines():
        line = raw_line.split("--", 1)[0].strip()
        if line.lower().startswith("build-depends:"):
            in_build_depends = True
            line = line[len("build-depends:") :].strip()
        elif in_build_depends and (
            not line
            or field_pattern.match(line)
            or (raw_line and raw_line[0] not in " \t")
        ):
            in_build_depends = False

        if not in_build_depends:
            continue
        for piece in line.split(","):
            match = name_pattern.match(piece.strip())
            if not match:
                continue
            dependency = known_names.get(match.group(1).lower())
            if dependency is not None and dependency != package.name:
                internal_deps.add(dependency)

    return sorted(internal_deps)


# ---------------------------------------------------------------------------
# Gradle (Java / Kotlin) dependency parsing
# ---------------------------------------------------------------------------
def _skip_gradle_string(source: str | list[str], index: int) -> int:
    """Return the first character after one double-quoted Kotlin string."""
    index += 1
    while index < len(source):
        if source[index] == "\\":
            index += 2
        elif source[index] == '"':
            return index + 1
        else:
            index += 1
    return index


def _strip_gradle_comments(source: str) -> str:
    """Blank nested block and line comments while preserving offsets."""
    visible = list(source)
    block_depth = 0
    index = 0
    while index < len(visible):
        pair = "".join(visible[index : index + 2])
        if block_depth:
            if pair == "/*":
                visible[index : index + 2] = [" ", " "]
                block_depth += 1
                index += 2
            elif pair == "*/":
                visible[index : index + 2] = [" ", " "]
                block_depth -= 1
                index += 2
            else:
                if visible[index] not in "\r\n":
                    visible[index] = " "
                index += 1
            continue

        if visible[index] == '"':
            index = _skip_gradle_string(visible, index)
        elif pair == "//":
            while index < len(visible) and visible[index] not in "\r\n":
                visible[index] = " "
                index += 1
        elif pair == "/*":
            visible[index : index + 2] = [" ", " "]
            block_depth = 1
            index += 2
        else:
            index += 1
    return "".join(visible)


def _skip_gradle_whitespace(source: str, index: int) -> int:
    while index < len(source) and source[index] in " \t\r\n":
        index += 1
    return index


def _gradle_identifier_at(source: str, index: int, identifier: str) -> bool:
    if not source.startswith(identifier, index):
        return False
    identifier_chars = "_abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
    if index and source[index - 1] in identifier_chars:
        return False
    end = index + len(identifier)
    return end == len(source) or source[end] not in identifier_chars


def _parse_gradle_include_build(
    source: str, index: int
) -> tuple[str | None, int]:
    index = _skip_gradle_whitespace(source, index)
    if index >= len(source) or source[index] != "(":
        return None, index
    index = _skip_gradle_whitespace(source, index + 1)
    if index >= len(source) or source[index] != '"':
        return None, index

    start = index + 1
    index = start
    while index < len(source):
        if source[index] == "\\":
            index += 2
            continue
        if source[index] != '"':
            index += 1
            continue
        path = source[start:index]
        next_index = _skip_gradle_whitespace(source, index + 1)
        if next_index >= len(source) or source[next_index] != ")":
            return None, next_index
        return path, next_index + 1
    return None, index


def _gradle_include_build_paths(source: str) -> list[str]:
    visible = _strip_gradle_comments(source)
    paths: list[str] = []
    index = 0
    while index < len(visible):
        if visible[index] == '"':
            index = _skip_gradle_string(visible, index)
            continue
        if _gradle_identifier_at(visible, index, "includeBuild"):
            path, next_index = _parse_gradle_include_build(
                visible, index + len("includeBuild")
            )
            if path is not None:
                paths.append(path)
                index = next_index
                continue
        index += 1
    return paths


def _portable_path_is_absolute(path: str) -> bool:
    return bool(
        path
        and (
            path[0] in "/\\"
            or (len(path) >= 2 and path[0].isalpha() and path[1] == ":")
        )
    )


def _normalized_gradle_package_path(path: Path | str) -> str:
    return os.path.normcase(os.path.normpath(str(path))).lower()


def _build_known_gradle_paths_for_language(
    packages: list[Package], language: str
) -> dict[str, str]:
    return {
        _normalized_gradle_package_path(package.path): package.name
        for package in packages
        if package.language == language
    }


def _parse_gradle_deps(package: Package, known_paths: dict[str, str]) -> list[str]:
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
        known_paths: Mapping from normalized discovered roots to package names.

    Returns:
        List of internal dependency package names.
    """
    settings_file = package.path / "settings.gradle.kts"
    if not settings_file.exists():
        return []

    text = settings_file.read_text(encoding="utf-8")
    internal_deps: set[str] = set()
    for relative_path in _gradle_include_build_paths(text):
        if (
            not relative_path
            or "\\" in relative_path
            or "$" in relative_path
            or _portable_path_is_absolute(relative_path)
        ):
            continue
        target = _normalized_gradle_package_path(
            os.path.join(package.path, relative_path.replace("/", os.sep))
        )
        dependency = known_paths.get(target)
        if dependency is not None and dependency != package.name:
            internal_deps.add(dependency)

    return sorted(internal_deps)


# ---------------------------------------------------------------------------
# .NET (C# / F#) dependency parsing
# ---------------------------------------------------------------------------


def _root_dotnet_project_files(root: Path) -> list[Path]:
    """Return only C# and F# project files directly inside a package root."""
    try:
        return sorted(
            (
                entry
                for entry in root.iterdir()
                if entry.is_file() and entry.suffix.lower() in {".csproj", ".fsproj"}
            ),
            key=lambda path: path.name.lower(),
        )
    except OSError:
        return []


def _skip_xml_markup(source: str, index: int, terminator: str) -> int:
    relative = source.find(terminator, index)
    if relative < 0:
        return len(source)
    return relative + len(terminator)


def _is_xml_name_character(character: str) -> bool:
    return character.isascii() and (character.isalnum() or character in ":_-.")


def _parse_xml_start_tag(source: str, index: int) -> tuple[str | None, str, int]:
    if (
        index >= len(source)
        or source[index] != "<"
        or index + 1 >= len(source)
        or source[index + 1] == "/"
    ):
        return None, "", index

    name_start = index + 1
    name_end = name_start
    while name_end < len(source) and _is_xml_name_character(source[name_end]):
        name_end += 1
    if name_end == name_start:
        return None, "", index

    quote: str | None = None
    end = name_end
    while end < len(source):
        character = source[end]
        if quote is not None:
            if character == quote:
                quote = None
        elif character in {"'", '"'}:
            quote = character
        elif character == ">":
            return source[name_start:name_end], source[name_end:end], end + 1
        end += 1
    return None, "", len(source)


def _skip_xml_whitespace(source: str, index: int) -> int:
    while index < len(source) and source[index] in " \t\r\n":
        index += 1
    return index


def _xml_literal_attribute(attributes: str, wanted: str) -> str | None:
    index = 0
    while index < len(attributes):
        index = _skip_xml_whitespace(attributes, index)
        if index >= len(attributes) or attributes[index] == "/":
            return None

        name_start = index
        while index < len(attributes) and _is_xml_name_character(attributes[index]):
            index += 1
        if index == name_start:
            index += 1
            continue
        name = attributes[name_start:index]

        index = _skip_xml_whitespace(attributes, index)
        if index >= len(attributes) or attributes[index] != "=":
            continue
        index = _skip_xml_whitespace(attributes, index + 1)
        if index >= len(attributes) or attributes[index] not in {"'", '"'}:
            continue

        quote = attributes[index]
        value_start = index + 1
        index = value_start
        while index < len(attributes) and attributes[index] != quote:
            index += 1
        if index >= len(attributes):
            return None
        value = attributes[value_start:index]
        index += 1
        if name == wanted:
            return value
    return None


def _dotnet_project_reference_includes(source: str) -> list[str]:
    """Read literal Include attributes from unqualified start elements."""
    includes: list[str] = []
    index = 0
    while index < len(source):
        index = source.find("<", index)
        if index < 0:
            break
        if source.startswith("<!--", index):
            index = _skip_xml_markup(source, index + 4, "-->")
            continue
        if source.startswith("<![CDATA[", index):
            index = _skip_xml_markup(source, index + 9, "]]>")
            continue
        if source.startswith("<?", index):
            index = _skip_xml_markup(source, index + 2, "?>")
            continue
        if source.startswith("<!", index):
            index = _skip_xml_markup(source, index + 2, ">")
            continue

        name, attributes, next_index = _parse_xml_start_tag(source, index)
        if name is None:
            index += 1
            continue
        index = next_index
        if name != "ProjectReference":
            continue
        include = _xml_literal_attribute(attributes, "Include")
        if include is not None:
            includes.append(include)
    return includes


def _normalized_dotnet_project_path(path: Path | str) -> str:
    return os.path.normcase(os.path.normpath(str(path))).lower()


def _dotnet_project_reference_path(project_file: Path, include: str) -> str | None:
    if (
        not include
        or any(character in include for character in "*?#&")
        or "$(" in include
        or _portable_path_is_absolute(include)
    ):
        return None
    portable = include.replace("/", os.sep).replace("\\", os.sep)
    return _normalized_dotnet_project_path(project_file.parent / portable)


def _build_known_dotnet_project_paths(
    packages: list[Package],
) -> dict[str, str]:
    known: dict[str, str] = {}
    for package in packages:
        if not _in_dependency_scope(package.language, "dotnet"):
            continue
        for project_file in _root_dotnet_project_files(package.path):
            known[_normalized_dotnet_project_path(project_file)] = package.name
    return known


def _parse_dotnet_deps(
    package: Package, known_project_paths: dict[str, str]
) -> list[str]:
    """Resolve literal root ProjectReference paths without opening targets."""
    dependencies: set[str] = set()
    for project_file in _root_dotnet_project_files(package.path):
        try:
            source = project_file.read_text(encoding="utf-8")
        except (OSError, UnicodeError):
            continue
        for include in _dotnet_project_reference_includes(source):
            target = _dotnet_project_reference_path(project_file, include)
            if target is None:
                continue
            dependency = known_project_paths.get(target)
            if dependency is not None and dependency != package.name:
                dependencies.add(dependency)
    return sorted(dependencies)


# ---------------------------------------------------------------------------
# Public API
# ---------------------------------------------------------------------------


_BUILD_TOOL_DEPS_RE = re.compile(
    r"(?m)^[ \t]*#\s*build-tool:\s*deps\s*=\s*(.+)$"
)


def _parse_build_tool_deps(
    package: Package, known_package_names: set[str]
) -> list[str]:
    """Read exact qualified cross-ecosystem dependencies from BUILD comments."""
    if not package.build_content:
        return []

    dependencies: set[str] = set()
    for match in _BUILD_TOOL_DEPS_RE.finditer(package.build_content):
        for raw in re.split(r"[,\t ]+", match.group(1)):
            dependency = raw.strip()
            if (
                dependency
                and dependency != package.name
                and dependency in known_package_names
            ):
                dependencies.add(dependency)
    return sorted(dependencies)


def _dependency_scope(language: str) -> str:
    """Return the ecosystem scope used for ordinary manifest aliases."""
    if language in {"csharp", "fsharp", "dotnet"}:
        return "dotnet"
    return language


def _in_dependency_scope(package_language: str, scope: str) -> bool:
    """Whether a package may contribute aliases to one resolver scope."""
    if scope == "dotnet":
        return package_language in {"csharp", "fsharp", "dotnet"}
    return package_language == scope


def _build_known_names_for_language(
    packages: list[Package], language: str
) -> dict[str, str]:
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
    known_paths: dict[str, Path] = {}
    known_languages: dict[str, str] = {}
    ambiguous: set[str] = set()
    scope = _dependency_scope(language)

    def _set_known(key: str, value: str, pkg_path: Path) -> None:
        """Prioritize libraries while rejecting same-priority Dart ambiguity."""
        package_language = value.split("/", 1)[0]
        if key in ambiguous:
            return
        if key not in known:
            known[key] = value
            known_paths[key] = pkg_path
            known_languages[key] = package_language
            return
        existing_is_program = "/programs/" in str(known_paths[key]).replace("\\", "/")
        current_is_program = "/programs/" in str(pkg_path).replace("\\", "/")
        if existing_is_program and not current_is_program:
            known[key] = value
            known_paths[key] = pkg_path
            known_languages[key] = package_language
            return
        if not existing_is_program and current_is_program:
            return
        if scope == "dart" and known[key] != value:
            del known[key]
            del known_paths[key]
            del known_languages[key]
            ambiguous.add(key)
            return
        if scope == "dotnet":
            if known_languages[key] == language:
                return
            if package_language == language:
                known[key] = value
                known_paths[key] = pkg_path
                known_languages[key] = package_language

    for pkg in packages:
        if not _in_dependency_scope(pkg.language, scope):
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

        elif pkg.language == "rust":
            # Rust crate names use the directory name directly (kebab-case).
            crate_name = pkg.path.name.lower()
            _set_known(crate_name, pkg.name, pkg.path)

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

        elif pkg.language == "dart":
            # Pub package identifiers use lower-case snake_case. Keep the
            # legacy coding_adventures_ prefix and the declared root name as
            # aliases so directory and manifest migrations resolve equally.
            dir_base = pkg.path.name.replace("-", "_").lower()
            _set_known(dir_base, pkg.name, pkg.path)
            _set_known(f"coding_adventures_{dir_base}", pkg.name, pkg.path)
            declared_name = _read_dart_package_name(pkg.path)
            if declared_name is not None:
                _set_known(declared_name, pkg.name, pkg.path)

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
            # Register modern directory names, the legacy prefix, and the
            # declared name from the sole root manifest.
            dir_base = pkg.path.name.lower()
            _set_known(dir_base, pkg.name, pkg.path)
            _set_known(f"coding-adventures-{dir_base}", pkg.name, pkg.path)
            declared_name = _read_cabal_package_name(pkg.path)
            if declared_name is not None:
                _set_known(declared_name, pkg.name, pkg.path)

        elif pkg.language in ("java", "kotlin"):
            # Java and Kotlin packages use Gradle composite builds. Dependencies
            # are referenced by directory name in settings.gradle.kts via
            # includeBuild("../dep-name"). The directory name maps directly.
            dir_base = pkg.path.name.lower()
            _set_known(dir_base, pkg.name, pkg.path)

        elif pkg.language in ("csharp", "fsharp", "dotnet"):
            # C#, F#, and shared dotnet programs form one MSBuild scope.
            dir_base = pkg.path.name.lower()
            _set_known(dir_base, pkg.name, pkg.path)

    return known


def _build_known_names(packages: list[Package]) -> dict[str, str]:
    """Build the legacy unscoped alias view used by mapping unit tests.

    Dependency resolution does not consume this view. It builds one table per
    ecosystem so same-spelled aliases from another language cannot redirect an
    edge.
    """
    known: dict[str, str] = {}
    for language in dict.fromkeys(package.language for package in packages):
        for alias, package_name in _build_known_names_for_language(
            packages, language
        ).items():
            known.setdefault(alias, package_name)
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

    # Ordinary manifest aliases are ecosystem-local. Exact qualified BUILD
    # comments are the portable escape hatch for intentional cross-lane edges.
    known_names_by_language = {
        language: _build_known_names_for_language(packages, language)
        for language in dict.fromkeys(pkg.language for pkg in packages)
    }
    known_gradle_paths_by_language = {
        language: _build_known_gradle_paths_for_language(packages, language)
        for language in ("java", "kotlin")
    }
    known_dotnet_project_paths = _build_known_dotnet_project_paths(packages)
    known_package_names = {pkg.name for pkg in packages}

    # Parse dependencies for each package.
    for pkg in packages:
        known_names = known_names_by_language[pkg.language]
        if pkg.language == "python":
            deps = _parse_python_deps(pkg, known_names)
        elif pkg.language == "ruby":
            deps = _parse_ruby_deps(pkg, known_names)
        elif pkg.language == "go":
            deps = _parse_go_deps(pkg, known_names)
        elif pkg.language == "typescript":
            deps = _parse_typescript_deps(pkg, known_names)
        elif pkg.language == "rust":
            deps = _parse_rust_deps(pkg, known_names)
        elif pkg.language == "elixir":
            deps = _parse_elixir_deps(pkg, known_names)
        elif pkg.language == "dart":
            deps = _parse_dart_deps(pkg, known_names)
        elif pkg.language == "lua":
            deps = _parse_lua_deps(pkg, known_names)
        elif pkg.language == "perl":
            deps = _parse_perl_deps(pkg, known_names)
        elif pkg.language == "swift":
            deps = _parse_swift_deps(pkg, known_names)
        elif pkg.language == "haskell":
            deps = _parse_haskell_deps(pkg, known_names)
        elif pkg.language in ("java", "kotlin"):
            deps = _parse_gradle_deps(
                pkg, known_gradle_paths_by_language[pkg.language]
            )
        elif pkg.language in ("csharp", "fsharp", "dotnet"):
            deps = _parse_dotnet_deps(pkg, known_dotnet_project_paths)
        else:
            deps = []

        deps.extend(_parse_build_tool_deps(pkg, known_package_names))

        for dep_name in sorted(set(deps)):
            # Edge direction: dep -> pkg means "dep must be built before pkg".
            # This makes independent_groups() produce the correct build order:
            # nodes with zero in-degree (no dependencies) come first.
            graph.add_edge(dep_name, pkg.name)

    return graph
