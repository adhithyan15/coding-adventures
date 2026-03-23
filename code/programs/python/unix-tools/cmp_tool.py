"""cmp -- compare two files byte by byte.

=== What This Program Does ===

This is a reimplementation of the POSIX ``cmp`` utility. It compares two
files byte by byte and reports the first position where they differ::

    cmp file1.txt file2.txt
    file1.txt file2.txt differ: byte 42, line 3

=== How Byte Comparison Works ===

Unlike ``diff``, which works on lines and finds a minimal edit script,
``cmp`` does a strict byte-by-byte comparison. It reads both files in
lockstep, comparing corresponding bytes. This makes ``cmp`` faster than
``diff`` for binary files and for quick "are these identical?" checks.

=== Output Modes ===

1. **Default**: Report the first difference (byte offset and line number).

2. **Verbose** (``-l``): List ALL differing bytes, showing the byte
   offset and the octal values from each file::

       42  141  142
       43  143  144

3. **Silent** (``-s``): Produce no output at all. Only the exit status
   matters: 0 = identical, 1 = different, 2 = error.

4. **Print bytes** (``-b``): Show the differing bytes as characters
   alongside their octal values.

=== Skip and Limit ===

- ``-i SKIP``: Skip the first SKIP bytes of both files before comparing.
  This is useful for ignoring headers.
- ``-n LIMIT``: Compare at most LIMIT bytes. This is useful for
  comparing only a prefix of each file.

=== Exit Status ===

+--------+-----------------------------------------------+
| Status | Meaning                                       |
+========+===============================================+
| 0      | Files are identical                            |
| 1      | Files differ                                  |
| 2      | An error occurred (e.g., file not found)       |
+--------+-----------------------------------------------+

=== CLI Builder Integration ===

The entire CLI is defined in ``cmp.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "cmp.json")


# ---------------------------------------------------------------------------
# Business logic -- compare two files byte by byte.
# ---------------------------------------------------------------------------


def cmp_files(
    file1: str,
    file2: str,
    *,
    print_bytes: bool = False,
    verbose: bool = False,
    silent: bool = False,
    skip: int = 0,
    max_bytes: int | None = None,
) -> tuple[int, list[str]]:
    """Compare two files byte by byte.

    This is the core comparison function. It reads both files and
    compares them byte by byte, respecting the various output modes.

    Args:
        file1: Path to the first file (or ``-`` for stdin).
        file2: Path to the second file (or ``-`` for stdin).
        print_bytes: If True, show differing bytes as characters.
        verbose: If True, list all differences (not just the first).
        silent: If True, produce no output.
        skip: Number of bytes to skip at the start of both files.
        max_bytes: Maximum number of bytes to compare.

    Returns:
        A tuple of (exit_code, output_lines).

        - exit_code 0: files are identical
        - exit_code 1: files differ
        - exit_code 2: an error occurred

    How the comparison loop works
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~

    We read both files entirely into memory (as bytes), then iterate
    through them in lockstep. We track both the byte offset (1-based)
    and the line number (counting newline bytes in file1).

    +-----------+--------------------------------------------------+
    | Variable  | Purpose                                          |
    +===========+==================================================+
    | offset    | Current byte position (1-based, after skip)      |
    | line      | Current line number (counting ``\\n`` in file1)  |
    | data1[i]  | Byte from file1 at position i                    |
    | data2[i]  | Byte from file2 at position i                    |
    +-----------+--------------------------------------------------+
    """
    output: list[str] = []

    # --- Read file contents ---
    try:
        if file1 == "-":
            data1 = sys.stdin.buffer.read()
        else:
            with open(file1, "rb") as f:
                data1 = f.read()
    except FileNotFoundError:
        if not silent:
            output.append(f"cmp: {file1}: No such file or directory")
        return 2, output
    except PermissionError:
        if not silent:
            output.append(f"cmp: {file1}: Permission denied")
        return 2, output

    try:
        if file2 == "-":
            data2 = sys.stdin.buffer.read()
        else:
            with open(file2, "rb") as f:
                data2 = f.read()
    except FileNotFoundError:
        if not silent:
            output.append(f"cmp: {file2}: No such file or directory")
        return 2, output
    except PermissionError:
        if not silent:
            output.append(f"cmp: {file2}: Permission denied")
        return 2, output

    # --- Apply skip ---
    data1 = data1[skip:]
    data2 = data2[skip:]

    # --- Apply max_bytes limit ---
    if max_bytes is not None:
        data1 = data1[:max_bytes]
        data2 = data2[:max_bytes]

    # --- Compare ---
    min_len = min(len(data1), len(data2))
    line = 1
    found_diff = False

    for i in range(min_len):
        if data1[i] != data2[i]:
            found_diff = True
            byte_offset = skip + i + 1  # 1-based

            if silent:
                return 1, []

            if verbose:
                # Verbose mode: list all differences.
                if print_bytes:
                    output.append(
                        f"{byte_offset:6d} {data1[i]:3o} {chr(data1[i])!s:>3s}"
                        f" {data2[i]:3o} {chr(data2[i])!s:>3s}"
                    )
                else:
                    output.append(
                        f"{byte_offset:6d} {data1[i]:3o} {data2[i]:3o}"
                    )
            else:
                # Default mode: report first difference and stop.
                if print_bytes:
                    output.append(
                        f"{file1} {file2} differ: byte {byte_offset}, "
                        f"line {line} is {data1[i]:3o} {chr(data1[i])!s}"
                        f" {data2[i]:3o} {chr(data2[i])!s}"
                    )
                else:
                    output.append(
                        f"{file1} {file2} differ: byte {byte_offset}, "
                        f"line {line}"
                    )
                return 1, output

        # Track line numbers by counting newlines in file1.
        if data1[i] == ord("\n"):
            line += 1

    # --- Check for length difference ---
    if len(data1) != len(data2):
        if silent:
            return 1, []
        shorter = file1 if len(data1) < len(data2) else file2
        if not verbose or not found_diff:
            output.append(f"cmp: EOF on {shorter}")
        return 1, output

    if found_diff:
        return 1, output
    return 0, output


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then compare files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"cmp: {error.message}", file=sys.stderr)
        raise SystemExit(2) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # --- Extract flags ---
    print_bytes = result.flags.get("print_bytes", False)
    verbose = result.flags.get("list", False)
    silent = result.flags.get("silent", False)
    ignore_initial = result.flags.get("ignore_initial", None)
    max_bytes = result.flags.get("max_bytes", None)

    # Parse skip value.
    skip = 0
    if ignore_initial:
        try:
            skip = int(ignore_initial)
        except ValueError:
            msg = f"cmp: invalid --ignore-initial value: {ignore_initial}"
            print(msg, file=sys.stderr)
            raise SystemExit(2) from None

    # --- Extract arguments ---
    file1 = result.arguments["file1"]
    file2 = result.arguments.get("file2", "-")

    # --- Compare ---
    exit_code, output = cmp_files(
        file1, file2,
        print_bytes=print_bytes,
        verbose=verbose,
        silent=silent,
        skip=skip,
        max_bytes=max_bytes,
    )

    for line in output:
        print(line)

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
