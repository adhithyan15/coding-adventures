"""tar -- an archiving utility.

=== What This Program Does ===

This is a reimplementation of the GNU ``tar`` utility. It creates,
extracts, and lists tape archive (tar) files::

    tar -cf archive.tar file1 file2    # Create archive
    tar -xf archive.tar                # Extract archive
    tar -tf archive.tar                # List contents
    tar -czf archive.tar.gz dir/       # Create gzip-compressed archive
    tar -xzf archive.tar.gz            # Extract gzip-compressed archive

=== History of tar ===

``tar`` stands for "tape archive" — it was originally designed to write
files to magnetic tape drives for backup. The format concatenates files
together with metadata headers, producing a single "archive" file.

Even though tape drives are rare today, the tar format remains the
standard way to bundle files on Unix systems. Combined with compression
(gzip, bzip2, xz), it's the basis for ``.tar.gz`` and ``.tar.bz2``
distribution archives.

=== How tar Archives Work ===

A tar archive is a sequence of 512-byte blocks. Each file entry has:

1. A **header block** containing metadata (filename, size, permissions,
   owner, modification time, etc.)
2. **Data blocks** containing the file's contents (padded to 512 bytes)

The archive ends with two blocks of zeros.

Python's ``tarfile`` module handles all the low-level details of
reading and writing these blocks.

=== Compression ===

tar itself doesn't compress — it just concatenates. Compression is
handled by piping through an external program:

+------+-------------+-------------------+
| Flag | Program     | File extension    |
+======+=============+===================+
| -z   | gzip        | .tar.gz / .tgz    |
| -j   | bzip2       | .tar.bz2          |
| -J   | xz          | .tar.xz           |
+------+-------------+-------------------+

Python's ``tarfile`` module supports all three natively.

=== Operations ===

+------+--------------------------------------------------+
| Flag | Operation                                        |
+======+==================================================+
| -c   | Create a new archive                             |
| -x   | Extract files from an archive                    |
| -t   | List the contents of an archive                  |
| -r   | Append files to an existing archive               |
| -u   | Update — append only files newer than in archive  |
+------+--------------------------------------------------+

=== CLI Builder Integration ===

The CLI is defined in ``tar.json``. CLI Builder handles flag parsing.
"""

from __future__ import annotations

import fnmatch
import os
import stat
import sys
import tarfile
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "tar.json")


# ---------------------------------------------------------------------------
# Determine the tarfile open mode based on flags.
# ---------------------------------------------------------------------------


def _get_mode(
    operation: str,
    *,
    gzip: bool = False,
    bzip2: bool = False,
    xz: bool = False,
) -> str:
    """Determine the tarfile open mode string.

    The mode string combines the operation with the compression type.

    +----+--------+--------+--------+----------+
    | Op | None   | gzip   | bzip2  | xz       |
    +====+========+========+========+==========+
    | c  | w      | w:gz   | w:bz2  | w:xz     |
    | x  | r      | r:gz   | r:bz2  | r:xz     |
    | t  | r      | r:gz   | r:bz2  | r:xz     |
    | r  | a      | (err)  | (err)  | (err)    |
    +----+--------+--------+--------+----------+

    Args:
        operation: One of "create", "extract", "list", "append".
        gzip: Use gzip compression.
        bzip2: Use bzip2 compression.
        xz: Use xz compression.

    Returns:
        The tarfile mode string.
    """
    if operation == "create":
        base = "w"
    elif operation in ("extract", "list"):
        base = "r"
    elif operation == "append":
        base = "a"
    else:
        base = "r"

    if gzip:
        return f"{base}:gz"
    elif bzip2:
        return f"{base}:bz2"
    elif xz:
        return f"{base}:xz"
    return base


# ---------------------------------------------------------------------------
# Create an archive.
# ---------------------------------------------------------------------------


