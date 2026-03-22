"""yes — repeatedly output a line with a string.

=== What This Program Does ===

This is a reimplementation of the POSIX ``yes`` utility. It repeatedly
prints a string to standard output, over and over, forever (or until
killed). If you give it arguments, it joins them with spaces and prints
that. If you give it no arguments, it prints ``y``.

=== Why Does This Exist? ===

``yes`` is used in shell scripting to automatically answer "yes" to
interactive prompts. For example::

    yes | rm -i *.tmp

The ``rm -i`` command asks "remove file?" for each file. ``yes`` feeds
it an endless stream of "y" responses, so every file gets removed
without manual intervention.

You can also use it with custom strings::

    yes "I agree" | some-license-tool

=== How It Works ===

The algorithm is trivially simple:

1. Join all positional arguments with spaces (or use "y" if none given).
2. Print that string in an infinite loop.

The only subtlety is that in real usage, the loop runs until the process
is killed (usually by SIGPIPE when the downstream reader closes). We
handle the BrokenPipeError that Python raises in that case.

=== CLI Builder Integration ===

The JSON spec ``yes.json`` defines a single variadic argument called
``string`` with a default of ``"y"``. CLI Builder handles all parsing,
and we just implement the infinite loop.
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

SPEC_FILE = str(Path(__file__).parent / "yes.json")


# ---------------------------------------------------------------------------
# Business logic
# ---------------------------------------------------------------------------


def build_yes_line(strings: list[str] | str | None) -> str:
    """Build the line that yes will print repeatedly.

    If the user provided one or more strings, join them with spaces.
    If no strings were provided (or the default kicked in), use "y".

    Args:
        strings: The positional arguments from CLI Builder. Could be a list
                 of strings, a single string, or None if nothing was passed.

    Returns:
        The string to print on each iteration.

    Examples:
        >>> build_yes_line(None)
        'y'
        >>> build_yes_line([])
        'y'
        >>> build_yes_line("y")
        'y'
        >>> build_yes_line(["hello", "world"])
        'hello world'
        >>> build_yes_line(["sure"])
        'sure'
    """
    if strings is None:
        return "y"
    if isinstance(strings, list):
        return " ".join(strings) if strings else "y"
    return str(strings) if strings else "y"


def yes_loop(line: str, max_count: int | None = None) -> None:
    """Print *line* repeatedly to stdout.

    In normal operation, ``max_count`` is ``None`` and this loops forever
    (until the process is killed or a BrokenPipeError occurs). For testing,
    pass ``max_count`` to limit the number of iterations.

    Args:
        line: The string to print on each iteration.
        max_count: If set, stop after this many iterations. None means
                   loop forever.

    Raises:
        BrokenPipeError: Silently handled — this is expected when the
                         downstream reader closes (e.g., ``yes | head -5``).
    """
    try:
        count = 0
        while max_count is None or count < max_count:
            print(line)
            count += 1
    except BrokenPipeError:
        # This is normal — the downstream reader closed the pipe.
        # For example: yes | head -5
        # After head reads 5 lines, it closes stdin, and our write fails.
        pass


def main() -> None:
    """Entry point: parse args via CLI Builder, then loop forever."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    # Hand the spec file and sys.argv to CLI Builder. The parser reads the
    # JSON spec, handles --help and --version, and returns a typed result.
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"yes: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # Build the output line and loop forever.
    assert isinstance(result, ParseResult)

    strings = result.arguments.get("string")
    line = build_yes_line(strings)
    yes_loop(line)


if __name__ == "__main__":
    main()
