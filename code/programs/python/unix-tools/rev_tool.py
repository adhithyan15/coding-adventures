"""rev — reverse lines characterwise.

=== What This Program Does ===

This is a reimplementation of the ``rev`` utility. It reads lines from
files or standard input and writes each line to standard output with the
characters in reverse order.

For example::

    $ echo "Hello, World!" | rev
    !dlroW ,olleH

    $ echo -e "abc\\n123" | rev
    cba
    321

=== How It Works ===

The algorithm is beautifully simple:

1. Read each line of input (preserving the trailing newline).
2. Reverse the non-newline characters.
3. Re-attach the newline (if present).
4. Write to stdout.

The newline is preserved in its original position (at the end) so that
the output maintains proper line structure. Without this special handling,
the newline would appear at the *beginning* of the reversed line.

=== Why rev is Useful ===

rev seems like a toy, but it has practical uses in shell scripting:

- Extracting the last field when you don't know how many fields there are::

      $ echo "/usr/local/bin/python" | rev | cut -d/ -f1 | rev
      python

- Reversing the order of characters for palindrome checking.

- Quick text transformations in pipelines.

=== CLI Builder Integration ===

The entire CLI is defined in ``rev.json``. rev has no flags — it's one
of the simplest Unix tools. CLI Builder still handles ``--help`` and
``--version``.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "rev.json")


def reverse_line(line: str) -> str:
    """Reverse the characters in a line, preserving the trailing newline.

    The trailing newline (if present) stays at the end of the output.
    Only the content characters are reversed.

    For example:
    - ``"hello\\n"`` becomes ``"olleh\\n"``
    - ``"hello"`` becomes ``"olleh"``
    - ``"\\n"`` stays ``"\\n"`` (reversing nothing gives nothing)

    Args:
        line: A single line of text, possibly ending with a newline.

    Returns:
        The line with its content characters reversed.
    """
    if line.endswith("\n"):
        # Reverse everything except the trailing newline, then re-add it.
        return line[:-1][::-1] + "\n"
    # No trailing newline: reverse the entire string.
    return line[::-1]


def read_and_reverse(filename: str) -> None:
    """Read a file (or stdin) and print each line reversed.

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Raises:
        SystemExit: If the file cannot be opened.
    """
    if filename == "-":
        try:
            for line in sys.stdin:
                sys.stdout.write(reverse_line(line))
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        return

    try:
        with open(filename) as f:
            for line in f:
                sys.stdout.write(reverse_line(line))
    except FileNotFoundError:
        print(f"rev: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"rev: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"rev: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def main() -> None:
    """Entry point: parse args via CLI Builder, then reverse lines."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"rev: {error.message}", file=sys.stderr)
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

    # Get the list of files.
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]
    if not files:
        files = ["-"]

    for filename in files:
        read_and_reverse(filename)


if __name__ == "__main__":
    main()
