"""pwd — print the absolute pathname of the current working directory.

=== What This Program Does ===

This is a reimplementation of the POSIX ``pwd`` utility. It prints the
absolute path of the current working directory to standard output.

=== How CLI Builder Powers This ===

The entire command-line interface — flags, help text, version output,
error messages — is defined in ``pwd.json``. This program never parses
a single argument by hand. Instead:

1. We hand ``pwd.json`` and ``sys.argv`` to CLI Builder's ``Parser``.
2. The parser validates the input, enforces mutual exclusivity of
   ``-L`` and ``-P``, generates help text, and returns a typed result.
3. We pattern-match on the result type and run the business logic.

The result is that *this file contains only business logic*. All parsing,
validation, and help generation happen inside CLI Builder, driven by the
JSON spec.

=== Logical vs Physical Paths ===

When you ``cd`` through a symbolic link, the shell updates the ``$PWD``
environment variable to reflect the path *as you typed it* — including
the symlink. This is the "logical" path.

The "physical" path resolves all symlinks. For example, if ``/home`` is
a symlink to ``/usr/home``:

    Logical:  /home/user       (what $PWD says)
    Physical: /usr/home/user   (what the filesystem says)

By default (``-L``), we print the logical path. With ``-P``, we resolve
symlinks and print the physical path.

=== POSIX Compliance Note ===

If ``$PWD`` is not set, or if it doesn't match the actual current
directory, even ``-L`` mode falls back to the physical path. This
matches POSIX behavior.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "pwd.json")


def get_logical_pwd() -> str:
    """Return the logical working directory.

    The logical path comes from the ``$PWD`` environment variable, which
    the shell maintains as the user navigates — including through symlinks.

    If ``$PWD`` is not set or is stale (doesn't match the real cwd), we
    fall back to the physical path. This matches POSIX behavior: the
    logical path is best-effort, never wrong.
    """
    env_pwd = os.environ.get("PWD")

    if env_pwd is not None:
        # Verify that $PWD actually points to the current directory.
        # It could be stale if the directory was moved/deleted, or if
        # the process changed directories without updating $PWD.
        try:
            env_real = os.path.realpath(env_pwd)
            cwd_real = os.path.realpath(".")
            if env_real == cwd_real:
                return env_pwd
        except OSError:
            pass

    # Fallback: resolve the physical path.
    return str(Path.cwd().resolve())


def get_physical_pwd() -> str:
    """Return the physical working directory with all symlinks resolved.

    This calls ``pathlib.Path.cwd().resolve()``, which follows every
    symlink in the path to produce the canonical filesystem path.
    """
    return str(Path.cwd().resolve())


def main() -> None:
    """Entry point: parse args via CLI Builder, then print the cwd."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    # Hand the spec file and sys.argv to CLI Builder. The parser reads the
    # JSON spec, validates the flags, enforces mutual exclusivity, and
    # returns one of three result types.
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"pwd: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    # CLI Builder returns one of:
    #   - HelpResult:    user passed --help
    #   - VersionResult: user passed --version
    #   - ParseResult:   normal invocation; flags and arguments are populated

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # This is the *only* part that is specific to the pwd tool.
    # CLI Builder has already validated the flags, so we just check
    # whether the "physical" flag is set.

    assert isinstance(result, ParseResult)

    if result.flags.get("physical"):
        print(get_physical_pwd())
    else:
        print(get_logical_pwd())


if __name__ == "__main__":
    main()
