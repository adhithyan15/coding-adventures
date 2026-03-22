"""dirname — strip last component from file name.

=== What This Program Does ===

This is a reimplementation of the GNU ``dirname`` utility. Given a pathname,
it strips the last component (the filename), leaving just the directory path.

For example::

    $ dirname /usr/bin/python3
    /usr/bin

    $ dirname python3
    .

    $ dirname /usr/bin/
    /usr

=== How It Works ===

The algorithm follows POSIX rules:

1. If the string is ``//``, it may be implementation-defined (we return ``/``).
2. Strip trailing slashes.
3. If no slashes remain, the result is ``.`` (current directory).
4. Strip everything after the last slash.
5. Strip trailing slashes from the result.
6. If nothing remains, the result is ``/``.

Python's ``os.path.dirname`` handles most of these cases, but we need
special handling for trailing slashes (e.g., ``dirname /usr/bin/`` should
return ``/usr``, not ``/usr/bin``).

=== The "dot" Case ===

When the input has no directory component (like a bare filename), dirname
returns ``.``, meaning "the current directory." This is consistent with
how Unix paths work: a bare filename implies the current directory::

    $ dirname myfile.txt
    .

=== CLI Builder Integration ===

The entire CLI is defined in ``dirname.json``. CLI Builder handles flag
parsing, help text, and version output. This file implements only the
directory extraction logic.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "dirname.json")


def get_dirname(name: str) -> str:
    """Extract the directory component from a pathname.

    This function implements POSIX dirname semantics:

    - ``/usr/bin/python`` -> ``/usr/bin``
    - ``/usr/bin/`` -> ``/usr``
    - ``python`` -> ``.``
    - ``/`` -> ``/``
    - ``//`` -> ``/`` (or ``//`` on some systems, we normalize to ``/``)

    The key insight is that we need to strip trailing slashes before
    calling os.path.dirname, because Python's os.path.dirname treats
    a trailing slash as indicating a directory (so it returns the path
    unchanged).

    Args:
        name: The pathname to process.

    Returns:
        The directory component of the pathname.
    """
    # Strip trailing slashes (but not all of them — keep at least one
    # if the entire path is slashes).
    if name != "/" and name != "//":
        name = name.rstrip("/")

    result = os.path.dirname(name)

    # If os.path.dirname returns empty string, the input was a bare
    # filename with no directory component. POSIX says return ".".
    if not result:
        return "."

    return result


def main() -> None:
    """Entry point: parse args via CLI Builder, then extract directory names."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"dirname: {error.message}", file=sys.stderr)
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

    zero = result.flags.get("zero", False)
    terminator = "\0" if zero else "\n"

    # Get the names to process.
    names = result.arguments.get("names", [])
    if isinstance(names, str):
        names = [names]

    for name in names:
        print(get_dirname(name), end=terminator)


if __name__ == "__main__":
    main()