def create_archive(
    archive_path: str,
    files: list[str],
    *,
    gzip: bool = False,
    bzip2: bool = False,
    xz: bool = False,
    verbose: bool = False,
    directory: str | None = None,
    exclude_patterns: list[str] | None = None,
) -> bool:
    """Create a new tar archive.

    Args:
        archive_path: Path for the new archive file.
        files: List of files/directories to include.
        gzip: Compress with gzip.
        bzip2: Compress with bzip2.
        xz: Compress with xz.
        verbose: Print each file as it's added.
        directory: Change to this directory before archiving.
        exclude_patterns: Glob patterns to exclude.

    Returns:
        True on success, False on error.
    """
    mode = _get_mode("create", gzip=gzip, bzip2=bzip2, xz=xz)
    original_dir = os.getcwd()

    try:
        if directory:
            os.chdir(directory)

        def _filter(info: tarfile.TarInfo) -> tarfile.TarInfo | None:
            """Filter function to exclude matching patterns."""
            if exclude_patterns:
                for pattern in exclude_patterns:
                    if fnmatch.fnmatch(info.name, pattern):
                        return None
                    # Also check just the basename.
                    if fnmatch.fnmatch(os.path.basename(info.name), pattern):
                        return None
            if verbose:
                print(info.name)
            return info

        # If archive_path is relative, resolve it relative to original dir.
        if not os.path.isabs(archive_path) and directory:
            archive_path = os.path.join(original_dir, archive_path)

        with tarfile.open(archive_path, mode) as tar:
            for filepath in files:
                if not os.path.exists(filepath):
                    print(
                        f"tar: {filepath}: Cannot stat: No such file or directory",
                        file=sys.stderr,
                    )
                    continue
                tar.add(filepath, filter=_filter)

        return True

    except PermissionError as e:
        print(f"tar: {e}", file=sys.stderr)
        return False
    except OSError as e:
        print(f"tar: {e}", file=sys.stderr)
        return False
    finally:
        os.chdir(original_dir)


# ---------------------------------------------------------------------------
# Extract an archive.
# ---------------------------------------------------------------------------


def extract_archive(
    archive_path: str,
    files: list[str] | None = None,
    *,
    gzip: bool = False,
    bzip2: bool = False,
    xz: bool = False,
    verbose: bool = False,
    directory: str | None = None,
    keep_old_files: bool = False,
    strip_components: int | None = None,
) -> bool:
    """Extract files from a tar archive.

    Args:
        archive_path: Path to the archive file.
        files: Specific files to extract (None = all).
        gzip: Archive is gzip-compressed.
        bzip2: Archive is bzip2-compressed.
        xz: Archive is xz-compressed.
        verbose: Print each file as it's extracted.
        directory: Extract into this directory.
        keep_old_files: Don't overwrite existing files.
        strip_components: Strip N leading path components.

    Returns:
        True on success, False on error.
    """
    mode = _get_mode("extract", gzip=gzip, bzip2=bzip2, xz=xz)
    extract_dir = directory or "."

    try:
        with tarfile.open(archive_path, mode) as tar:
            members = tar.getmembers()

            # Filter to specific files if requested.
            if files:
                members = [m for m in members if m.name in files]

            # Strip leading path components.
            if strip_components:
                for member in members:
                    parts = member.name.split("/")
                    if len(parts) > strip_components:
                        member.name = "/".join(parts[strip_components:])
                    else:
                        member.name = ""
                members = [m for m in members if m.name]

            for member in members:
                target_path = os.path.join(extract_dir, member.name)

                # Keep old files: skip if target exists.
                if keep_old_files and os.path.exists(target_path):
                    if verbose:
                        print(f"tar: {member.name}: Already exists, skipping")
                    continue

                if verbose:
                    print(member.name)

                tar.extract(member, path=extract_dir, filter="data")

        return True

    except FileNotFoundError:
        print(
            f"tar: {archive_path}: Cannot open: No such file or directory",
            file=sys.stderr,
        )
        return False
    except tarfile.ReadError as e:
        print(f"tar: {archive_path}: {e}", file=sys.stderr)
        return False
    except PermissionError as e:
        print(f"tar: {e}", file=sys.stderr)
        return False
    except OSError as e:
        print(f"tar: {e}", file=sys.stderr)
        return False


