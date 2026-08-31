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
2. Normalize relative paths to forward-slash form and sort them for determinism.
3. Frame each repository-relative UTF-8 path with its byte length.
4. Append each file's unsigned 64-bit content length and exact raw bytes.
5. SHA256-hash that unambiguous sequence to produce the final package hash.

This framed hashing means:
- Reordering files doesn't change the hash (we sort normalized paths first).
- Adding or removing a file changes the hash (the framed sequence changes).
- Modifying any file's contents changes the hash.
- Renaming a file changes the hash, even when its contents do not.

Dependency hashing
------------------

A package should be rebuilt if any of its transitive dependencies changed.
``hash_deps`` takes a package name, the dependency graph, and the per-package
hashes, then produces a single hash representing the state of all dependencies.
"""

from __future__ import annotations

import hashlib
import os
import stat
from pathlib import Path
from typing import Protocol

from build_tool.discovery import Package
from build_tool.glob_match import match_path
from build_tool.resolver import DirectedGraph

# Source file extensions that matter for each language.
# If any of these files change, the package needs rebuilding.
SOURCE_EXTENSIONS: dict[str, set[str]] = {
    "python": {".py", ".toml", ".cfg"},
    "ruby": {".rb", ".gemspec"},
    "go": {".go"},
    "perl": {".pl", ".pm", ".t", ".xs"},
    "ocaml": {".ml", ".mli", ".opam"},
}

# Special filenames to always include regardless of extension.
SPECIAL_FILENAMES: dict[str, set[str]] = {
    "python": set(),
    "ruby": {"Gemfile", "Rakefile"},
    "go": {"go.mod", "go.sum"},
    "perl": {
        "Makefile.PL",
        "Build.PL",
        "cpanfile",
        "MANIFEST",
        "META.json",
        "META.yml",
    },
    "ocaml": {".ocamlformat", "dune", "dune-project"},
}

# Manifest extensions that affect the package independently of a Starlark
# target's declared source globs. Source extensions such as ``.ml`` remain
# governed by ``declared_srcs``; package manifests such as ``.opam`` do not.
DECLARED_MANIFEST_EXTENSIONS: dict[str, set[str]] = {
    "ocaml": {".opam"},
}

# Exact, case-sensitive generated, dependency, VCS, cache, and temporary
# directory components excluded by the shared source-collection contract.
GENERATED_DIRECTORY_COMPONENTS: frozenset[str] = frozenset(
    {
        ".git",
        ".hg",
        ".svn",
        ".venv",
        ".tox",
        ".mypy_cache",
        ".pytest_cache",
        ".ruff_cache",
        ".stack-work",
        "__pycache__",
        "node_modules",
        "vendor",
        "dist",
        "dist-newstyle",
        "_build",
        "build",
        "target",
        ".claude",
        "Pods",
        ".gradle",
        ".dart_tool",
        "gradle-build",
        "deps",
        ".build",
        ".cargo",
        "cover",
    }
)


class _HashUpdater(Protocol):
    """Structural type for the byte-update surface used by hashlib objects."""

    def update(self, data: bytes, /) -> None: ...


def _is_link_or_reparse(path: Path) -> bool:
    """Return whether ``path`` is a symlink, junction, or Windows reparse point."""
    if os.path.islink(path):
        return True

    isjunction = getattr(os.path, "isjunction", None)
    if isjunction is not None and isjunction(path):
        return True

    if os.name == "nt":
        try:
            attributes = os.lstat(path).st_file_attributes
        except (AttributeError, OSError):
            return True
        return bool(attributes & stat.FILE_ATTRIBUTE_REPARSE_POINT)

    return False


def _prune_generated_directories(dirpath: str, dirnames: list[str]) -> None:
    """Prevent ``os.walk`` from descending into generated or linked components."""
    dirnames[:] = [
        dirname
        for dirname in dirnames
        if dirname not in GENERATED_DIRECTORY_COMPONENTS
        and not _is_link_or_reparse(Path(dirpath) / dirname)
    ]


def _collect_source_files(package: Package) -> list[Path]:
    """Collect all source files in a package directory.

    There are two collection modes:

    1. **Starlark mode** (``package.is_starlark`` and ``package.declared_srcs``):
       Walk the directory tree with ``os.walk()`` and test each file against
       the declared source patterns using ``glob_match.match_path()``. BUILD
       files are always included.

       This fixes a bug with Python's ``pathlib.Path.glob("**/*.py")`` which
       does NOT match files in the immediate directory (only subdirectories).
       By using ``os.walk()`` + ``match_path()``, we correctly handle ``**``
       as "zero or more directory levels", matching the Bazel/Go semantics.

    2. **Extension mode** (shell BUILD files or no declared_srcs):
       Walk the directory tree and filter by the language's relevant file
       extensions and special filenames. This is the original behavior.

    In both modes, BUILD files are always included, and the result is sorted
    by relative path for deterministic hashing.

    Returns a sorted list of absolute paths.
    """
    files: list[Path] = []
    pkg_root = str(package.path)
    extensions = SOURCE_EXTENSIONS.get(package.language, set())
    special_names = SPECIAL_FILENAMES.get(package.language, set())
    manifest_extensions = DECLARED_MANIFEST_EXTENSIONS.get(
        package.language, set()
    )

    if package.is_starlark and package.declared_srcs:
        # Starlark mode: use os.walk + glob_match for precise source matching.
        #
        # os.walk gives us (dirpath, dirnames, filenames) tuples. We compute
        # each file's path relative to the package root and test it against
        # every declared source pattern.
        #
        # This replaces pathlib.glob/rglob which has inconsistent behavior
        # with ** patterns across Python versions and platforms.
        for dirpath, dirnames, filenames in os.walk(pkg_root, followlinks=False):
            _prune_generated_directories(dirpath, dirnames)
            for filename in filenames:
                abs_path = Path(dirpath) / filename
                if _is_link_or_reparse(abs_path):
                    continue

                # Always include BUILD files (a change to the build definition
                # itself should always trigger a rebuild).
                if filename in ("BUILD", "BUILD_mac", "BUILD_linux",
                                "BUILD_windows", "BUILD_mac_and_linux"):
                    files.append(abs_path)
                    continue

                # Manifests affect the package even when a Starlark target's
                # declared source globs omit them. This is especially visible
                # for OCaml's exact ``dune-project`` and ``.ocamlformat`` names.
                if filename in special_names or (
                    abs_path.parent == package.path
                    and Path(filename).suffix in manifest_extensions
                ):
                    files.append(abs_path)
                    continue

                # Compute the file's path relative to the package root.
                # os.path.relpath gives us a platform-native path, but we
                # need forward slashes for glob matching consistency.
                rel_path = os.path.relpath(abs_path, pkg_root).replace(
                    os.sep, "/"
                )

                # Test against each declared source pattern.
                for pattern in package.declared_srcs:
                    if match_path(pattern, rel_path):
                        files.append(abs_path)
                        break
    else:
        # Extension mode: filter by language-specific extensions.
        for dirpath, dirnames, filenames in os.walk(pkg_root, followlinks=False):
            _prune_generated_directories(dirpath, dirnames)
            for filename in filenames:
                abs_path = Path(dirpath) / filename
                if _is_link_or_reparse(abs_path):
                    continue

                # Always include BUILD files
                if filename in ("BUILD", "BUILD_mac", "BUILD_linux",
                                "BUILD_windows", "BUILD_mac_and_linux"):
                    files.append(abs_path)
                    continue

                # Check extension
                if Path(filename).suffix in extensions:
                    files.append(abs_path)
                    continue

                # Check special filenames
                if filename in special_names:
                    files.append(abs_path)
                    continue

    # ``Path`` renders separators according to the host. Hash ordering is part
    # of the portable contract, so normalize before sorting rather than merely
    # replacing separators later in ``hash_package``.
    files.sort(key=lambda path: path.relative_to(package.path).as_posix())
    return files


def _hash_file(filepath: Path) -> str:
    """Compute the SHA256 hex digest of a single file's contents."""
    sha = hashlib.sha256()
    with open(filepath, "rb") as f:
        for chunk in iter(lambda: f.read(8192), b""):
            sha.update(chunk)
    return sha.hexdigest()


