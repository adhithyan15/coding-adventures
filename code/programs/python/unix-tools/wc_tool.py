"""wc — print newline, word, and byte counts for each file.

=== What This Program Does ===

This is a reimplementation of the ``wc`` (word count) utility. Despite its
name, ``wc`` counts much more than words. By default, it prints three
numbers for each input file:

    lines  words  bytes  filename

For example::

    $ wc hello.txt
      5  12  68 hello.txt

This tells us that ``hello.txt`` has 5 lines, 12 words, and 68 bytes.

=== What Counts as a "Line", "Word", and "Byte"? ===

- **Line**: A newline character (``\\n``) terminates a line. So ``wc -l``
  counts the number of newline characters. A file that ends without a
  trailing newline has one fewer "line" than you might expect.

- **Word**: A contiguous sequence of non-whitespace characters. Whitespace
  includes spaces, tabs, newlines, carriage returns, etc. The string
  ``"hello  world"`` has 2 words regardless of how many spaces separate them.

- **Byte**: A single byte in the file. For ASCII text, bytes and characters
  are the same. For UTF-8 text, a single character might be 2-4 bytes.

- **Character** (-m): A Unicode code point. The string ``"cafe\\u0301"``
  (café with combining accent) has 5 characters but might be 6 bytes.

=== Column Alignment ===

When displaying counts, ``wc`` right-aligns the numbers in columns. The
column width is determined by the largest number in the output. This makes
multi-file output easy to scan::

    $ wc *.py
       142   589  4521 cat_tool.py
        35    98   812 echo_tool.py
        10    25   187 true_tool.py
       187   712  5520 total

=== The "total" Line ===

When given multiple files, ``wc`` prints a "total" line at the end that
sums up all the counts. This only appears when there are 2 or more files.

=== CLI Builder Integration ===

The entire CLI is defined in ``wc.json``. CLI Builder handles flag parsing,
help text, version output, and the mutual exclusivity of ``-c`` and ``-m``
(you can't count both bytes and characters — they'd occupy the same column).
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

SPEC_FILE = str(Path(__file__).parent / "wc.json")


def count_content(content: str, raw_bytes: bytes) -> dict[str, int]:
    """Count lines, words, bytes, characters, and max line length.

    This function computes all five metrics that ``wc`` can report. Even
    if the user only asked for one (e.g., ``-l``), we compute all of them
    here and let the caller decide which to display. This keeps the
    counting logic in one place.

    Args:
        content: The file content as a decoded string (for line/word/char counts).
        raw_bytes: The file content as raw bytes (for byte count).

    Returns:
        A dictionary with keys: lines, words, bytes, chars, max_line_length.
    """
    # --- Line count ---
    # Count newline characters. This matches POSIX behavior: a file with
    # content "hello" (no trailing newline) has 0 lines.
    lines = content.count("\n")

    # --- Word count ---
    # split() with no arguments splits on any whitespace and ignores
    # leading/trailing whitespace. This is exactly what wc does.
    words = len(content.split())

    # --- Byte count ---
    # The number of bytes in the raw file content. For UTF-8 files,
    # this may differ from the character count.
    byte_count = len(raw_bytes)

    # --- Character count ---
    # The number of Unicode code points. For ASCII files, this equals
    # the byte count. For multibyte encodings, it may be smaller.
    chars = len(content)

    # --- Maximum line length ---
    # The length of the longest line, not counting the newline character.
    # This is useful for determining terminal width requirements.
    if content:
        max_line_length = max(len(line) for line in content.split("\n"))
    else:
        max_line_length = 0

    return {
        "lines": lines,
        "words": words,
        "bytes": byte_count,
        "chars": chars,
        "max_line_length": max_line_length,
    }


def read_input(filename: str) -> tuple[str, bytes]:
    """Read file content as both string and bytes.

    We need both representations because:
    - Line, word, and character counts operate on the decoded string.
    - Byte count operates on the raw bytes.

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Returns:
        A tuple of (decoded_string, raw_bytes).

    Raises:
        SystemExit: If the file cannot be opened.
    """
    if filename == "-":
        try:
            content = sys.stdin.read()
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        return content, content.encode("utf-8")

    try:
        raw = Path(filename).read_bytes()
        content = raw.decode("utf-8", errors="replace")
        return content, raw
    except FileNotFoundError:
        print(f"wc: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"wc: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"wc: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def format_counts(
    counts: dict[str, int],
    *,
    show_lines: bool,
    show_words: bool,
    show_bytes: bool,
    show_chars: bool,
    show_max_line_length: bool,
    width: int,
    filename: str | None = None,
) -> str:
    """Format a single line of wc output.

    The output consists of right-aligned numeric columns followed by the
    filename (if any). The column width is determined by the caller based
    on the largest number across all files.

    The order of columns is always: lines, words, bytes/chars, max-line-length.
    This matches the standard ``wc`` output order.

    Args:
        counts: Dictionary of count values.
        show_lines: Whether to include the line count.
        show_words: Whether to include the word count.
        show_bytes: Whether to include the byte count.
        show_chars: Whether to include the character count.
        show_max_line_length: Whether to include max line length.
        width: The minimum column width for right-alignment.
        filename: Optional filename to append (None for stdin-only).

    Returns:
        A formatted string ready to print.
    """
    parts: list[str] = []

    if show_lines:
        parts.append(f"{counts['lines']:>{width}}")
    if show_words:
        parts.append(f"{counts['words']:>{width}}")
    if show_bytes:
        parts.append(f"{counts['bytes']:>{width}}")
    if show_chars:
        parts.append(f"{counts['chars']:>{width}}")
    if show_max_line_length:
        parts.append(f"{counts['max_line_length']:>{width}}")

    result = " ".join(parts)

    if filename is not None:
        result += f" {filename}"

    return result


def main() -> None:
    """Entry point: parse args via CLI Builder, then count words/lines/bytes."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"wc: {error.message}", file=sys.stderr)
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

    # Determine which counts to show.
    # If no specific flags are given, show lines + words + bytes (the default).
    show_lines = result.flags.get("lines", False)
    show_words = result.flags.get("words", False)
    show_bytes = result.flags.get("bytes", False)
    show_chars = result.flags.get("chars", False)
    show_max_line_length = result.flags.get("max_line_length", False)

    # If none of the specific flags are set, default to lines + words + bytes.
    any_specific = show_lines or show_words or show_bytes or show_chars or show_max_line_length
    if not any_specific:
        show_lines = True
        show_words = True
        show_bytes = True

    # Get the list of files.
    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]
    if not files:
        files = ["-"]

    # --- Count each file ---
    all_counts: list[dict[str, int]] = []
    filenames: list[str] = []

    for filename in files:
        content, raw_bytes = read_input(filename)
        counts = count_content(content, raw_bytes)
        all_counts.append(counts)
        filenames.append(filename)

    # --- Compute totals (for multiple files) ---
    totals: dict[str, int] = {
        "lines": sum(c["lines"] for c in all_counts),
        "words": sum(c["words"] for c in all_counts),
        "bytes": sum(c["bytes"] for c in all_counts),
        "chars": sum(c["chars"] for c in all_counts),
        "max_line_length": max((c["max_line_length"] for c in all_counts), default=0),
    }

    # --- Determine column width ---
    # The width is based on the largest number we'll display. This ensures
    # all columns are properly aligned.
    all_values: list[int] = []
    for counts in all_counts:
        if show_lines:
            all_values.append(counts["lines"])
        if show_words:
            all_values.append(counts["words"])
        if show_bytes:
            all_values.append(counts["bytes"])
        if show_chars:
            all_values.append(counts["chars"])
        if show_max_line_length:
            all_values.append(counts["max_line_length"])
    # Include totals in width calculation if we'll print them.
    if len(files) > 1:
        if show_lines:
            all_values.append(totals["lines"])
        if show_words:
            all_values.append(totals["words"])
        if show_bytes:
            all_values.append(totals["bytes"])
        if show_chars:
            all_values.append(totals["chars"])
        if show_max_line_length:
            all_values.append(totals["max_line_length"])

    max_val = max(all_values) if all_values else 0
    width = max(len(str(max_val)), 1)

    # --- Print results ---
    for i, counts in enumerate(all_counts):
        # For a single stdin-only invocation, don't print a filename.
        fname = None if (len(files) == 1 and filenames[i] == "-") else filenames[i]
        print(
            format_counts(
                counts,
                show_lines=show_lines,
                show_words=show_words,
                show_bytes=show_bytes,
                show_chars=show_chars,
                show_max_line_length=show_max_line_length,
                width=width,
                filename=fname,
            )
        )

    # Print total line if there were multiple files.
    if len(files) > 1:
        print(
            format_counts(
                totals,
                show_lines=show_lines,
                show_words=show_words,
                show_bytes=show_bytes,
                show_chars=show_chars,
                show_max_line_length=show_max_line_length,
                width=width,
                filename="total",
            )
        )


if __name__ == "__main__":
    main()
