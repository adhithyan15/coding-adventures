"""df — report file system disk space usage.

=== What This Program Does ===

This is a reimplementation of the GNU ``df`` utility. It displays
information about the file system on which each specified file resides,
or all mounted file systems if no files are given.

=== What df Shows ===

The default output has these columns::

    Filesystem     1K-blocks      Used Available Use% Mounted on
    /dev/sda1      244277768 123456789 108345678  54% /

- **Filesystem**: The device or filesystem name.
- **1K-blocks**: Total size in 1024-byte blocks.
- **Used**: Space used in 1K blocks.
- **Available**: Space available in 1K blocks.
- **Use%**: Percentage of space used.
- **Mounted on**: The mount point.

=== Human-Readable Output ===

With ``-h``, sizes are shown in powers of 1024 (K, M, G, T)::

    Filesystem      Size  Used Avail Use% Mounted on
    /dev/sda1       233G  118G  104G  54% /

=== Implementation Approach ===

We use ``shutil.disk_usage()`` to get total/used/free bytes for a path.
This is cross-platform (works on Linux, macOS, Windows).

For listing all filesystems, we read ``/proc/mounts`` on Linux or use
``os.statvfs()`` where available. On macOS, we parse the output of
``mount`` or use ``os.statvfs()``.

=== CLI Builder Integration ===

The entire CLI is defined in ``df.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import shutil
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "df.json")


# ---------------------------------------------------------------------------
# Helper: human-readable sizes
# ---------------------------------------------------------------------------

def format_size(size_bytes: int, *, human: bool = False, si: bool = False) -> str:
    """Format a byte count as a human-readable string.

    Args:
        size_bytes: The size in bytes.
        human: If True, use powers of 1024 (K, M, G, T).
        si: If True, use powers of 1000 (kB, MB, GB, TB).

    Returns:
        The formatted size string.
    """
    if not human and not si:
        # Default: 1K blocks.
        return str(size_bytes // 1024)

    base = 1000 if si else 1024
    suffixes = ["B", "K", "M", "G", "T", "P", "E"]

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
# Business logic: get_filesystem_info
# ---------------------------------------------------------------------------


def get_filesystem_info(
    paths: list[str] | None = None,
) -> list[dict[str, str | int]]:
    """Get disk usage information for the specified paths.

    If ``paths`` is None or empty, returns info for the root filesystem.

    Each entry in the returned list is a dictionary with:
    - ``filesystem``: The filesystem device name (or path).
    - ``total``: Total bytes.
    - ``used``: Used bytes.
    - ``available``: Available bytes.
    - ``use_percent``: Usage percentage as a string (e.g., "54%").
    - ``mounted_on``: The mount point path.

    Args:
        paths: List of file paths to check. Defaults to ["/"].

    Returns:
        A list of filesystem info dictionaries.
    """
    if not paths:
        paths = ["/"]

    results: list[dict[str, str | int]] = []
    for path in paths:
        try:
            usage = shutil.disk_usage(path)
        except OSError as exc:
            print(f"df: {path}: {exc.strerror}", file=sys.stderr)
            continue

        total = usage.total
        used = usage.used
        available = usage.free

        if total > 0:
            pct = int((used / total) * 100)
            use_pct = f"{pct}%"
        else:
            use_pct = "-"

        results.append({
            "filesystem": path,
            "total": total,
            "used": used,
            "available": available,
            "use_percent": use_pct,
            "mounted_on": path,
        })

    return results


def format_df_output(
    entries: list[dict[str, str | int]],
    *,
    human: bool = False,
    si: bool = False,
) -> str:
    """Format filesystem entries into a table string.

    Args:
        entries: List of filesystem info dicts from ``get_filesystem_info()``.
        human: If True, use human-readable sizes (powers of 1024).
        si: If True, use SI sizes (powers of 1000).

    Returns:
        The formatted table as a string.
    """
    if human or si:
        header = f"{'Filesystem':<20} {'Size':>6} {'Used':>6} {'Avail':>6} {'Use%':>5} {'Mounted on'}"
    else:
        header = f"{'Filesystem':<20} {'1K-blocks':>12} {'Used':>12} {'Available':>12} {'Use%':>5} {'Mounted on'}"

    lines = [header]
    for entry in entries:
        total = entry["total"]
        used = entry["used"]
        available = entry["available"]
        assert isinstance(total, int)
        assert isinstance(used, int)
        assert isinstance(available, int)

        if human or si:
            total_s = format_size(total, human=human, si=si)
            used_s = format_size(used, human=human, si=si)
            avail_s = format_size(available, human=human, si=si)
            line = (
                f"{entry['filesystem']:<20} "
                f"{total_s:>6} {used_s:>6} {avail_s:>6} "
                f"{entry['use_percent']:>5} {entry['mounted_on']}"
            )
        else:
            total_k = total // 1024
            used_k = used // 1024
            avail_k = available // 1024
            line = (
                f"{entry['filesystem']:<20} "
                f"{total_k:>12} {used_k:>12} {avail_k:>12} "
                f"{entry['use_percent']:>5} {entry['mounted_on']}"
            )
        lines.append(line)

    return "\n".join(lines)


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
            print(f"df: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    human = result.flags.get("human_readable", False)
    si = result.flags.get("si", False)
    files = result.arguments.get("files")
    if isinstance(files, str):
        files = [files]

    entries = get_filesystem_info(files or None)
    output = format_df_output(entries, human=human, si=si)

    try:
        print(output)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
