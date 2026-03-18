"""
discovery.py -- Package Discovery via DIRS/BUILD Files
======================================================

This module walks a monorepo directory tree following DIRS files to discover
packages. A "package" is any directory that contains a BUILD file. DIRS files
act as a routing table: each non-blank, non-comment line names a subdirectory
to descend into.

The walk is recursive: if ``code/DIRS`` contains "packages", we look at
``code/packages/``. If ``code/packages/DIRS`` contains "python" and "ruby",
we look at both. When we find a BUILD file in a directory, we stop recursing
there and register that directory as a package.

Platform-specific BUILD files
-----------------------------

If we're on macOS and a ``BUILD_mac`` file exists, we use that instead of
``BUILD``. Similarly, ``BUILD_linux`` on Linux. This lets packages define
platform-specific build commands (e.g., different compiler flags).

Language inference
-----------------

We infer the language from the directory path. If the path contains
``packages/python/X`` or ``programs/python/X``, the language is "python".
Similarly for "ruby" and "go". The package name is ``{language}/{dir-name}``.
"""

from __future__ import annotations

import platform
import re
from dataclasses import dataclass, field
from pathlib import Path


@dataclass
class Package:
    """Represents a discovered package in the monorepo.

    Attributes:
        name: A qualified name like "python/logic-gates" or "ruby/arithmetic".
        path: Absolute path to the package directory.
        build_commands: Lines from the BUILD file (commands to execute).
        language: Inferred language -- "python", "ruby", "go", or "unknown".
    """

    name: str
    path: Path
    build_commands: list[str] = field(default_factory=list)
    language: str = "unknown"


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
    or "go" that sits under "packages" or "programs".
    """
    parts = path.parts
    for lang in ("python", "ruby", "go"):
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

    Priority:
    1. BUILD_mac on macOS, BUILD_linux on Linux
    2. BUILD (fallback)
    3. None if no BUILD file exists
    """
    system = platform.system()

    if system == "Darwin":
        platform_build = directory / "BUILD_mac"
        if platform_build.exists():
            return platform_build

    if system == "Linux":
        platform_build = directory / "BUILD_linux"
        if platform_build.exists():
            return platform_build

    generic_build = directory / "BUILD"
    if generic_build.exists():
        return generic_build

    return None


def discover_packages(root: Path) -> list[Package]:
    """Walk DIRS files recursively, collect packages with BUILD files.

    Starting from ``root``, we read the DIRS file (if present) and descend
    into each listed subdirectory. When we find a BUILD file, we register
    that directory as a package and stop recursing into it.

    Args:
        root: The monorepo root directory (where the top-level DIRS file is).

    Returns:
        A list of discovered Package objects, sorted by name.
    """
    packages: list[Package] = []
    _walk_dirs(root, packages)
    packages.sort(key=lambda p: p.name)
    return packages


def _walk_dirs(directory: Path, packages: list[Package]) -> None:
    """Recursively walk DIRS files and collect packages.

    If the current directory has a BUILD file, it's a package -- register it
    and don't recurse further. Otherwise, if it has a DIRS file, read the
    listed subdirectories and recurse into each one.
    """
    build_file = _get_build_file(directory)

    if build_file is not None:
        # This directory is a package. Read the BUILD commands.
        commands = _read_lines(build_file)
        language = _infer_language(directory)
        name = _infer_package_name(directory, language)

        packages.append(
            Package(
                name=name,
                path=directory,
                build_commands=commands,
                language=language,
            )
        )
        return

    # Not a package -- look for DIRS file to find subdirectories.
    dirs_file = directory / "DIRS"
    if not dirs_file.exists():
        return

    subdirs = _read_lines(dirs_file)
    for subdir_name in subdirs:
        subdir_path = directory / subdir_name
        if subdir_path.is_dir():
            _walk_dirs(subdir_path, packages)
