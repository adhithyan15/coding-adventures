"""unexpand — convert spaces to tabs.

=== What This Program Does ===

This is a reimplementation of the GNU ``unexpand`` utility. It is the
inverse of ``expand``: it replaces runs of spaces with tab characters
where possible, based on tab stop positions.

=== How unexpand Decides What to Replace ===

unexpand walks through each line character by character, tracking the
current column position. When it encounters spaces, it checks whether
a tab character could replace some or all of them:

1. Count consecutive spaces starting from the current position.
2. Check if any tab stop falls within this run of spaces.
3. If a tab stop aligns, replace the spaces up to that point with a tab.
4. Continue checking for more tab stops within the remaining spaces.

=== Default Behavior: Initial Blanks Only ===

By default, unexpand only converts spaces at the beginning of each
line (before any non-blank character). This preserves intentional
spacing within text while normalizing indentation.

=== The -a Flag (All Blanks) ===

With ``-a``, unexpand converts spaces throughout the entire line,
not just at the beginning. This is useful when you want to maximize
the use of tabs for compression or alignment.

=== Tab Stop Specification ===

Like ``expand``, unexpand accepts ``-t N`` for uniform tab stops
or ``-t N1,N2,...`` for explicit positions.

=== CLI Builder Integration ===

The entire CLI is defined in ``unexpand.json``. CLI Builder handles
flag parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "unexpand.json")


def parse_tab_stops(tab_str: str | None) -> list[int] | int:
    """Parse the -t flag value into tab stop positions.

    See expand_tool.py for detailed documentation on tab stop parsing.

    Args:
        tab_str: The raw -t flag value, or None for default.

    Returns:
        Either an integer (uniform tab width) or a sorted list of
        positions (explicit tab stops).
    """
    if tab_str is None:
        return 8

    if "," in tab_str:
        try:
            stops = sorted(int(s.strip()) for s in tab_str.split(","))
        except ValueError:
            print(f"unexpand: invalid tab stop spec '{tab_str}'", file=sys.stderr)
            raise SystemExit(1) from None
        return stops

    try:
        return int(tab_str)
    except ValueError:
        print(f"unexpand: invalid tab stop spec '{tab_str}'", file=sys.stderr)
        raise SystemExit(1) from None


def is_tab_stop(column: int, tab_stops: list[int] | int) -> bool:
    """Check whether a given column is a tab stop position.

    Args:
        column: The column position (0-based).
        tab_stops: Tab stop specification.

    Returns:
        True if the column is a tab stop.
    """
    if isinstance(tab_stops, int):
        return column % tab_stops == 0
    return column in tab_stops


def unexpand_line(line: str, tab_stops: list[int] | int, *, convert_all: bool) -> str:
    """Convert spaces to tabs in a single line.

    The algorithm walks through the line character by character:
    - When we encounter spaces, we accumulate them.
    - At each tab stop, we check if we can replace the accumulated
      spaces with a tab character.
    - Non-space characters are emitted directly.

    Args:
        line: The input line (may include trailing newline).
        tab_stops: Tab stop specification.
        convert_all: If True, convert all blanks (not just initial).

    Returns:
        The line with appropriate spaces replaced by tabs.
    """
    result: list[str] = []
    column = 0
    space_count = 0
    space_start_col = 0
    seen_non_blank = False

    for ch in line:
        if ch == " " and (convert_all or not seen_non_blank):
            # Accumulate spaces.
            if space_count == 0:
                space_start_col = column
            space_count += 1
            column += 1

            # Check if we've reached a tab stop.
            if is_tab_stop(column, tab_stops) and space_count > 1:
                # Replace the spaces with a single tab.
                result.append("\t")
                space_count = 0
        elif ch == "\t":
            # A tab in the input: flush any accumulated spaces, keep the tab.
            if space_count > 0:
                result.append(" " * space_count)
                space_count = 0
            result.append("\t")
            # Advance column to the next tab stop.
            if isinstance(tab_stops, int):
                column = column + (tab_stops - column % tab_stops)
            else:
                # Find next explicit tab stop.
                next_stop = column + 1
                for stop in tab_stops:
                    if stop > column:
                        next_stop = stop
                        break
                column = next_stop
        else:
            # Non-space character: flush any accumulated spaces.
            if space_count > 0:
                result.append(" " * space_count)
                space_count = 0

            if ch != " ":
                seen_non_blank = True

            result.append(ch)
            if ch == "\n":
                column = 0
                seen_non_blank = False
            else:
                column += 1

    # Flush any remaining spaces.
    if space_count > 0:
        result.append(" " * space_count)

    return "".join(result)


def main() -> None:
    """Entry point: parse args via CLI Builder, then unexpand spaces."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"unexpand: {error.message}", file=sys.stderr)
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
    convert_all = result.flags.get("all", False)

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
                        unexpand_line(line, tab_stops, convert_all=convert_all)
                    )
            else:
                with open(filename) as f:
                    for line in f:
                        sys.stdout.write(
                            unexpand_line(line, tab_stops, convert_all=convert_all)
                        )
        except FileNotFoundError:
            print(
                f"unexpand: {filename}: No such file or directory",
                file=sys.stderr,
            )
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        except BrokenPipeError:
            raise SystemExit(0) from None


if __name__ == "__main__":
    main()
