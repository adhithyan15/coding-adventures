"""expand — convert tabs to spaces.

=== What This Program Does ===

This is a reimplementation of the GNU ``expand`` utility. It replaces
tab characters with the appropriate number of spaces to reach the next
tab stop. By default, tab stops are every 8 columns.

=== How Tab Stops Work ===

A tab character doesn't represent a fixed number of spaces. Instead,
it advances the cursor to the next tab stop position. With the default
8-column tab stops, the positions are 0, 8, 16, 24, ...

For example, if the cursor is at column 3, a tab advances it to
column 8 (inserting 5 spaces). If the cursor is at column 7, a tab
advances it to column 8 (inserting 1 space). If the cursor is at
column 8, a tab advances it to column 16 (inserting 8 spaces).

The formula is: ``spaces_needed = tab_size - (column % tab_size)``

=== Custom Tab Stops ===

The ``-t`` flag changes the tab stop interval. It accepts either:

- A single number (``-t 4``): Tab stops every 4 columns.
- A comma-separated list (``-t 4,8,12``): Tab stops at specific columns.
  After the last listed stop, tabs are replaced with a single space.

=== The -i Flag (Initial Only) ===

With ``-i``, expand only processes tabs at the beginning of each line
(before the first non-blank character). Tabs after non-blank characters
are left unchanged. This is useful for preserving intentional tab
formatting within lines while normalizing indentation.

=== CLI Builder Integration ===

The entire CLI is defined in ``expand.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "expand.json")


def parse_tab_stops(tab_str: str | None) -> list[int] | int:
    """Parse the -t flag value into tab stop positions.

    If the value is a single number, return it as an integer (uniform
    tab width). If it's a comma-separated list, return the list of
    positions (explicit tab stops).

    Args:
        tab_str: The raw -t flag value, or None for default.

    Returns:
        Either an integer (uniform tab width) or a sorted list of
        positions (explicit tab stops).
    """
    if tab_str is None:
        return 8

    if "," in tab_str:
        # Explicit list of tab stop positions.
        try:
            stops = sorted(int(s.strip()) for s in tab_str.split(","))
        except ValueError:
            print(f"expand: invalid tab stop spec '{tab_str}'", file=sys.stderr)
            raise SystemExit(1) from None
        return stops

    try:
        return int(tab_str)
    except ValueError:
        print(f"expand: invalid tab stop spec '{tab_str}'", file=sys.stderr)
        raise SystemExit(1) from None


def spaces_to_next_stop(column: int, tab_stops: list[int] | int) -> int:
    """Calculate the number of spaces to the next tab stop.

    Args:
        column: The current column position (0-based).
        tab_stops: Either a uniform tab width (int) or a list of
            explicit tab stop positions.

    Returns:
        The number of spaces needed to reach the next tab stop.
    """
    if isinstance(tab_stops, int):
        # Uniform tab stops: simple modulo arithmetic.
        return tab_stops - (column % tab_stops)

    # Explicit tab stops: find the first stop after the current column.
    for stop in tab_stops:
        if stop > column:
            return stop - column

    # Past the last explicit stop: use a single space.
    return 1


def expand_line(line: str, tab_stops: list[int] | int, *, initial_only: bool) -> str:
    """Expand tabs in a single line.

    Args:
        line: The input line (may include trailing newline).
        tab_stops: Tab stop specification.
        initial_only: If True, only expand tabs before non-blank chars.

    Returns:
        The line with tabs replaced by spaces.
    """
    result: list[str] = []
    column = 0
    # Track whether we've seen a non-blank character (for -i flag).
    seen_non_blank = False

    for ch in line:
        if ch == "\t":
            if initial_only and seen_non_blank:
                # After non-blank content, keep the tab as-is.
                result.append("\t")
                column += 1  # Approximate; tab display depends on terminal.
            else:
                # Replace the tab with spaces.
                num_spaces = spaces_to_next_stop(column, tab_stops)
                result.append(" " * num_spaces)
                column += num_spaces
        elif ch == "\n":
            result.append(ch)
            column = 0
            seen_non_blank = False
        else:
            if ch != " ":
                seen_non_blank = True
            result.append(ch)
            column += 1

    return "".join(result)


def main() -> None:
    """Entry point: parse args via CLI Builder, then expand tabs."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"expand: {error.message}", file=sys.stderr)
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

    tab_str = result.flags.get("tabs")
    initial_only = result.flags.get("initial", False)

    tab_stops = parse_tab_stops(tab_str)

    # Get the list of files.
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]
    if not files:
        files = ["-"]

    for filename in files:
        try:
            if filename == "-":
                for line in sys.stdin:
                    sys.stdout.write(
                        expand_line(line, tab_stops, initial_only=initial_only)
                    )
            else:
                with open(filename) as f:
                    for line in f:
                        sys.stdout.write(
                            expand_line(line, tab_stops, initial_only=initial_only)
                        )
        except FileNotFoundError:
            print(
                f"expand: {filename}: No such file or directory",
                file=sys.stderr,
            )
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        except BrokenPipeError:
            raise SystemExit(0) from None


if __name__ == "__main__":
    main()
