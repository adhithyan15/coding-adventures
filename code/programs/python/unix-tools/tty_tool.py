"""tty — print the file name of the terminal connected to standard input.

=== What This Program Does ===

This is a reimplementation of the POSIX ``tty`` utility. It prints the
path of the terminal device connected to standard input. If stdin is not
a terminal, it prints "not a tty" and exits with status 1.

=== Why Does This Exist? ===

Shell scripts sometimes need to know whether they're running interactively
(connected to a terminal) or being fed input from a pipe or file. The
``tty`` command answers this question::

    if tty -s; then
        echo "Running interactively"
    else
        echo "Running from a pipe or script"
    fi

The terminal path itself is also useful — it identifies *which* terminal
you're on::

    $ tty
    /dev/ttys003

=== The -s (Silent) Flag ===

With ``-s`` or ``--silent``, ``tty`` prints nothing at all. It only
communicates via its exit status:

- Exit 0: stdin IS a terminal
- Exit 1: stdin is NOT a terminal

This is perfect for scripts that only care about the yes/no answer::

    if tty -s; then
        # We have a terminal — can prompt the user
        read -p "Continue? " answer
    fi

=== How It Works ===

We use ``os.ttyname(0)`` to get the terminal device path for file
descriptor 0 (stdin). If stdin is not a terminal, this raises ``OSError``,
and we know we're not connected to a tty.

=== CLI Builder Integration ===

The JSON spec ``tty.json`` defines one flag: ``-s``/``--silent``. CLI
Builder handles the parsing, and we just implement the terminal detection
logic.
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

SPEC_FILE = str(Path(__file__).parent / "tty.json")


# ---------------------------------------------------------------------------
# Business logic
# ---------------------------------------------------------------------------


def get_tty_name() -> str | None:
    """Return the terminal device path for stdin, or None if not a tty.

    We call ``os.ttyname(0)`` which asks the operating system what
    terminal device is connected to file descriptor 0 (standard input).

    - If stdin is a terminal (e.g., ``/dev/ttys003``), we return that path.
    - If stdin is a pipe, file, or socket, ``os.ttyname`` raises ``OSError``
      and we return ``None``.

    Returns:
        The terminal device path as a string, or None if stdin is not
        a terminal.

    Examples:
        If running interactively:  "/dev/ttys003"
        If running from a pipe:    None
    """
    try:
        return os.ttyname(0)
    except OSError:
        return None


def main() -> None:
    """Entry point: parse args via CLI Builder, then print the tty name."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"tty: {error.message}", file=sys.stderr)
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

    silent = result.flags.get("silent", False)
    tty_name = get_tty_name()

    if tty_name is not None:
        # stdin IS a terminal.
        if not silent:
            print(tty_name)
        raise SystemExit(0)
    else:
        # stdin is NOT a terminal.
        if not silent:
            print("not a tty")
        raise SystemExit(1)


if __name__ == "__main__":
    main()