def _repository_relative_package_path(package: Package) -> str:
    """Return the package root in normalized repository-relative form.

    Production packages live below the canonical ``code/packages`` or
    ``code/programs`` buckets. Locating that bucket in the absolute checkout
    path removes machine-specific prefixes while preserving any nested package
    path. The identity fallback keeps isolated unit fixtures deterministic.
    """
    parts = package.path.parts
    for index in range(len(parts) - 2, -1, -1):
        if parts[index] == "code" and parts[index + 1] in {
            "packages",
            "programs",
        }:
            return "/".join(parts[index:])

    identity = package.name.split("/")
    if len(identity) == 3 and identity[1] == "programs":
        return "/".join(("code", "programs", identity[0], identity[2]))
    if len(identity) == 2:
        return "/".join(("code", "packages", *identity))
    raise ValueError(f"cannot derive repository path for package {package.name!r}")


def _source_signature(source_stat: os.stat_result) -> tuple[int, int, int, int, int]:
    """Return identity and mutation-sensitive fields for an opened source."""
    return (
        source_stat.st_dev,
        source_stat.st_ino,
        source_stat.st_size,
        source_stat.st_mtime_ns,
        source_stat.st_ctime_ns,
    )


def _validate_open_source(filepath: Path, source_stat: os.stat_result) -> None:
    """Reject linked, replaced, or non-regular paths after opening a handle."""
    path_stat = os.lstat(filepath)
    attributes = getattr(path_stat, "st_file_attributes", 0)
    reparse_flag = getattr(stat, "FILE_ATTRIBUTE_REPARSE_POINT", 0)
    is_reparse = bool(attributes & reparse_flag)
    if (
        not stat.S_ISREG(source_stat.st_mode)
        or not stat.S_ISREG(path_stat.st_mode)
        or is_reparse
        or not os.path.samestat(source_stat, path_stat)
    ):
        raise OSError("source path changed or is not a regular file")


