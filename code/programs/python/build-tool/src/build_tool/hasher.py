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
hashes, then frames each sorted dependency identity with its decoded 32-byte
digest and produces a single hash representing the dependency state.
``combine_hashes`` hashes the raw package and dependency digests together for
the portable cache identity.
"""

from __future__ import annotations

import hashlib
import os
import stat
import unicodedata
from pathlib import Path
from typing import Protocol

from build_tool.discovery import Package
from build_tool.glob_match import match_path, validate_pattern
from build_tool.resolver import DirectedGraph
from build_tool.source_input_registry import (
    generated_components,
    package_registry_identity,
    source_input_selected,
)

# Exact, case-sensitive generated, dependency, VCS, cache, and temporary
# directory components come from the same production projection as selectors.
GENERATED_DIRECTORY_COMPONENTS: frozenset[str] = generated_components()
_MAX_CANDIDATES = 100_000
_MAX_SELECTED = 50_000
_MAX_FILE_BYTES = 64 * 1024 * 1024
_MAX_PACKAGE_BYTES = 1024 * 1024 * 1024
_MAX_GLOB_WORK = 50_000_000
_WINDOWS_RESERVED = frozenset(
    {"CON", "PRN", "AUX", "NUL", "CONIN$", "CONOUT$", "CLOCK$"}
    | {f"COM{number}" for number in range(1, 10)}
    | {f"LPT{number}" for number in range(1, 10)}
    | {f"COM{number}" for number in "¹²³"}
    | {f"LPT{number}" for number in "¹²³"}
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


def _validate_candidate_path(relative: str, seen: dict[str, str]) -> None:
    """Reject unsafe names and portable-identity aliases before selection."""
    if len(relative.encode("utf-8")) > 512:
        raise OSError("SOURCE_HASH_LIMIT_EXCEEDED")
    if unicodedata.normalize("NFC", relative) != relative:
        raise OSError("source path has a noncanonical Unicode spelling")
    prefix: list[str] = []
    for component in relative.split("/"):
        if (
            not component
            or component in {".", ".."}
            or component.endswith((".", " "))
            or any(char in '\\<>:"|?*' for char in component)
            or any(unicodedata.category(char).startswith("C") for char in component)
            or component.split(".", 1)[0].upper() in _WINDOWS_RESERVED
        ):
            raise OSError("source path has an unsafe component")
        prefix.append(component)
        spelling = "/".join(prefix)
        identity = spelling.casefold()
        previous = seen.setdefault(identity, spelling)
        if previous != spelling:
            raise OSError("source paths have a portable identity alias")


def _charge_glob_work(pattern: str, path: str, previous: int) -> int:
    """Bound the complete pattern/path product before an attempted match."""
    total = previous + (len(pattern) + 1) * (len(path) + 1)
    if total > _MAX_GLOB_WORK:
        raise OSError("SOURCE_HASH_LIMIT_EXCEEDED")
    return total


def _collect_source_files(package: Package) -> list[Path]:
    """Collect all source files in a package directory.

    Fixed BUILD, root, and exact-path inputs apply in both modes. Recursive
    suffix/name and scoped inputs apply only in extension mode; declared mode
    additionally admits target-declared portable globs. Pruning and inert link
    boundaries precede all selectors. Return absolute paths in UTF-8 order.
    """
    files: list[Path] = []
    language, canonical_root = package_registry_identity(
        package.path, package.language, package.repository_root
    )
    # Refuse every lexical ancestor before opening the root for enumeration.
    if any(
        _is_link_or_reparse(component)
        for component in (package.path, *package.path.parents)
    ):
        raise OSError("source package root contains a linked directory")
    declared = package.is_starlark
    if declared:
        for pattern in package.declared_srcs:
            validate_pattern(pattern)

    seen: dict[str, str] = {}
    candidate_count = 0
    glob_work = 0
    pending = [package.path]
    while pending:
        directory = pending.pop()
        # scandir yields incrementally. A single huge directory cannot make
        # os.walk materialize an unbounded pair of entry lists before the cap.
        with os.scandir(directory) as entries:
            for entry in entries:
                candidate_count += 1
                if candidate_count > _MAX_CANDIDATES:
                    raise OSError("SOURCE_HASH_LIMIT_EXCEEDED")
                abs_path = Path(entry.path)
                rel_path = abs_path.relative_to(package.path).as_posix()
                _validate_candidate_path(rel_path, seen)
                if _is_link_or_reparse(abs_path):
                    continue
                if entry.is_dir(follow_symlinks=False):
                    if entry.name not in GENERATED_DIRECTORY_COMPONENTS:
                        pending.append(abs_path)
                    continue
                selected = source_input_selected(
                    language, canonical_root, rel_path, declared=declared
                )
                if declared and not selected:
                    for pattern in package.declared_srcs:
                        glob_work = _charge_glob_work(pattern, rel_path, glob_work)
                        if match_path(pattern, rel_path):
                            selected = True
                            break
                if selected:
                    if not entry.is_file(follow_symlinks=False):
                        raise OSError("source path is not a regular file")
                    if len(files) >= _MAX_SELECTED:
                        raise OSError("SOURCE_HASH_LIMIT_EXCEEDED")
                    files.append(abs_path)

    # ``Path`` renders separators according to the host. Hash ordering is part
    # of the portable contract, so normalize before sorting rather than merely
    # replacing separators later in ``hash_package``.
    files.sort(
        key=lambda path: path.relative_to(package.path).as_posix().encode("utf-8")
    )
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

    Production discovery supplies a repository root, including for reviewed
    ``code/sites`` graph nodes. The fallback keeps isolated unit fixtures
    deterministic without granting exact-package selector authority.
    """
    if package.repository_root is not None:
        return package.path.relative_to(package.repository_root).as_posix()
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
        or source_stat.st_nlink != 1
        or path_stat.st_nlink != 1
        or is_reparse
        or not os.path.samestat(source_stat, path_stat)
    ):
        raise OSError("source path changed or is not a regular file")


