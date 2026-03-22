"""uniq — report or omit repeated lines.

=== What This Program Does ===

This is a reimplementation of the GNU ``uniq`` utility. It filters
**adjacent** duplicate lines from its input. This is the key detail:
uniq only considers lines that are next to each other. If you want to
find all duplicates regardless of position, sort the input first::

    $ sort file.txt | uniq

=== How Adjacent Comparison Works ===

uniq reads lines one at a time and compares each line to the previous
one. If they match (according to the current comparison rules), the
duplicate is either suppressed or counted. The comparison can be
modified by:

- ``-i``: Ignore case differences ("Hello" == "hello").
- ``-f N``: Skip the first N whitespace-delimited fields before comparing.
- ``-s N``: Skip the first N characters before comparing.
- ``-w N``: Compare at most N characters (after skipping).

=== Output Modes ===

uniq has three output modes, controlled by flags:

- **Default**: Print one copy of each group of adjacent identical lines.
- ``-d`` (duplicated): Print only lines that appear more than once.
- ``-u`` (unique): Print only lines that appear exactly once.
- ``-c`` (count): Prefix each line with the number of occurrences.

=== Field and Character Skipping ===

The ``-f`` and ``-s`` flags let you skip parts of each line before
comparing. This is useful for structured data where some columns
should be ignored. For example, if each line starts with a timestamp,
``-f 1`` skips the timestamp and compares the rest.

=== CLI Builder Integration ===

The entire CLI is defined in ``uniq.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "uniq.json")


def get_comparison_key(
    line: str,
    *,
    skip_fields: int,
    skip_chars: int,
    check_chars: int | None,
    ignore_case: bool,
) -> str:
    """Extract the portion of a line used for comparison.

    This function implements the field-skipping, character-skipping,
    and width-limiting logic that uniq uses to decide whether two
    lines are "the same."

    The order of operations is:
    1. Skip N fields (whitespace-delimited tokens).
    2. Skip N characters from what remains.
    3. Take at most W characters from what remains.
    4. Optionally fold to lowercase for case-insensitive comparison.

    Args:
        line: The full line (without trailing newline).
        skip_fields: Number of fields to skip (-f).
        skip_chars: Number of characters to skip (-s).
        check_chars: Maximum characters to compare (-w), or None.
        ignore_case: Whether to ignore case (-i).

    Returns:
        The comparison key string.
    """
    s = line

    # Step 1: Skip fields. A "field" is preceded by whitespace.
    if skip_fields > 0:
        remaining = s
        for _ in range(skip_fields):
            # Skip leading whitespace.
            remaining = remaining.lstrip(" \t")
            # Skip the field (non-whitespace characters).
            idx = 0
            while idx < len(remaining) and remaining[idx] not in (" ", "\t"):
                idx += 1
            remaining = remaining[idx:]
        s = remaining

    # Step 2: Skip characters.
    if skip_chars > 0:
        s = s[skip_chars:]

    # Step 3: Limit comparison width.
    if check_chars is not None:
        s = s[:check_chars]

    # Step 4: Fold case if requested.
    if ignore_case:
        s = s.lower()

    return s


def uniq_lines(
    lines: list[str],
    *,
    count: bool,
    repeated: bool,
    unique: bool,
    ignore_case: bool,
    skip_fields: int,
    skip_chars: int,
    check_chars: int | None,
) -> list[str]:
    """Filter adjacent duplicate lines.

    This is the core algorithm. We iterate through the lines, grouping
    adjacent lines that have the same comparison key. For each group,
    we decide what to output based on the flags.

    Args:
        lines: The input lines (with trailing newlines stripped).
        count: If True, prefix each output line with occurrence count.
        repeated: If True, only output lines that appear more than once.
        unique: If True, only output lines that appear exactly once.
        ignore_case: Ignore case in comparisons.
        skip_fields: Number of fields to skip.
        skip_chars: Number of characters to skip.
        check_chars: Max characters to compare, or None.

    Returns:
        The filtered output lines.
    """
    if not lines:
        return []

    result: list[str] = []

    # Process groups of adjacent identical lines.
    current_line = lines[0]
    current_key = get_comparison_key(
        current_line,
        skip_fields=skip_fields,
        skip_chars=skip_chars,
        check_chars=check_chars,
        ignore_case=ignore_case,
    )
    current_count = 1

    def emit_group(line: str, group_count: int) -> None:
        """Emit a group based on the current flags."""
        # -d: only print if duplicated (count > 1).
        if repeated and group_count < 2:
            return
        # -u: only print if unique (count == 1).
        if unique and group_count > 1:
            return

        if count:
            result.append(f"{group_count:7} {line}")
        else:
            result.append(line)

    for line in lines[1:]:
        key = get_comparison_key(
            line,
            skip_fields=skip_fields,
            skip_chars=skip_chars,
            check_chars=check_chars,
            ignore_case=ignore_case,
        )

        if key == current_key:
            # Same group — increment count.
            current_count += 1
        else:
            # New group — emit the previous one.
            emit_group(current_line, current_count)
            current_line = line
            current_key = key
            current_count = 1

    # Don't forget the last group.
    emit_group(current_line, current_count)

    return result


def main() -> None:
    """Entry point: parse args via CLI Builder, then filter lines."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"uniq: {error.message}", file=sys.stderr)
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

    count_flag = result.flags.get("count", False)
    repeated = result.flags.get("repeated", False)
    unique_flag = result.flags.get("unique", False)
    ignore_case = result.flags.get("ignore_case", False)
    skip_fields = result.flags.get("skip_fields", 0) or 0
    skip_chars = result.flags.get("skip_chars", 0) or 0
    check_chars = result.flags.get("check_chars")

    input_file = result.arguments.get("input_file")
    output_file = result.arguments.get("output_file")

    # Read input.
    try:
        if input_file and input_file != "-":
            with open(input_file) as f:
                raw_lines = f.readlines()
        else:
            raw_lines = sys.stdin.readlines()
    except FileNotFoundError:
        print(
            f"uniq: {input_file}: No such file or directory",
            file=sys.stderr,
        )
        raise SystemExit(1) from None
    except KeyboardInterrupt:
        raise SystemExit(130) from None

    # Strip trailing newlines for comparison, but preserve them for output.
    lines = [line.rstrip("\n") for line in raw_lines]

    # Process.
    output_lines = uniq_lines(
        lines,
        count=count_flag,
        repeated=repeated,
        unique=unique_flag,
        ignore_case=ignore_case,
        skip_fields=skip_fields,
        skip_chars=skip_chars,
        check_chars=check_chars,
    )

    # Write output.
    try:
        if output_file:
            with open(output_file, "w") as f:
                for line in output_lines:
                    f.write(line + "\n")
        else:
            for line in output_lines:
                print(line)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
