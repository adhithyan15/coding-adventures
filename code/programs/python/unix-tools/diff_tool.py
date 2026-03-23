"""diff -- compare files line by line.

=== What This Program Does ===

This is a reimplementation of the GNU ``diff`` utility. It compares two
files (or directories) and reports the differences between them::

    diff file1.txt file2.txt         # Normal diff output
    diff -u file1.txt file2.txt      # Unified diff (most common)
    diff -c file1.txt file2.txt      # Context diff
    diff -r dir1/ dir2/              # Recursive directory comparison

=== How Diffing Works ===

At its core, ``diff`` finds the **longest common subsequence** (LCS) of
lines between two files. Lines not in the LCS are either:

- **Added** (present in file2 but not file1)
- **Removed** (present in file1 but not file2)

Python's ``difflib`` module implements this using the SequenceMatcher
algorithm, which is a refined version of the Ratcliff/Obershelp pattern
matching algorithm. It produces output similar to GNU diff.

=== Output Formats ===

There are three main output formats:

1. **Normal diff** (default): Shows ranges and change commands::

       2,3c2,3
       < old line 1
       < old line 2
       ---
       > new line 1
       > new line 2

2. **Unified diff** (``-u``): Shows context around changes with ``+``
   and ``-`` prefixes. This is the most commonly used format, especially
   for patches::

       --- file1.txt
       +++ file2.txt
       @@ -1,3 +1,3 @@
        common line
       -old line
       +new line
        common line

3. **Context diff** (``-c``): Similar to unified but with ``!`` for
   changed lines and separate sections for each file.

=== Preprocessing Options ===

Before comparing, lines can be preprocessed:

- ``-i``: Convert to lowercase (case-insensitive comparison)
- ``-b``: Collapse runs of whitespace to a single space
- ``-w``: Remove all whitespace
- ``-B``: Ignore blank lines entirely

=== CLI Builder Integration ===

The entire CLI is defined in ``diff.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import difflib
import fnmatch
import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "diff.json")


# ---------------------------------------------------------------------------
# Line preprocessing -- normalize lines before comparison.
# ---------------------------------------------------------------------------
# These functions transform lines so that certain differences are ignored.
# For example, -i makes the comparison case-insensitive by lowering all
# characters before comparison.


def _preprocess_line(
    line: str,
    *,
    ignore_case: bool = False,
    ignore_space_change: bool = False,
    ignore_all_space: bool = False,
) -> str:
    """Preprocess a single line according to diff flags.

    The preprocessing is applied in this order:

    1. Remove all whitespace (``-w``) -- takes priority over ``-b``
    2. Collapse whitespace runs (``-b``)
    3. Fold case (``-i``)

    Args:
        line: The original line.
        ignore_case: If True, fold to lowercase.
        ignore_space_change: If True, collapse runs of whitespace.
        ignore_all_space: If True, remove all whitespace.

    Returns:
        The preprocessed line.
    """
    if ignore_all_space:
        line = "".join(line.split())
    elif ignore_space_change:
        # Collapse runs of whitespace to a single space, strip trailing.
        parts = line.split()
        line = " ".join(parts)
    if ignore_case:
        line = line.lower()
    return line


def _preprocess_lines(
    lines: list[str],
    *,
    ignore_case: bool = False,
    ignore_space_change: bool = False,
    ignore_all_space: bool = False,
    ignore_blank_lines: bool = False,
) -> list[str]:
    """Preprocess a list of lines for comparison.

    Args:
        lines: The raw lines (with newlines stripped).
        ignore_case: Fold case for comparison.
        ignore_space_change: Collapse whitespace runs.
        ignore_all_space: Remove all whitespace.
        ignore_blank_lines: Remove blank lines before comparing.

    Returns:
        The preprocessed list of lines.
    """
    result: list[str] = []
    for line in lines:
        if ignore_blank_lines and line.strip() == "":
            continue
        result.append(
            _preprocess_line(
                line,
                ignore_case=ignore_case,
                ignore_space_change=ignore_space_change,
                ignore_all_space=ignore_all_space,
            )
        )
    return result


# ---------------------------------------------------------------------------
# Read a file into lines.
# ---------------------------------------------------------------------------


def _read_file(path: str) -> list[str]:
    """Read a file and return its lines with newlines stripped.

    If the path is ``-``, read from stdin.

    Args:
        path: Path to the file, or ``-`` for stdin.

    Returns:
        List of lines without trailing newlines.

    Raises:
        FileNotFoundError: If the file does not exist.
    """
    if path == "-":
        return [line.rstrip("\n") for line in sys.stdin]
    with open(path) as f:
        return [line.rstrip("\n") for line in f]


# ---------------------------------------------------------------------------
# Normal diff output format.
# ---------------------------------------------------------------------------
# Normal diff is the oldest and simplest format. It shows change commands
# using ``a`` (add), ``d`` (delete), and ``c`` (change), followed by the
# affected lines prefixed with ``<`` (from file1) or ``>`` (from file2).


def _format_range(start: int, end: int) -> str:
    """Format a line range for normal diff output.

    Normal diff uses 1-based line numbers. A single line is shown as
    just the number; a range is shown as ``start,end``.

    Examples:
        >>> _format_range(0, 1)
        '1'
        >>> _format_range(0, 3)
        '1,3'
    """
    if end - start == 1:
        return str(start + 1)
    return f"{start + 1},{end}"


def normal_diff(lines1: list[str], lines2: list[str]) -> list[str]:
    """Produce a normal diff between two lists of lines.

    This is the default output format for ``diff``. It uses the
    SequenceMatcher to find matching blocks and reports the gaps
    between them.

    Args:
        lines1: Lines from the first file.
        lines2: Lines from the second file.

    Returns:
        List of output lines (without trailing newlines).
    """
    output: list[str] = []
    matcher = difflib.SequenceMatcher(None, lines1, lines2)

    for tag, i1, i2, j1, j2 in matcher.get_opcodes():
        if tag == "equal":
            continue
        elif tag == "replace":
            output.append(f"{_format_range(i1, i2)}c{_format_range(j1, j2)}")
            for line in lines1[i1:i2]:
                output.append(f"< {line}")
            output.append("---")
            for line in lines2[j1:j2]:
                output.append(f"> {line}")
        elif tag == "delete":
            output.append(f"{_format_range(i1, i2)}d{j1}")
            for line in lines1[i1:i2]:
                output.append(f"< {line}")
        elif tag == "insert":
            output.append(f"{i1}a{_format_range(j1, j2)}")
            for line in lines2[j1:j2]:
                output.append(f"> {line}")

    return output


# ---------------------------------------------------------------------------
# Unified diff output format.
# ---------------------------------------------------------------------------


def unified_diff(
    lines1: list[str],
    lines2: list[str],
    file1: str,
    file2: str,
    context: int = 3,
) -> list[str]:
    """Produce unified diff output.

    Unified diff is the most widely used format. It shows changes with
    ``-`` for removed lines and ``+`` for added lines, surrounded by
    context lines prefixed with a space.

    Args:
        lines1: Lines from the first file.
        lines2: Lines from the second file.
        file1: Name/path of the first file (for the header).
        file2: Name/path of the second file (for the header).
        context: Number of context lines (default 3).

    Returns:
        List of output lines.
    """
    result = list(
        difflib.unified_diff(
            lines1,
            lines2,
            fromfile=file1,
            tofile=file2,
            lineterm="",
            n=context,
        )
    )
    return result


# ---------------------------------------------------------------------------
# Context diff output format.
# ---------------------------------------------------------------------------


def context_diff(
    lines1: list[str],
    lines2: list[str],
    file1: str,
    file2: str,
    context: int = 3,
) -> list[str]:
    """Produce context diff output.

    Context diff shows the changes with ``!`` for modified lines,
    ``-`` for removed lines, and ``+`` for added lines. Each file's
    changes are shown in separate sections.

    Args:
        lines1: Lines from the first file.
        lines2: Lines from the second file.
        file1: Name of the first file.
        file2: Name of the second file.
        context: Number of context lines.

    Returns:
        List of output lines.
    """
    result = list(
        difflib.context_diff(
            lines1,
            lines2,
            fromfile=file1,
            tofile=file2,
            lineterm="",
            n=context,
        )
    )
    return result


# ---------------------------------------------------------------------------
# Brief (quiet) diff -- just report whether files differ.
# ---------------------------------------------------------------------------


def brief_diff(file1: str, file2: str) -> str | None:
    """Report whether two files differ without showing the differences.

    This is the ``-q`` / ``--brief`` mode. It only says whether the
    files differ, not how.

    Args:
        file1: Path to the first file.
        file2: Path to the second file.

    Returns:
        A message string if files differ, None if they are identical.
    """
    try:
        lines1 = _read_file(file1)
        lines2 = _read_file(file2)
    except FileNotFoundError:
        return "diff: cannot compare: file not found"

    if lines1 != lines2:
        return f"Files {file1} and {file2} differ"
    return None


# ---------------------------------------------------------------------------
# Recursive directory comparison.
# ---------------------------------------------------------------------------


def diff_directories(
    dir1: str,
    dir2: str,
    *,
    ignore_case: bool = False,
    ignore_space_change: bool = False,
    ignore_all_space: bool = False,
    ignore_blank_lines: bool = False,
    brief: bool = False,
    output_format: str = "normal",
    context_lines: int = 3,
    exclude_patterns: list[str] | None = None,
    new_file: bool = False,
) -> list[str]:
    """Recursively compare two directories.

    This walks both directory trees and compares files with matching
    names. Files present in only one directory are reported.

    Args:
        dir1: Path to the first directory.
        dir2: Path to the second directory.
        ignore_case: Fold case for comparison.
        ignore_space_change: Collapse whitespace.
        ignore_all_space: Remove all whitespace.
        ignore_blank_lines: Ignore blank lines.
        brief: Only report whether files differ.
        output_format: "normal", "unified", or "context".
        context_lines: Number of context lines for unified/context.
        exclude_patterns: Patterns to exclude from comparison.
        new_file: Treat absent files as empty.

    Returns:
        List of output lines.
    """
    output: list[str] = []

    # Collect all entries from both directories.
    entries1 = set(os.listdir(dir1)) if os.path.isdir(dir1) else set()
    entries2 = set(os.listdir(dir2)) if os.path.isdir(dir2) else set()

    # Filter excluded patterns.
    if exclude_patterns:
        for pattern in exclude_patterns:
            entries1 = {e for e in entries1 if not fnmatch.fnmatch(e, pattern)}
            entries2 = {e for e in entries2 if not fnmatch.fnmatch(e, pattern)}

    all_entries = sorted(entries1 | entries2)

    for entry in all_entries:
        path1 = os.path.join(dir1, entry)
        path2 = os.path.join(dir2, entry)
        in1 = entry in entries1
        in2 = entry in entries2

        if in1 and not in2:
            if new_file:
                # Treat absent file as empty.
                if os.path.isfile(path1):
                    result = diff_files(
                        path1, path2,
                        ignore_case=ignore_case,
                        ignore_space_change=ignore_space_change,
                        ignore_all_space=ignore_all_space,
                        ignore_blank_lines=ignore_blank_lines,
                        brief=brief,
                        output_format=output_format,
                        context_lines=context_lines,
                        treat_absent_as_empty=True,
                    )
                    output.extend(result)
            else:
                output.append(f"Only in {dir1}: {entry}")
        elif in2 and not in1:
            if new_file:
                if os.path.isfile(path2):
                    result = diff_files(
                        path1, path2,
                        ignore_case=ignore_case,
                        ignore_space_change=ignore_space_change,
                        ignore_all_space=ignore_all_space,
                        ignore_blank_lines=ignore_blank_lines,
                        brief=brief,
                        output_format=output_format,
                        context_lines=context_lines,
                        treat_absent_as_empty=True,
                    )
                    output.extend(result)
            else:
                output.append(f"Only in {dir2}: {entry}")
        elif os.path.isdir(path1) and os.path.isdir(path2):
            # Recurse into subdirectories.
            sub_output = diff_directories(
                path1, path2,
                ignore_case=ignore_case,
                ignore_space_change=ignore_space_change,
                ignore_all_space=ignore_all_space,
                ignore_blank_lines=ignore_blank_lines,
                brief=brief,
                output_format=output_format,
                context_lines=context_lines,
                exclude_patterns=exclude_patterns,
                new_file=new_file,
            )
            output.extend(sub_output)
        elif os.path.isfile(path1) and os.path.isfile(path2):
            result = diff_files(
                path1, path2,
                ignore_case=ignore_case,
                ignore_space_change=ignore_space_change,
                ignore_all_space=ignore_all_space,
                ignore_blank_lines=ignore_blank_lines,
                brief=brief,
                output_format=output_format,
                context_lines=context_lines,
            )
            output.extend(result)
        else:
            output.append(
                f"File {path1} is a {_file_type(path1)} while "
                f"file {path2} is a {_file_type(path2)}"
            )

    return output


def _file_type(path: str) -> str:
    """Return a human-readable file type description."""
    if os.path.isdir(path):
        return "directory"
    if os.path.islink(path):
        return "symbolic link"
    return "regular file"


# ---------------------------------------------------------------------------
# Main diff function -- compare two files.
# ---------------------------------------------------------------------------


def diff_files(
    file1: str,
    file2: str,
    *,
    ignore_case: bool = False,
    ignore_space_change: bool = False,
    ignore_all_space: bool = False,
    ignore_blank_lines: bool = False,
    brief: bool = False,
    output_format: str = "normal",
    context_lines: int = 3,
    treat_absent_as_empty: bool = False,
) -> list[str]:
    """Compare two files and return the diff output.

    This is the main comparison function. It reads both files,
    preprocesses the lines, and produces the output in the requested
    format.

    Args:
        file1: Path to the first file.
        file2: Path to the second file.
        ignore_case: Fold case for comparison.
        ignore_space_change: Collapse whitespace runs.
        ignore_all_space: Remove all whitespace.
        ignore_blank_lines: Remove blank lines.
        brief: Only report whether files differ.
        output_format: "normal", "unified", or "context".
        context_lines: Number of context lines.
        treat_absent_as_empty: If True, treat missing files as empty.

    Returns:
        List of output lines.
    """
    # --- Read files ---
    try:
        lines1 = _read_file(file1)
    except FileNotFoundError:
        if treat_absent_as_empty:
            lines1 = []
        else:
            return [f"diff: {file1}: No such file or directory"]

    try:
        lines2 = _read_file(file2)
    except FileNotFoundError:
        if treat_absent_as_empty:
            lines2 = []
        else:
            return [f"diff: {file2}: No such file or directory"]

    # --- Brief mode ---
    if brief:
        if lines1 != lines2:
            return [f"Files {file1} and {file2} differ"]
        return []

    # --- Preprocess ---
    proc1 = _preprocess_lines(
        lines1,
        ignore_case=ignore_case,
        ignore_space_change=ignore_space_change,
        ignore_all_space=ignore_all_space,
        ignore_blank_lines=ignore_blank_lines,
    )
    proc2 = _preprocess_lines(
        lines2,
        ignore_case=ignore_case,
        ignore_space_change=ignore_space_change,
        ignore_all_space=ignore_all_space,
        ignore_blank_lines=ignore_blank_lines,
    )

    # --- Generate diff ---
    if output_format == "unified":
        return unified_diff(proc1, proc2, file1, file2, context_lines)
    elif output_format == "context":
        return context_diff(proc1, proc2, file1, file2, context_lines)
    else:
        return normal_diff(proc1, proc2)


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then diff files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"diff: {error.message}", file=sys.stderr)
        raise SystemExit(2) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # --- Extract flags ---
    ignore_case = result.flags.get("ignore_case", False)
    ignore_space_change = result.flags.get("ignore_space_change", False)
    ignore_all_space = result.flags.get("ignore_all_space", False)
    ignore_blank_lines = result.flags.get("ignore_blank_lines", False)
    brief = result.flags.get("brief", False)
    recursive = result.flags.get("recursive", False)
    new_file = result.flags.get("new_file", False)
    exclude = result.flags.get("exclude", None)
    if isinstance(exclude, list):
        exclude_patterns = exclude
    else:
        exclude_patterns = [exclude] if exclude else None

    # Determine output format.
    if result.flags.get("unified"):
        output_format = "unified"
        context_lines = result.flags.get("unified", 3)
    elif result.flags.get("context_format"):
        output_format = "context"
        context_lines = result.flags.get("context_format", 3)
    else:
        output_format = "normal"
        context_lines = 3

    # --- Extract arguments ---
    file1 = result.arguments["file1"]
    file2 = result.arguments["file2"]

    # --- Perform comparison ---
    if recursive and os.path.isdir(file1) and os.path.isdir(file2):
        output = diff_directories(
            file1, file2,
            ignore_case=ignore_case,
            ignore_space_change=ignore_space_change,
            ignore_all_space=ignore_all_space,
            ignore_blank_lines=ignore_blank_lines,
            brief=brief,
            output_format=output_format,
            context_lines=context_lines,
            exclude_patterns=exclude_patterns,
            new_file=new_file,
        )
    else:
        output = diff_files(
            file1, file2,
            ignore_case=ignore_case,
            ignore_space_change=ignore_space_change,
            ignore_all_space=ignore_all_space,
            ignore_blank_lines=ignore_blank_lines,
            brief=brief,
            output_format=output_format,
            context_lines=context_lines,
        )

    if output:
        for line in output:
            print(line)
        raise SystemExit(1)

    raise SystemExit(0)


if __name__ == "__main__":
    main()
