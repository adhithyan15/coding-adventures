"""comm — compare two sorted files line by line.

=== What This Program Does ===

This is a reimplementation of the GNU ``comm`` utility. It compares
two *sorted* files and produces three columns of output:

- **Column 1**: Lines unique to FILE1.
- **Column 2**: Lines unique to FILE2.
- **Column 3**: Lines common to both files.

=== Three-Column Output ===

The output uses tab indentation to distinguish the columns::

    $ comm file1 file2
    apple               <- only in file1 (column 1)
            banana       <- only in file2 (column 2)
                    cherry <- in both (column 3)

The number of leading tabs tells you which column a line belongs to.
Column 2 lines are preceded by 1 tab, column 3 lines by 2 tabs.

=== Suppressing Columns ===

The ``-1``, ``-2``, and ``-3`` flags suppress the corresponding columns.
This is useful for extracting specific relationships::

    comm -12 file1 file2   # Show only lines common to both
    comm -23 file1 file2   # Show only lines unique to file1
    comm -13 file1 file2   # Show only lines unique to file2

=== The Algorithm ===

Because both files are sorted, we can use a merge-style algorithm:

1. Compare the current line from each file.
2. If file1's line is smaller, it's unique to file1 — advance file1.
3. If file2's line is smaller, it's unique to file2 — advance file2.
4. If they're equal, it's common — advance both.
5. When one file is exhausted, remaining lines from the other are unique.

This runs in O(n + m) time where n and m are the file lengths.

=== CLI Builder Integration ===

The entire CLI is defined in ``comm.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "comm.json")


# ---------------------------------------------------------------------------
# Business logic: compare_sorted
# ---------------------------------------------------------------------------


def compare_sorted(
    lines1: list[str],
    lines2: list[str],
    *,
    suppress: tuple[bool, bool, bool] = (False, False, False),
    output_delimiter: str = "\t",
) -> list[str]:
    """Compare two sorted lists of lines and produce three-column output.

    This implements the classic merge-comparison algorithm. Both inputs
    must be sorted in ascending order for correct results.

    Args:
        lines1: Sorted lines from the first file.
        lines2: Sorted lines from the second file.
        suppress: A 3-tuple of booleans. ``suppress[0]`` suppresses
                  column 1 (lines unique to file1), ``suppress[1]``
                  suppresses column 2, ``suppress[2]`` suppresses column 3.
        output_delimiter: The string used to separate columns (default: TAB).

    Returns:
        The output lines with appropriate column indentation.

    Example::

        >>> compare_sorted(["a", "b", "d"], ["b", "c", "d"])
        ['a', '\\tb', '\\tc', '\\t\\td']
    """
    result: list[str] = []
    i, j = 0, 0
    suppress1, suppress2, suppress3 = suppress

    # Calculate the prefix for each column.
    # Column 1: no prefix.
    # Column 2: one delimiter (or nothing if col1 is suppressed).
    # Column 3: two delimiters (minus one for each suppressed earlier column).
    def col1_prefix() -> str:
        return ""

    def col2_prefix() -> str:
        if suppress1:
            return ""
        return output_delimiter

    def col3_prefix() -> str:
        tabs = 2
        if suppress1:
            tabs -= 1
        if suppress2:
            tabs -= 1
        return output_delimiter * tabs

    while i < len(lines1) and j < len(lines2):
        if lines1[i] < lines2[j]:
            # Line is unique to file1 (column 1).
            if not suppress1:
                result.append(col1_prefix() + lines1[i])
            i += 1
        elif lines1[i] > lines2[j]:
            # Line is unique to file2 (column 2).
            if not suppress2:
                result.append(col2_prefix() + lines2[j])
            j += 1
        else:
            # Line is common to both (column 3).
            if not suppress3:
                result.append(col3_prefix() + lines1[i])
            i += 1
            j += 1

    # Remaining lines in file1 are unique to file1.
    while i < len(lines1):
        if not suppress1:
            result.append(col1_prefix() + lines1[i])
        i += 1

    # Remaining lines in file2 are unique to file2.
    while j < len(lines2):
        if not suppress2:
            result.append(col2_prefix() + lines2[j])
        j += 1

    return result


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then compare files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"comm: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    suppress1 = result.flags.get("suppress_col1", False)
    suppress2 = result.flags.get("suppress_col2", False)
    suppress3 = result.flags.get("suppress_col3", False)
    output_delimiter = result.flags.get("output_delimiter") or "\t"

    file1 = result.arguments.get("file1")
    file2 = result.arguments.get("file2")

    try:
        if file1 == "-":
            lines1 = [line.rstrip("\n") for line in sys.stdin]
        else:
            with open(file1) as f:
                lines1 = [line.rstrip("\n") for line in f]

        if file2 == "-":
            lines2 = [line.rstrip("\n") for line in sys.stdin]
        else:
            with open(file2) as f:
                lines2 = [line.rstrip("\n") for line in f]
    except FileNotFoundError as exc:
        print(f"comm: {exc.filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None

    output_lines = compare_sorted(
        lines1,
        lines2,
        suppress=(suppress1, suppress2, suppress3),
        output_delimiter=output_delimiter,
    )

    try:
        for line in output_lines:
            print(line)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
