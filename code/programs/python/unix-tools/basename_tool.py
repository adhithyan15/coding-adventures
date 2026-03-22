"""basename — strip directory and suffix from filenames.

=== What This Program Does ===

This is a reimplementation of the GNU ``basename`` utility. Given a pathname,
it strips the directory prefix and optionally strips a trailing suffix.

For example::

    $ basename /usr/bin/python3
    python3

    $ basename /usr/bin/python3 3
    python

    $ basename -s .py /home/user/script.py
    script

=== Two Invocation Modes ===

GNU basename has two invocation styles:

1. **Single-argument mode** (traditional):
   ``basename NAME [SUFFIX]``

   The first argument is the pathname. If a second argument is given,
   it's treated as a suffix to remove. For example:
   ``basename /path/to/file.txt .txt`` prints ``file``.

2. **Multiple-argument mode** (``-a`` or ``-s``):
   ``basename -a NAME...`` or ``basename -s SUFFIX NAME...``

   Process each NAME independently. The ``-s`` flag implies ``-a`` and
   specifies the suffix to remove from all names.

=== How Suffix Stripping Works ===

Suffix removal is a simple string operation: if the basename ends with
the given suffix (and the suffix is not the entire basename), the suffix
is removed. This matches POSIX behavior::

    basename("archive.tar.gz", ".tar.gz")  ->  "archive"
    basename("archive.tar.gz", ".gz")      ->  "archive.tar"
    basename(".gz", ".gz")                 ->  ".gz"  (suffix IS the name)

=== CLI Builder Integration ===

The entire CLI is defined in ``basename.json``. CLI Builder handles flag
parsing, help text, and version output. This file implements the directory
stripping and suffix removal logic.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "basename.json")


def strip_basename(name: str, suffix: str = "") -> str:
    """Strip directory components and optionally a suffix from a pathname.

    The algorithm follows POSIX rules:

    1. Strip trailing slashes (so ``/foo/bar/`` becomes ``/foo/bar``).
    2. Take only the last component (everything after the final ``/``).
    3. If a suffix is given and the basename ends with it (and the suffix
       is not the entire basename), remove the suffix.

    Args:
        name: The full pathname to process.
        suffix: Optional suffix to strip from the result.

    Returns:
        The basename with directory and optional suffix removed.
    """
    # os.path.basename handles trailing slashes and path separation.
    # For example: os.path.basename("/foo/bar/") returns "" in Python,
    # but GNU basename returns "bar". So we strip trailing slashes first.
    cleaned = name.rstrip("/") if name != "/" else name
    base = os.path.basename(cleaned) if cleaned else ""

    # Handle the root case: basename of "/" is "/".
    if not base and name:
        base = "/"

    # Strip the suffix if it matches and isn't the entire basename.
    if suffix and base.endswith(suffix) and base != suffix:
        base = base[: -len(suffix)]

    return base


def main() -> None:
    """Entry point: parse args via CLI Builder, then strip basenames."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"basename: {error.message}", file=sys.stderr)
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

    multiple = result.flags.get("multiple", False)
    suffix = result.flags.get("suffix", "")
    zero = result.flags.get("zero", False)

    # -s implies -a (multiple mode).
    if suffix:
        multiple = True

    # Get the names to process.
    names = result.arguments.get("name", [])
    if isinstance(names, str):
        names = [names]

    # Determine the line terminator.
    terminator = "\0" if zero else "\n"

    if multiple:
        # Multiple mode: process each name independently.
        for name in names:
            print(strip_basename(name, suffix), end=terminator)
    else:
        # Traditional mode: first arg is NAME, optional second is SUFFIX.
        if len(names) == 1:
            print(strip_basename(names[0]), end=terminator)
        elif len(names) == 2:  # noqa: PLR2004
            print(strip_basename(names[0], names[1]), end=terminator)
        else:
            # More than 2 args without -a: treat as multiple mode.
            for name in names:
                print(strip_basename(name), end=terminator)


if __name__ == "__main__":
    main()
