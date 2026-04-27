"""
discovery.py -- Package Discovery via Recursive BUILD File Walk
================================================================

This module walks a monorepo directory tree to discover packages. A "package"
is any directory that contains a BUILD file. The walk is recursive: starting
from the root, we list all subdirectories and descend into each one, skipping
known non-source directories (.git, .venv, node_modules, etc.).

When we find a BUILD file in a directory, we stop recursing there and register
that directory as a package. This is the same approach used by Bazel, Buck,
and Pants — no configuration files are needed to route the walk.

Platform-specific BUILD files
-----------------------------

If we're on macOS and a ``BUILD_mac`` file exists, we use that instead of
``BUILD``. Similarly, ``BUILD_linux`` on Linux. This lets packages define
platform-specific build commands (e.g., different compiler flags).

Language inference
-----------------

We infer the language from the directory path. If the path contains
``packages/python/X`` or ``programs/python/X``, the language is "python".
Similarly for "ruby", "go", and "rust". The package name is
``{language}/{dir-name}``.
"""

from __future__ import annotations

import platform
from dataclasses import dataclass, field
from pathlib import Path


# Directories that should never be traversed during package discovery.
# These are known to contain non-source files (caches, dependencies,
# build artifacts) that would waste time to scan.
SKIP_DIRS: frozenset[str] = frozenset({
    ".git",
    ".hg",
    ".svn",
    ".venv",
    ".tox",
    ".mypy_cache",
    ".pytest_cache",
    ".ruff_cache",
    "__pycache__",
    "node_modules",
    "vendor",
    "dist",
    "build",
    "target",
    ".claude",
    "Pods",
    ".gradle",
    "gradle-build",
})


@dataclass
class Package:
    """Represents a discovered package in the monorepo.

    Attributes:
        name: A qualified name like "python/logic-gates" or "ruby/arithmetic".
        path: Absolute path to the package directory.
        build_commands: Lines from the BUILD file (commands to execute).
        language: Inferred language -- "python", "ruby", "go", "rust", or "unknown".
        build_content: Raw BUILD file content (used for Starlark detection).
        is_starlark: Whether the BUILD file uses Starlark syntax.
        declared_srcs: Glob patterns from the Starlark srcs field.
        declared_deps: Qualified dependency names from the Starlark deps field.
    """

    name: str
    path: Path
    build_commands: list[str] = field(default_factory=list)
    language: str = "unknown"
    build_content: str = ""
    is_starlark: bool = False
    declared_srcs: list[str] = field(default_factory=list)
    declared_deps: list[str] = field(default_factory=list)


def _read_lines(filepath: Path) -> list[str]:
    """Read a file and return non-blank, non-comment lines.

    Blank lines and lines starting with '#' are stripped out. Leading and
    trailing whitespace is removed from each line.
    """
    if not filepath.exists():
        return []

    lines: list[str] = []
    text = filepath.read_text(encoding="utf-8")
    for line in text.splitlines():
        stripped = line.strip()
        if stripped and not stripped.startswith("#"):
            lines.append(stripped)
    return lines


def _infer_language(path: Path) -> str:
    """Infer the programming language from the directory path.

    We look for known language directory names in the path components.
    The pattern we look for is a parent directory named "python", "ruby",
    "go", or "rust" that sits under "packages" or "programs".
    """
    parts = path.parts
    for lang in ("python", "ruby", "go", "rust", "typescript", "elixir"):
        if lang in parts:
            return lang
    return "unknown"


def _infer_package_name(path: Path, language: str) -> str:
    """Build a qualified package name like 'python/logic-gates'.

    Uses the language and the directory's basename.
    """
    return f"{language}/{path.name}"


def _get_build_file(directory: Path) -> Path | None:
    """Return the appropriate BUILD file for the current platform.

    Priority (most specific wins):
    1. Platform-specific: BUILD_mac (macOS), BUILD_linux (Linux), BUILD_windows (Windows)
    2. Shared: BUILD_mac_and_linux (macOS or Linux — for Unix-like systems)
    3. Generic: BUILD (all platforms)
    4. None if no BUILD file exists

    This layering lets packages provide Windows-specific build commands via
    BUILD_windows while sharing a single BUILD_mac_and_linux for the common
    Unix case, falling back to BUILD when no platform differences exist.
    """
    system = platform.system()

    # Step 1: Check for the most specific platform file.
    if system == "Darwin":
        platform_build = directory / "BUILD_mac"
        if platform_build.exists():
            return platform_build

    if system == "Linux":
        platform_build = directory / "BUILD_linux"
        if platform_build.exists():
            return platform_build

    if system == "Windows":
        platform_build = directory / "BUILD_windows"
        if platform_build.exists():
            return platform_build

    # Step 2: Check for the shared Unix file (macOS + Linux).
    if system in ("Darwin", "Linux"):
        shared_build = directory / "BUILD_mac_and_linux"
        if shared_build.exists():
            return shared_build

    # Step 3: Fall back to the generic BUILD file.
    generic_build = directory / "BUILD"
    if generic_build.exists():
        return generic_build

    return None


def discover_packages(root: Path) -> list[Package]:
    """Recursively walk the directory tree, collect packages with BUILD files.

    Starting from ``root``, we list all subdirectories and descend into
    each one (skipping directories in the skip list). When we find a BUILD
    file, we register that directory as a package and stop recursing into it.

    Args:
        root: The monorepo root directory.

    Returns:
        A list of discovered Package objects, sorted by name.
    """
    packages: list[Package] = []
    _walk_dirs(root, packages)
    packages.sort(key=lambda p: p.name)
    return packages


def _walk_dirs(directory: Path, packages: list[Package]) -> None:
    """Recursively walk directories and collect packages.

    If the current directory's name is in the skip list, ignore it entirely.
    If the current directory has a BUILD file, it's a package -- register it
    and don't recurse further. Otherwise, list all subdirectories and recurse.
    """
    # Skip known non-source directories.
    if directory.name in SKIP_DIRS:
        return

    build_file = _get_build_file(directory)

    if build_file is not None:
        # This directory is a package. Read the BUILD commands and raw content.
        commands = _read_lines(build_file)
        try:
            content = build_file.read_text(encoding="utf-8")
        except OSError:
            content = ""
        language = _infer_language(directory)
        name = _infer_package_name(directory, language)

        packages.append(
            Package(
                name=name,
                path=directory,
                build_commands=commands,
                language=language,
                build_content=content,
            )
        )
        return

    # Not a package -- list subdirectories and recurse into each one.
    try:
        entries = sorted(directory.iterdir())
    except PermissionError:
        return

    for entry in entries:
        if entry.is_dir():
            _walk_dirs(entry, packages)