def _windows_final_handle_path(descriptor: int) -> str:
    """Return the final resolved path owned by an open Windows descriptor."""
    import ctypes
    import msvcrt
    from ctypes import wintypes

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    get_final_path = kernel32.GetFinalPathNameByHandleW
    get_final_path.argtypes = (
        wintypes.HANDLE,
        wintypes.LPWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
    )
    get_final_path.restype = wintypes.DWORD
    handle = msvcrt.get_osfhandle(descriptor)
    buffer = ctypes.create_unicode_buffer(32768)
    length = get_final_path(handle, buffer, len(buffer), 0)
    if length == 0 or length >= len(buffer):
        raise OSError("cannot resolve opened source path")

    final_path = buffer.value
    if final_path.startswith("\\\\?\\UNC\\"):
        return f"\\\\{final_path[8:]}"
    if final_path.startswith("\\\\?\\"):
        return final_path[4:]
    return final_path


def _windows_lock_unlinked_directories(directory: Path) -> list[int]:
    """Lock each lexical directory component and reject Windows reparses."""
    import ctypes
    from ctypes import wintypes

    class FileAttributeTagInfo(ctypes.Structure):
        _fields_ = (
            ("file_attributes", wintypes.DWORD),
            ("reparse_tag", wintypes.DWORD),
        )

    kernel32 = ctypes.WinDLL("kernel32", use_last_error=True)
    create_file = kernel32.CreateFileW
    create_file.argtypes = (
        wintypes.LPCWSTR,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.LPVOID,
        wintypes.DWORD,
        wintypes.DWORD,
        wintypes.HANDLE,
    )
    create_file.restype = wintypes.HANDLE
    get_file_information = kernel32.GetFileInformationByHandleEx
    get_file_information.argtypes = (
        wintypes.HANDLE,
        ctypes.c_int,
        wintypes.LPVOID,
        wintypes.DWORD,
    )
    get_file_information.restype = wintypes.BOOL
    close_handle = kernel32.CloseHandle
    close_handle.argtypes = (wintypes.HANDLE,)
    close_handle.restype = wintypes.BOOL

    file_read_attributes = 0x0080
    file_share_read = 0x00000001
    open_existing = 3
    file_attribute_reparse_point = 0x00000400
    file_flag_backup_semantics = 0x02000000
    file_flag_open_reparse_point = 0x00200000
    invalid_handle_value = ctypes.c_void_p(-1).value

    handles: list[int] = []
    current = Path(directory.absolute().parts[0])
    try:
        for component in directory.absolute().parts[1:]:
            current /= component
            handle = create_file(
                str(current),
                file_read_attributes,
                file_share_read,
                None,
                open_existing,
                file_flag_backup_semantics | file_flag_open_reparse_point,
                None,
            )
            if handle == invalid_handle_value:
                raise OSError("cannot lock source directory")
            handles.append(handle)

            attributes = FileAttributeTagInfo()
            if not get_file_information(
                handle,
                9,
                ctypes.byref(attributes),
                ctypes.sizeof(attributes),
            ):
                raise OSError("cannot inspect source directory")
            if attributes.file_attributes & file_attribute_reparse_point:
                raise OSError("source path contains a linked directory")
    except BaseException:
        for handle in reversed(handles):
            close_handle(handle)
        raise
    return handles


def _windows_close_handles(handles: list[int]) -> None:
    """Close Windows directory locks acquired for a source path."""
    import ctypes
    from ctypes import wintypes

    close_handle = ctypes.WinDLL("kernel32", use_last_error=True).CloseHandle
    close_handle.argtypes = (wintypes.HANDLE,)
    close_handle.restype = wintypes.BOOL
    for handle in reversed(handles):
        close_handle(handle)


