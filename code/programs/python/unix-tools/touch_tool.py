"""touch — change file timestamps or create empty files.

=== What This Program Does ===

This is a reimplementation of the GNU ``touch`` utility. It has two
main functions:

1. **Create files**: If a file doesn't exist, touch creates it as an
   empty file. This is probably the most common use of touch.

2. **Update timestamps**: If a file already exists, touch updates its
   access time and/or modification time to the current time (or a
   specified time).

=== File Timestamps Explained ===

Every file on a Unix system has (at least) three timestamps:

- **atime** (access time): When the file was last read.
- **mtime** (modification time): When the file contents were last changed.
- **ctime** (change time): When the file metadata (permissions, owner,
  etc.) was last changed. This one cannot be set directly by touch.

By default, touch updates both atime and mtime. The ``-a`` flag updates
only atime, and ``-m`` updates only mtime.

=== Specifying a Time ===

You can set timestamps to something other than "now":

- ``-t [[CC]YY]MMDDhhmm[.ss]``: A precise timestamp in a compact format.
- ``-d STRING``: An ISO 8601 date string.
- ``-r FILE``: Copy timestamps from another file.

=== The -c Flag (No Create) ===

With ``-c``, touch will not create files that don't exist. It only
updates timestamps of existing files. This is useful in scripts where
you want to refresh timestamps without accidentally creating files.

=== CLI Builder Integration ===

The entire CLI is defined in ``touch.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import sys
import time
from datetime import datetime, timezone
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "touch.json")


def parse_timestamp(stamp: str) -> float | None:
    """Parse a touch-style timestamp: [[CC]YY]MMDDhhmm[.ss].

    The format is quite specific:
    - MMDDhhmm: month, day, hour, minute (8 digits)
    - YYMMDDhhmm: 2-digit year + above (10 digits)
    - CCYYMMDDhhmm: 4-digit year + above (12 digits)
    - Any of the above can end with .ss (seconds)

    Args:
        stamp: The timestamp string to parse.

    Returns:
        Unix epoch time as a float, or None if parsing fails.
    """
    # Split off optional seconds.
    if "." in stamp:
        main_part, sec_part = stamp.rsplit(".", 1)
        try:
            seconds = int(sec_part)
        except ValueError:
            return None
    else:
        main_part = stamp
        seconds = 0

    now = datetime.now()

    try:
        if len(main_part) == 8:
            # MMDDhhmm — use current year.
            dt = datetime(
                now.year,
                int(main_part[0:2]),
                int(main_part[2:4]),
                int(main_part[4:6]),
                int(main_part[6:8]),
                seconds,
            )
        elif len(main_part) == 10:
            # YYMMDDhhmm — 2-digit year.
            year = int(main_part[0:2])
            # POSIX: 69-99 => 1969-1999, 00-68 => 2000-2068.
            year = year + 1900 if year >= 69 else year + 2000
            dt = datetime(
                year,
                int(main_part[2:4]),
                int(main_part[4:6]),
                int(main_part[6:8]),
                int(main_part[8:10]),
                seconds,
            )
        elif len(main_part) == 12:
            # CCYYMMDDhhmm — 4-digit year.
            dt = datetime(
                int(main_part[0:4]),
                int(main_part[4:6]),
                int(main_part[6:8]),
                int(main_part[8:10]),
                int(main_part[10:12]),
                seconds,
            )
        else:
            return None
    except ValueError:
        return None

    return dt.timestamp()


def parse_date_string(date_str: str) -> float | None:
    """Parse an ISO 8601-ish date string.

    Supports common formats:
    - ``2024-01-15``
    - ``2024-01-15 10:30:00``
    - ``2024-01-15T10:30:00``

    Args:
        date_str: The date string to parse.

    Returns:
        Unix epoch time as a float, or None if parsing fails.
    """
    # Try several common formats.
    formats = [
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d",
    ]

    for fmt in formats:
        try:
            dt = datetime.strptime(date_str, fmt)
            return dt.timestamp()
        except ValueError:
            continue

    return None


def touch_file(
    filepath: str,
    *,
    no_create: bool,
    access_only: bool,
    modify_only: bool,
    timestamp: float | None,
) -> bool:
    """Touch a file: create it or update its timestamps.

    Args:
        filepath: The path to the file.
        no_create: If True, don't create the file if it doesn't exist.
        access_only: If True, only update the access time.
        modify_only: If True, only update the modification time.
        timestamp: The time to set, or None for the current time.

    Returns:
        True on success, False on failure.
    """
    path = Path(filepath)

    # If the file doesn't exist, create it (unless -c is specified).
    if not path.exists():
        if no_create:
            return True  # Not an error; just skip.
        try:
            path.touch()
        except PermissionError:
            print(
                f"touch: cannot touch '{filepath}': Permission denied",
                file=sys.stderr,
            )
            return False
        except FileNotFoundError:
            print(
                f"touch: cannot touch '{filepath}': No such file or directory",
                file=sys.stderr,
            )
            return False

    # Determine the new timestamps.
    # os.utime takes (atime, mtime). If we pass None, it uses the current time.
    try:
        current_stat = os.stat(filepath)
    except OSError as e:
        print(f"touch: cannot touch '{filepath}': {e.strerror}", file=sys.stderr)
        return False

    now = timestamp if timestamp is not None else time.time()

    if access_only:
        new_atime = now
        new_mtime = current_stat.st_mtime
    elif modify_only:
        new_atime = current_stat.st_atime
        new_mtime = now
    else:
        new_atime = now
        new_mtime = now

    try:
        os.utime(filepath, (new_atime, new_mtime))
    except PermissionError:
        print(
            f"touch: cannot touch '{filepath}': Permission denied",
            file=sys.stderr,
        )
        return False

    return True


def main() -> None:
    """Entry point: parse args via CLI Builder, then touch files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"touch: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    assert isinstance(result, ParseResult)

    access_only = result.flags.get("access_only", False)
    modify_only = result.flags.get("modify_only", False)
    no_create = result.flags.get("no_create", False)
    timestamp_str = result.flags.get("timestamp")
    date_str = result.flags.get("date")
    reference = result.flags.get("reference")

    # Determine the timestamp to use.
    timestamp: float | None = None

    if reference is not None:
        try:
            ref_stat = os.stat(reference)
            timestamp = ref_stat.st_mtime
        except FileNotFoundError:
            print(
                f"touch: failed to get attributes of '{reference}': "
                "No such file or directory",
                file=sys.stderr,
            )
            raise SystemExit(1) from None

    if timestamp_str is not None:
        timestamp = parse_timestamp(timestamp_str)
        if timestamp is None:
            print(f"touch: invalid date format '{timestamp_str}'", file=sys.stderr)
            raise SystemExit(1) from None

    if date_str is not None:
        timestamp = parse_date_string(date_str)
        if timestamp is None:
            print(f"touch: invalid date format '{date_str}'", file=sys.stderr)
            raise SystemExit(1) from None

    # Get the list of files.
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    exit_code = 0
    for filepath in files:
        if not touch_file(
            filepath,
            no_create=no_create,
            access_only=access_only,
            modify_only=modify_only,
            timestamp=timestamp,
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
