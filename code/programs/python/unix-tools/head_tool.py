"""head — output the first part of files.

=== What This Program Does ===

This is a reimplementation of the GNU ``head`` utility. It reads one or more
files (or standard input) and prints the first N lines (default 10) or the
first N bytes to standard output.

=== How It Works ===

The core algorithm is straightforward:

1. Open each input source (file or stdin).
2. Read either the first N lines or the first N bytes, depending on
   which flag was given (``-n`` for lines, ``-c`` for bytes).
3. Write the selected content to stdout.

When multiple files are given, a header line is printed before each file's
output (unless ``-q`` is specified). The header format is::

    ==> filename <==

This matches the behavior of GNU head.

=== Lines vs Bytes ===

The ``-n`` and ``-c`` flags are mutually exclusive:

- ``-n NUM`` (default: 10): Print the first NUM lines. A "line" is
  terminated by a newline character (``\\n``). If the file has fewer
  than NUM lines, all lines are printed.

- ``-c NUM``: Print the first NUM bytes. This is useful for binary files
  or when you need an exact byte count rather than line-based output.

=== Headers ===

When processing multiple files, head prints a header before each file.
The ``-q`` flag suppresses these headers, and ``-v`` forces them even
for a single file.

=== CLI Builder Integration ===

The entire CLI is defined in ``head.json``. CLI Builder handles flag parsing,
help text, version output, and mutual exclusivity enforcement. This file
contains only the business logic for reading and outputting file content.
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

SPEC_FILE = str(Path(__file__).parent / "head.json")


def read_input_lines(filename: str) -> list[str]:
    """Read all lines from a file or stdin.

    If the filename is ``-``, read from standard input. Otherwise, open
    the file and read its lines. Lines retain their trailing newline
    characters so we can reproduce the original formatting exactly.

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Returns:
        A list of lines (each ending with ``\\n`` except possibly the last).

    Raises:
        SystemExit: If the file cannot be opened.
    """
    if filename == "-":
        try:
            content = sys.stdin.read()
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        return content.splitlines(keepends=True) if content else []

    try:
        with open(filename) as f:
            return f.readlines()
    except FileNotFoundError:
        print(f"head: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"head: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"head: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def read_input_bytes(filename: str) -> bytes:
    """Read all bytes from a file or stdin.

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Returns:
        The raw bytes of the file content.

    Raises:
        SystemExit: If the file cannot be opened.
    """
    if filename == "-":
        try:
            return sys.stdin.buffer.read()
        except KeyboardInterrupt:
            raise SystemExit(130) from None

    try:
        return Path(filename).read_bytes()
    except FileNotFoundError:
        print(f"head: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"head: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"head: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def head_lines(lines: list[str], count: int) -> str:
    """Return the first ``count`` lines from the given list.

    This is the core business logic for line-based head. It simply
    slices the list and joins the results.

    Args:
        lines: All lines from the input.
        count: How many lines to keep.

    Returns:
        The concatenation of the first ``count`` lines.
    """
    return "".join(lines[:count])


def head_bytes(data: bytes, count: int) -> bytes:
    """Return the first ``count`` bytes from the given data.

    Args:
        data: The raw input bytes.
        count: How many bytes to keep.

    Returns:
        The first ``count`` bytes.
    """
    return data[:count]


def should_print_header(
    *,
    num_files: int,
    quiet: bool,
    verbose: bool,
) -> bool:
    """Determine whether to print the ``==> filename <==`` header.

    The rules match GNU head:
    - If ``-q`` is given, never print headers.
    - If ``-v`` is given, always print headers.
    - Otherwise, print headers only when there are multiple files.

    Args:
        num_files: How many files are being processed.
        quiet: Whether the ``-q`` flag was given.
        verbose: Whether the ``-v`` flag was given.

    Returns:
        True if headers should be printed.
    """
    if quiet:
        return False
    if verbose:
        return True
    return num_files > 1


def main() -> None:
    """Entry point: parse args via CLI Builder, then output first lines/bytes."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"head: {error.message}", file=sys.stderr)
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

    # Extract flags.
    num_lines = result.flags.get("lines", 10)
    num_bytes = result.flags.get("bytes")
    quiet = result.flags.get("quiet", False)
    verbose = result.flags.get("verbose", False)
    use_bytes = num_bytes is not None

    # Get the list of files.
    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]
    if not files:
        files = ["-"]

    print_header = should_print_header(
        num_files=len(files), quiet=quiet, verbose=verbose
    )

    for i, filename in enumerate(files):
        # Print separator between files.
        if print_header:
            if i > 0:
                print()
            display_name = "standard input" if filename == "-" else filename
            print(f"==> {display_name} <==")

        if use_bytes:
            data = read_input_bytes(filename)
            sys.stdout.buffer.write(head_bytes(data, num_bytes))
            sys.stdout.buffer.flush()
        else:
            lines = read_input_lines(filename)
            sys.stdout.write(head_lines(lines, num_lines))


if __name__ == "__main__":
    main()
