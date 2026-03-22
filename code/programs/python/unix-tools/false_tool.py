"""false — do nothing, unsuccessfully.

=== What This Program Does ===

This is a reimplementation of the POSIX ``false`` utility. It does absolutely
nothing and exits with status code 1 (failure). It is the mirror image of
``true``.

=== Why Does This Exist? ===

Like ``true``, ``false`` is a shell scripting building block. It's used in:

1. **Conditional testing**: ``if false; then ... fi`` (the body never runs)
2. **Disabling features**: ``ENABLE_FEATURE=false; if $ENABLE_FEATURE; ...``
3. **Forcing failure**: When you need a command that always fails

A common use case is temporarily disabling a block of code in a shell script::

    if false; then
        echo "This code is disabled"
        expensive_operation
    fi

This is the shell equivalent of commenting out code — ``false`` ensures the
block never executes, but the code remains visible and syntactically valid.

=== CLI Builder Integration ===

Just like ``true``, we wire ``false`` through CLI Builder so that
``false --help`` and ``false --version`` work. The key difference: after
printing help or version info, ``--help`` and ``--version`` exit 0 (because
the help request itself succeeded), but a bare invocation of ``false`` exits 1.
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

SPEC_FILE = str(Path(__file__).parent / "false.json")


def main() -> None:
    """Entry point: parse args via CLI Builder, then exit 1.

    The ``false`` utility must always exit with status 1, regardless of
    what arguments are passed. The only exceptions are ``--help`` and
    ``--version``, which print their respective output and then exit 0
    (because the help/version request itself succeeded).
    """
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    # Hand the spec file and sys.argv to CLI Builder. Even though false
    # ignores all arguments, we still parse them so --help and --version
    # work correctly.
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors:
        # POSIX false ignores all errors — it always fails.
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    # CLI Builder returns one of:
    #   - HelpResult:    user passed --help (exits 0 — the request succeeded)
    #   - VersionResult: user passed --version (exits 0 — the request succeeded)
    #   - ParseResult:   normal invocation; exit with failure

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # The business logic of ``false`` is: do nothing. Exit with failure.
    # This is the counterpart to ``true`` — a no-op that returns 1.

    assert isinstance(result, ParseResult)
    raise SystemExit(1)


if __name__ == "__main__":
    main()