def _update_file_frame(
    package_hash: _HashUpdater, repository_path: str, filepath: Path
) -> None:
    """Append one hashing-v1 path/content frame without decoding file bytes."""
    path_bytes = repository_path.encode("utf-8")
    package_hash.update(len(path_bytes).to_bytes(8, "big"))
    package_hash.update(path_bytes)

    flags = os.O_RDONLY | getattr(os, "O_BINARY", 0)
    flags |= getattr(os, "O_NOFOLLOW", 0)
    descriptor = os.open(filepath, flags)
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        _validate_open_source(filepath, before)
        before_signature = _source_signature(before)
        content_length = before.st_size
        package_hash.update(content_length.to_bytes(8, "big"))

        bytes_read = 0
        for chunk in iter(lambda: source.read(8192), b""):
            package_hash.update(chunk)
            bytes_read += len(chunk)

        after = os.fstat(source.fileno())
        _validate_open_source(filepath, after)

    if bytes_read != content_length or _source_signature(after) != before_signature:
        raise OSError("source changed while hashing")


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

    # A content-only sequence cannot distinguish a rename from an unchanged
    # file. Hashing v1 frames every normalized repository-relative UTF-8 path
    # and exact raw content with unsigned 64-bit byte lengths. This makes file
    # boundaries unambiguous without decoding bytes or incorporating absolute
    # checkout locations.
    package_hash = hashlib.sha256()
    package_root = _repository_relative_package_path(package)
    for filepath in files:
        relative_path = filepath.relative_to(package.path).as_posix()
        _update_file_frame(
            package_hash, f"{package_root}/{relative_path}", filepath
        )
    return package_hash.hexdigest()


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
