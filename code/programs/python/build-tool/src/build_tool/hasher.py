"""
hasher.py -- SHA256 File Hashing for Change Detection
=====================================================

This module computes SHA256 hashes for package source files. The hash of a
package is a single string that changes whenever any source file in the
package is modified, added, or removed.

How hashing works
-----------------

1. Collect all source files in the package directory, filtered by the
   language's relevant extensions. Always include the BUILD file.
2. Sort the file list lexicographically (by relative path) for determinism.
3. SHA256-hash each file's contents individually.
4. Concatenate all individual hashes into one string.
5. SHA256-hash that concatenated string to produce the final package hash.

This two-level hashing means:
- Reordering files doesn't change the hash (we sort first).
- Adding or removing a file changes the hash (the concatenated string changes).
- Modifying any file's contents changes the hash.

Dependency hashing
------------------

A package should be rebuilt if any of its transitive dependencies changed.
``hash_deps`` takes a package name, the dependency graph, and the per-package
hashes, then produces a single hash representing the state of all dependencies.
"""

from __future__ import annotations

import hashlib
from pathlib import Path

from build_tool.discovery import Package
from build_tool.resolver import DirectedGraph

# Source file extensions that matter for each language.
# If any of these files change, the package needs rebuilding.
SOURCE_EXTENSIONS: dict[str, set[str]] = {
    "python": {".py", ".toml", ".cfg"},
    "ruby": {".rb", ".gemspec"},
    "go": {".go"},
}

# Special filenames to always include regardless of extension.
SPECIAL_FILENAMES: dict[str, set[str]] = {
    "python": set(),
    "ruby": {"Gemfile", "Rakefile"},
    "go": {"go.mod", "go.sum"},
}


def _collect_source_files(package: Package) -> list[Path]:
    """Collect all source files in a package directory.

    Files are filtered by the language's relevant extensions and special
    filenames. The BUILD file is always included.

    Returns a sorted list of absolute paths.
    """
    extensions = SOURCE_EXTENSIONS.get(package.language, set())
    special_names = SPECIAL_FILENAMES.get(package.language, set())

    files: list[Path] = []

    for filepath in package.path.rglob("*"):
        if not filepath.is_file():
            continue

        # Always include BUILD files
        if filepath.name in ("BUILD", "BUILD_mac", "BUILD_linux"):
            files.append(filepath)
            continue

        # Check extension
        if filepath.suffix in extensions:
            files.append(filepath)
            continue

        # Check special filenames
        if filepath.name in special_names:
            files.append(filepath)
            continue

    # Sort by relative path for determinism
    files.sort(key=lambda f: str(f.relative_to(package.path)))
    return files


def _hash_file(filepath: Path) -> str:
    """Compute the SHA256 hex digest of a single file's contents."""
    sha = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            sha.update(chunk)
    return sha.hexdigest()


def hash_package(package: Package) -> str:
    """Compute a SHA256 hash representing all source files in the package.

    The hash changes if any source file is added, removed, or modified.

    Args:
        package: The package to hash.

    Returns:
        A hex-encoded SHA256 hash string.
    """
    files = _collect_source_files(package)

    if not files:
        # No source files -- hash the empty string for consistency
        return hashlib.sha256(b"").hexdigest()

    # Hash each file, concatenate, hash again
    file_hashes = [_hash_file(f) for f in files]
    combined = "".join(file_hashes)
    return hashlib.sha256(combined.encode("utf-8")).hexdigest()


def hash_deps(
    package_name: str,
    graph: DirectedGraph,
    package_hashes: dict[str, str],
) -> str:
    """Compute a SHA256 hash of all transitive dependency hashes.

    If any transitive dependency's source files changed, this hash will
    change too, triggering a rebuild of the dependent package.

    Args:
        package_name: The package whose dependencies we're hashing.
        graph: The dependency graph.
        package_hashes: Mapping from package name to its source hash.

    Returns:
        A hex-encoded SHA256 hash string. If the package has no dependencies,
        returns the hash of an empty string.
    """
    # Get all transitive dependencies (packages this one depends on).
    # In our graph, edges go dep -> pkg (dependency points to dependent),
    # so a package's dependencies are its predecessors (reverse direction).
    if not graph.has_node(package_name):
        return hashlib.sha256(b"").hexdigest()

    transitive_deps = graph.transitive_dependents(package_name)

    if not transitive_deps:
        return hashlib.sha256(b"").hexdigest()

    # Sort dependency names for determinism, concatenate their hashes.
    sorted_deps = sorted(transitive_deps)
    combined = "".join(package_hashes.get(dep, "") for dep in sorted_deps)
    return hashlib.sha256(combined.encode("utf-8")).hexdigest()