def _open_source_no_follow(package_root: Path, filepath: Path) -> int:
    """Open a source without following a linked component or escaping its root."""
    try:
        relative_path = filepath.relative_to(package_root)
    except ValueError as error:
        raise OSError("source path is outside its package") from error
    if any(component in {"", ".", ".."} for component in relative_path.parts):
        raise OSError("source path is outside its package")

    if os.name == "nt":
        directory_handles = _windows_lock_unlinked_directories(filepath.parent)
        try:
            descriptor = os.open(filepath, os.O_RDONLY | os.O_BINARY)
            try:
                final_path = os.path.normcase(
                    os.path.normpath(_windows_final_handle_path(descriptor))
                )
                lexical_path = os.path.normcase(
                    os.path.normpath(str(filepath.absolute()))
                )
                if final_path != lexical_path:
                    raise OSError("opened source did not retain its lexical path")
            except BaseException:
                os.close(descriptor)
                raise
            return descriptor
        except BaseException:
            raise
        finally:
            _windows_close_handles(directory_handles)

    absolute_path = filepath.absolute()
    parts = absolute_path.parts
    no_follow = getattr(os, "O_NOFOLLOW", None)
    if not isinstance(no_follow, int) or no_follow == 0:
        raise OSError("source no-follow support is unavailable")
    nonblocking = getattr(os, "O_NONBLOCK", None)
    if not isinstance(nonblocking, int) or nonblocking == 0:
        raise OSError("source nonblocking open support is unavailable")
    directory_flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0) | no_follow
    directory = os.open(parts[0], directory_flags)
    try:
        for component in parts[1:-1]:
            child = os.open(
                component,
                directory_flags,
                dir_fd=directory,
            )
            os.close(directory)
            directory = child
        return os.open(
            parts[-1],
            os.O_RDONLY | no_follow | nonblocking,
            dir_fd=directory,
        )
    finally:
        os.close(directory)


def _update_file_frame(
    package_hash: _HashUpdater,
    repository_path: str,
    filepath: Path,
    package_root: Path,
    remaining_bytes: int = _MAX_PACKAGE_BYTES,
) -> int:
    """Append one hashing-v1 path/content frame without decoding file bytes."""
    path_bytes = repository_path.encode("utf-8")
    package_hash.update(len(path_bytes).to_bytes(8, "big"))
    package_hash.update(path_bytes)

    descriptor = _open_source_no_follow(package_root, filepath)
    with os.fdopen(descriptor, "rb") as source:
        before = os.fstat(source.fileno())
        _validate_open_source(filepath, before)
        before_signature = _source_signature(before)
        content_length = before.st_size
        if content_length > _MAX_FILE_BYTES or content_length > remaining_bytes:
            raise OSError("SOURCE_HASH_LIMIT_EXCEEDED")
        package_hash.update(content_length.to_bytes(8, "big"))

        bytes_read = 0
        for chunk in iter(lambda: source.read(8192), b""):
            package_hash.update(chunk)
            bytes_read += len(chunk)

        after = os.fstat(source.fileno())
        _validate_open_source(filepath, after)

    if bytes_read != content_length or _source_signature(after) != before_signature:
        raise OSError("source changed while hashing")
    return bytes_read


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
    remaining_bytes = _MAX_PACKAGE_BYTES
    for filepath in files:
        relative_path = filepath.relative_to(package.path).as_posix()
        remaining_bytes -= _update_file_frame(
            package_hash,
            f"{package_root}/{relative_path}",
            filepath,
            package.path,
            remaining_bytes,
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

    # Hashing v1 frames each sorted UTF-8 package identity with the dependency's
    # decoded 32-byte digest. Including both byte lengths preserves boundaries,
    # while decoding the digest avoids hashing a presentation-specific hex
    # string. Missing or malformed digests fail closed instead of silently
    # producing a cache identity for incomplete graph state.
    dependencies_hash = hashlib.sha256()
    for dependency in sorted(transitive_deps):
        if dependency not in package_hashes:
            raise ValueError("missing SHA-256 dependency digest")
        dependency_name = dependency.encode("utf-8")
        dependency_digest = _decode_sha256_digest(
            package_hashes[dependency], "dependency"
        )
        dependencies_hash.update(len(dependency_name).to_bytes(8, "big"))
        dependencies_hash.update(dependency_name)
        dependencies_hash.update(len(dependency_digest).to_bytes(8, "big"))
        dependencies_hash.update(dependency_digest)
    return dependencies_hash.hexdigest()


def _decode_sha256_digest(digest: str, role: str) -> bytes:
    """Decode one canonical lowercase SHA-256 digest without echoing its value."""
    if (
        not isinstance(digest, str)
        or len(digest) != 64
        or any(character not in "0123456789abcdef" for character in digest)
    ):
        raise ValueError(f"invalid SHA-256 {role} digest")
    return bytes.fromhex(digest)


def combine_hashes(package_digest: str, dependencies_digest: str) -> str:
    """Return the hashing-v1 cache digest for package and dependency state.

    The two inputs are canonical lowercase SHA-256 hex strings. Hashing v1
    combines their decoded 32-byte values directly, with the package digest
    first, rather than hashing the 128-byte textual representation.
    """
    package_bytes = _decode_sha256_digest(package_digest, "package")
    dependency_bytes = _decode_sha256_digest(dependencies_digest, "dependency")
    return hashlib.sha256(package_bytes + dependency_bytes).hexdigest()
