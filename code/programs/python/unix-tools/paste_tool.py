"""paste — merge lines of files.

=== What This Program Does ===

This is a reimplementation of the GNU ``paste`` utility. It merges
corresponding lines from multiple files, joining them with a delimiter
(default: TAB).

=== How Paste Works ===

In its default (parallel) mode, ``paste`` reads one line from each
input file and joins them with the delimiter::

    File A:     File B:     Output:
    a1          b1          a1\\tb1
    a2          b2          a2\\tb2
    a3                      a3\\t

If files have different lengths, missing lines are treated as empty
strings. This is like a SQL FULL OUTER JOIN on line number.

=== Serial Mode (-s) ===

With ``-s``, paste reads all lines from one file and joins them on a
single output line, then does the same for the next file::

    File A: a1, a2, a3  ->  a1\\ta2\\ta3
    File B: b1, b2      ->  b1\\tb2

=== Delimiter Cycling ===

The ``-d`` flag accepts a list of delimiter characters. They are used
in round-robin fashion. For example, ``-d ',:'`` alternates between
comma and colon::

    paste -d ',:'  file1 file2 file3
    # a1,b1:c1
    # a2,b2:c2

=== CLI Builder Integration ===

The entire CLI is defined in ``paste.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import TextIO

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "paste.json")


# ---------------------------------------------------------------------------
# Business logic: paste_files
# ---------------------------------------------------------------------------


def paste_parallel(
    file_contents: list[list[str]],
    delimiters: str = "\t",
) -> list[str]:
    """Merge lines from multiple files in parallel (the default mode).

    For each line number, we take one line from each file and join
    them with the delimiter(s). If a file has fewer lines, its
    contribution is an empty string.

    Args:
        file_contents: A list of lists, where each inner list is the
                       lines from one file (newlines stripped).
        delimiters: The delimiter string. Characters are cycled through
                    when there are more than 2 files.

    Returns:
        The merged output lines.
    """
    if not file_contents:
        return []

    # Find the maximum number of lines across all files.
    max_lines = max(len(fc) for fc in file_contents)
    result: list[str] = []

    for line_idx in range(max_lines):
        parts: list[str] = []
        for file_idx, fc in enumerate(file_contents):
            if file_idx > 0:
                # Pick the delimiter using round-robin.
                delim_idx = (file_idx - 1) % len(delimiters)
                parts.append(delimiters[delim_idx])
            if line_idx < len(fc):
                parts.append(fc[line_idx])
            else:
                parts.append("")
        result.append("".join(parts))

    return result


def paste_serial(
    file_contents: list[list[str]],
    delimiters: str = "\t",
) -> list[str]:
    """Merge lines from each file serially (-s mode).

    Each file's lines are joined into a single output line. The
    delimiter characters cycle across the joins within one file.

    Args:
        file_contents: A list of lists, where each inner list is the
                       lines from one file (newlines stripped).
        delimiters: The delimiter string (cycled for multi-line files).

    Returns:
        One output line per input file.
    """
    result: list[str] = []
    for fc in file_contents:
        if not fc:
            result.append("")
            continue
        parts: list[str] = [fc[0]]
        for i, line in enumerate(fc[1:]):
            delim_idx = i % len(delimiters)
            parts.append(delimiters[delim_idx])
            parts.append(line)
        result.append("".join(parts))
    return result


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then paste files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"paste: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    delimiters = result.flags.get("delimiters") or "\t"
    serial = result.flags.get("serial", False)
    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]

    # Read all file contents.
    file_contents: list[list[str]] = []
    handles_to_close: list[TextIO] = []
    try:
        for fname in files:
            if fname == "-":
                lines = [line.rstrip("\n") for line in sys.stdin]
            else:
                f = open(fname)  # noqa: SIM115
                handles_to_close.append(f)
                lines = [line.rstrip("\n") for line in f]
            file_contents.append(lines)
    except FileNotFoundError:
        print(f"paste: {fname}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    finally:
        for h in handles_to_close:
            h.close()

    # Merge and output.
    if serial:
        output_lines = paste_serial(file_contents, delimiters)
    else:
        output_lines = paste_parallel(file_contents, delimiters)

    try:
        for line in output_lines:
            print(line)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