# ---------------------------------------------------------------------------
# List archive contents.
# ---------------------------------------------------------------------------


def list_archive(
    archive_path: str,
    files: list[str] | None = None,
    *,
    gzip: bool = False,
    bzip2: bool = False,
    xz: bool = False,
    verbose: bool = False,
) -> tuple[bool, list[str]]:
    """List the contents of a tar archive.

    Args:
        archive_path: Path to the archive file.
        files: Specific files to list (None = all).
        gzip: Archive is gzip-compressed.
        bzip2: Archive is bzip2-compressed.
        xz: Archive is xz-compressed.
        verbose: Show detailed information.

    Returns:
        A tuple of (success, list_of_entries).
    """
    mode = _get_mode("list", gzip=gzip, bzip2=bzip2, xz=xz)
    entries: list[str] = []

    try:
        with tarfile.open(archive_path, mode) as tar:
            members = tar.getmembers()

            if files:
                members = [m for m in members if m.name in files]

            for member in members:
                if verbose:
                    # Detailed listing similar to ls -l.
                    perms = stat.filemode(member.mode)
                    import time as time_mod
                    local_time = time_mod.localtime(member.mtime)
                    mtime = time_mod.strftime("%Y-%m-%d %H:%M", local_time)
                    entries.append(
                        f"{perms} {member.uname}/{member.gname} "
                        f"{member.size:>8d} {mtime} {member.name}"
                    )
                else:
                    entries.append(member.name)

        return True, entries

    except FileNotFoundError:
        return False, [f"tar: {archive_path}: Cannot open: No such file or directory"]
    except tarfile.ReadError as e:
        return False, [f"tar: {archive_path}: {e}"]
    except OSError as e:
        return False, [f"tar: {e}"]


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then run tar."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"tar: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # --- Extract flags ---
    create = result.flags.get("create", False)
    extract = result.flags.get("extract", False)
    list_op = result.flags.get("list", False)
    archive_file = result.flags.get("file", None)
    verbose = result.flags.get("verbose", False)
    use_gzip = result.flags.get("gzip", False)
    use_bzip2 = result.flags.get("bzip2", False)
    use_xz = result.flags.get("xz", False)
    directory = result.flags.get("directory", None)
    keep_old = result.flags.get("keep_old_files", False)
    strip = result.flags.get("strip_components", None)
    exclude = result.flags.get("exclude", None)
    if isinstance(exclude, list):
        exclude_patterns = exclude
    else:
        exclude_patterns = [exclude] if exclude else None

    # --- Extract arguments ---
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    # --- Determine archive path ---
    if not archive_file:
        print("tar: Refusing to read/write archive to terminal", file=sys.stderr)
        raise SystemExit(2)

    # --- Dispatch to operation ---
    if create:
        success = create_archive(
            archive_file, files,
            gzip=use_gzip,
            bzip2=use_bzip2,
            xz=use_xz,
            verbose=verbose,
            directory=directory,
            exclude_patterns=exclude_patterns,
        )
        raise SystemExit(0 if success else 2)

    elif extract:
        success = extract_archive(
            archive_file,
            files if files else None,
            gzip=use_gzip,
            bzip2=use_bzip2,
            xz=use_xz,
            verbose=verbose,
            directory=directory,
            keep_old_files=keep_old,
            strip_components=strip,
        )
        raise SystemExit(0 if success else 2)

    elif list_op:
        success, entries = list_archive(
            archive_file,
            files if files else None,
            gzip=use_gzip,
            bzip2=use_bzip2,
            xz=use_xz,
            verbose=verbose,
        )
        for entry in entries:
            print(entry)
        raise SystemExit(0 if success else 2)

    else:
        print("tar: You must specify one of -c, -x, -t", file=sys.stderr)
        raise SystemExit(2)


if __name__ == "__main__":
    main()
