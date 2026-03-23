"""true — do nothing, successfully.

=== What This Program Does ===

This is a reimplementation of the POSIX ``true`` utility. It does absolutely
nothing and exits with status code 0 (success). That's it. That's the whole
program.

=== Why Does This Exist? ===

At first glance, a program that does nothing seems pointless. But ``true`` is
a fundamental building block in shell scripting. It's used in:

1. **Infinite loops**: ``while true; do ... done``
2. **Default success commands**: ``COMMAND || true`` (ignore failures)
3. **Placeholder commands**: When a script needs a command that always succeeds

For example, in a Makefile you might write::

    clean:
        rm -f *.o || true

The ``|| true`` ensures that the ``clean`` target succeeds even if there are
no ``.o`` files to remove. Without it, ``rm`` would fail and ``make`` would
report an error.

=== CLI Builder Integration ===

Even though ``true`` takes no meaningful arguments, we still wire it through
CLI Builder so that ``true --help`` and ``true --version`` work correctly.
Any other arguments are silently ignored (per POSIX behavior), but the JSON
spec doesn't define any flags or arguments beyond the builtins.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "true.json")


def main() -> None:
    """Entry point: parse args via CLI Builder, then exit 0.

    The ``true`` utility must always exit with status 0, regardless of
    what arguments are passed. The only exceptions are ``--help`` and
    ``--version``, which print their respective output and then exit 0
    as well.
    """
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    # Hand the spec file and sys.argv to CLI Builder. Even though true
    # ignores all arguments, we still parse them so --help and --version
    # work correctly.
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors:
        # POSIX true ignores all errors — it always succeeds.
        # But we still exit 0 because that's what true does.
        raise SystemExit(0) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    # CLI Builder returns one of:
    #   - HelpResult:    user passed --help
    #   - VersionResult: user passed --version
    #   - ParseResult:   normal invocation; nothing to do

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # The business logic of ``true`` is: do nothing. Exit successfully.
    # This is the simplest possible program — a no-op that returns 0.

    assert isinstance(result, ParseResult)
    raise SystemExit(0)


if __name__ == "__main__":
    main()
