"""du — estimate file space usage.

=== What This Program Does ===

This is a reimplementation of the GNU ``du`` utility. It estimates the
disk space used by files and directories.

=== How du Works ===

``du`` walks the directory tree starting from each specified path. For
each directory, it sums up the sizes of all files within it (including
files in subdirectories) and reports the total.

By default, ``du`` reports sizes in 1024-byte blocks (1K blocks) and
only shows directories (not individual files).

=== Common Usage Patterns ===

::

    du                    # Show all directories under current dir
    du -s /home           # Show only the total for /home
    du -sh /home          # Show total in human-readable format
    du -a                 # Show files too, not just directories
    du -d 1               # Show only top-level subdirectories
    du -c /home /var      # Show totals plus a grand total

=== The -s (summarize) Flag ===

With ``-s``, only the total for each top-level argument is shown.
This is equivalent to ``-d 0`` (max depth of 0).

=== Human-Readable Output ===

With ``-h``, sizes are displayed with appropriate suffixes::

    4.0K    ./small_dir
    1.2M    ./medium_dir
    2.5G    ./large_dir

=== Implementation Notes ===

We use ``os.walk()`` to traverse directories and ``os.path.getsize()``
(or ``os.lstat().st_size``) to get file sizes. We use ``lstat`` to
avoid following symbolic links by default.

=== CLI Builder Integration ===

The entire CLI is defined in ``du.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "du.json")


# ---------------------------------------------------------------------------
# Helper: human-readable sizes
# ---------------------------------------------------------------------------


def format_size(size_bytes: int, *, human: bool = False, si: bool = False) -> str:
    """Format a byte count for display.

    Args:
        size_bytes: The size in bytes.
        human: If True, use powers of 1024 (K, M, G, T).
        si: If True, use powers of 1000 (K, M, G, T).

    Returns:
        The formatted size string. In default mode, returns 1K-blocks.
    """
    if not human and not si:
        # Default: 1K blocks.
        blocks = size_bytes // 1024
        return str(blocks if blocks > 0 else (1 if size_bytes > 0 else 0))

    base = 1000 if si else 1024
    suffixes = ["", "K", "M", "G", "T", "P"]

    if size_bytes == 0:
        return "0"

    value = float(size_bytes)
    idx = 0
    while value >= base and idx < len(suffixes) - 1:
        value /= base
        idx += 1

    if idx == 0:
        return str(size_bytes)
    if value >= 10:
        return f"{value:.0f}{suffixes[idx]}"
    return f"{value:.1f}{suffixes[idx]}"


# ---------------------------------------------------------------------------
# Business logic: disk_usage
# ---------------------------------------------------------------------------


def disk_usage(
    path: str,
    *,
    all_files: bool = False,
    summarize: bool = False,
    max_depth: int | None = None,
    dereference: bool = False,
) -> list[tuple[int, str]]:
    """Calculate disk usage for a path.

    Walks the directory tree rooted at ``path`` and returns a list of
    (size_in_bytes, path) tuples for each directory (and optionally
    each file).

    Args:
        path: The starting directory or file path.
        all_files: If True, include individual files in output, not
                   just directories.
        summarize: If True, only return the total for the top-level path.
                   Equivalent to max_depth=0.
        max_depth: Maximum depth of directories to show. None means
                   no limit.
        dereference: If True, follow symbolic links.

    Returns:
        A list of (size_bytes, path_string) tuples.
    """
    # Handle the case where path is a file, not a directory.
    if os.path.isfile(path):
        try:
            if dereference:
                size = os.stat(path).st_size
            else:
                size = os.lstat(path).st_size
        except OSError:
            size = 0
        return [(size, path)]

    if summarize:
        max_depth = 0

    result: list[tuple[int, str]] = []

    # We need to compute sizes bottom-up. os.walk with topdown=False
    # gives us leaves before parents.
    dir_sizes: dict[str, int] = {}

    stat_fn = os.stat if dereference else os.lstat

    try:
        for dirpath, dirnames, filenames in os.walk(path, followlinks=dereference):
            dir_size = 0

            # Sum up file sizes in this directory.
            for fname in filenames:
                fpath = os.path.join(dirpath, fname)
                try:
                    size = stat_fn(fpath).st_size
                except OSError:
                    size = 0
                dir_size += size

                # Optionally report individual files.
                if all_files and not summarize:
                    depth = _relative_depth(path, fpath)
                    if max_depth is None or depth <= max_depth:
                        result.append((size, fpath))

            # Add sizes of subdirectories (already computed if bottom-up,
            # but with topdown=True we'll accumulate after the walk).
            dir_sizes[dirpath] = dir_size

        # Now we need to accumulate subdirectory sizes bottom-up.
        # Sort directories by depth (deepest first) and propagate sizes up.
        all_dirs = sorted(dir_sizes.keys(), key=lambda d: d.count(os.sep), reverse=True)
        for d in all_dirs:
            parent = os.path.dirname(d)
            if parent in dir_sizes and parent != d:
                dir_sizes[parent] += dir_sizes[d]

        # Build directory entries in the result.
        dir_entries: list[tuple[int, str]] = []
        for d in dir_sizes:
            depth = _relative_depth(path, d)
            if summarize:
                if d == path:
                    dir_entries.append((dir_sizes[d], d))
            elif max_depth is not None:
                if depth <= max_depth:
                    dir_entries.append((dir_sizes[d], d))
            else:
                dir_entries.append((dir_sizes[d], d))

        # Combine file entries and directory entries, sorted by path.
        result.extend(dir_entries)
        result.sort(key=lambda x: x[1])

    except PermissionError:
        print(f"du: cannot read directory '{path}': Permission denied", file=sys.stderr)

    return result


def _relative_depth(base: str, target: str) -> int:
    """Calculate the directory depth of target relative to base.

    Returns 0 if target IS base, 1 if target is directly inside base, etc.
    """
    base_parts = os.path.normpath(base).split(os.sep)
    target_parts = os.path.normpath(target).split(os.sep)
    return len(target_parts) - len(base_parts)


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then report disk usage."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"du: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    all_files = result.flags.get("all", False)
    human = result.flags.get("human_readable", False)
    si = result.flags.get("si", False)
    summarize = result.flags.get("summarize", False)
    total = result.flags.get("total", False)
    max_depth_val = result.flags.get("max_depth")
    dereference = result.flags.get("dereference", False)

    files = result.arguments.get("files", ["."])
    if isinstance(files, str):
        files = [files]

    grand_total = 0
    try:
        for fpath in files:
            entries = disk_usage(
                fpath,
                all_files=all_files,
                summarize=summarize,
                max_depth=max_depth_val,
                dereference=dereference,
            )
            for size, name in entries:
                print(f"{format_size(size, human=human, si=si)}\t{name}")
                grand_total += size

        if total:
            print(f"{format_size(grand_total, human=human, si=si)}\ttotal")
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
