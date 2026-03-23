"""fold — wrap each input line to fit in specified width.

=== What This Program Does ===

This is a reimplementation of the GNU ``fold`` utility. It wraps long
lines by inserting newlines so that no output line exceeds a given
width (default 80 columns).

=== How Line Wrapping Works ===

The basic algorithm is simple: walk through each character of the input,
tracking the current column position. When the column reaches the width
limit, insert a newline and reset the column counter.

Tab characters are handled specially: they advance the column to the
next tab stop (every 8 columns), not by 1. Backspace characters
move the column back by 1 (but never below 0).

=== The -s Flag (Break at Spaces) ===

Without ``-s``, fold breaks lines at exactly the width limit, even if
that's in the middle of a word. With ``-s``, fold prefers to break at
the last space before the width limit, producing more readable output.

For example, wrapping "The quick brown fox" at width 10:

- Without ``-s``: ``"The quick \\nbrown fox"`` (breaks at column 10)
- With ``-s``: ``"The quick\\n brown fox"`` (breaks at the space before "brown")

If there's no space within the line, fold breaks at the width limit
as a fallback.

=== The -b Flag (Count Bytes) ===

By default, fold counts display columns (accounting for tabs and
backspaces). With ``-b``, fold counts raw bytes instead, treating
every character as width 1.

=== CLI Builder Integration ===

The entire CLI is defined in ``fold.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "fold.json")


def fold_line(
    line: str,
    width: int,
    *,
    break_at_spaces: bool,
    count_bytes: bool,
) -> str:
    """Fold a single line to the given width.

    This function implements the core line-wrapping algorithm. It walks
    through the line character by character, tracking the column position,
    and inserts newlines when the width limit is reached.

    Args:
        line: The input line (without trailing newline).
        width: The maximum line width.
        break_at_spaces: If True, prefer breaking at spaces.
        count_bytes: If True, count bytes instead of columns.

    Returns:
        The folded line (may contain embedded newlines).
    """
    if width <= 0:
        return line

    result: list[str] = []
    column = 0
    # For -s mode, track the last space position in the current segment.
    last_space_idx = -1
    segment_start = len(result)

    for ch in line:
        if ch == "\n":
            # Actual newline in input: emit and reset.
            result.append(ch)
            column = 0
            last_space_idx = -1
            segment_start = len(result)
            continue

        # Calculate the column advance for this character.
        if count_bytes:
            advance = 1
        elif ch == "\t":
            # Tab advances to the next multiple of 8.
            advance = 8 - (column % 8)
        elif ch == "\b":
            # Backspace moves back by 1.
            advance = -1 if column > 0 else 0
        else:
            advance = 1

        # Check if this character would exceed the width.
        if column + advance > width:
            if break_at_spaces and last_space_idx >= segment_start:
                # Break at the last space. We need to insert a newline
                # after the last space and re-emit characters after it.
                # Find the position in result to split.
                result.insert(last_space_idx + 1, "\n")
                # Recalculate column for the part after the break.
                after_break = "".join(result[last_space_idx + 2 :])
                column = 0
                for c in after_break:
                    if count_bytes:
                        column += 1
                    elif c == "\t":
                        column += 8 - (column % 8)
                    elif c == "\b":
                        column = max(0, column - 1)
                    else:
                        column += 1
                last_space_idx = -1
                segment_start = last_space_idx + 2
            else:
                # Hard break at the width limit.
                result.append("\n")
                column = 0
                last_space_idx = -1
                segment_start = len(result)

        result.append(ch)
        column += advance

        if ch == " ":
            last_space_idx = len(result) - 1

    return "".join(result)


def main() -> None:
    """Entry point: parse args via CLI Builder, then fold lines."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"fold: {error.message}", file=sys.stderr)
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

    width = result.flags.get("width", 80)
    break_at_spaces = result.flags.get("spaces", False)
    count_bytes = result.flags.get("bytes", False)

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
                    # Strip the trailing newline, fold, then re-add it.
                    stripped = line.rstrip("\n")
                    folded = fold_line(
                        stripped,
                        width,
                        break_at_spaces=break_at_spaces,
                        count_bytes=count_bytes,
                    )
                    sys.stdout.write(folded)
                    if line.endswith("\n"):
                        sys.stdout.write("\n")
            else:
                with open(filename) as f:
                    for line in f:
                        stripped = line.rstrip("\n")
                        folded = fold_line(
                            stripped,
                            width,
                            break_at_spaces=break_at_spaces,
                            count_bytes=count_bytes,
                        )
                        sys.stdout.write(folded)
                        if line.endswith("\n"):
                            sys.stdout.write("\n")
        except FileNotFoundError:
            print(
                f"fold: {filename}: No such file or directory",
                file=sys.stderr,
            )
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        except BrokenPipeError:
            raise SystemExit(0) from None


if __name__ == "__main__":
    main()
