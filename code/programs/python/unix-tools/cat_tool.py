"""cat — concatenate files and print on the standard output.

=== What This Program Does ===

This is a reimplementation of the ``cat`` utility. It reads one or more
files (or standard input) and writes their contents to standard output.
The name "cat" is short for "concatenate" — when given multiple files,
it joins them end-to-end.

=== Flags Overview ===

cat supports several flags that transform the output:

-n    Number all output lines, starting from 1. Each line is prefixed
      with a right-justified 6-character line number followed by a tab.
      Example::

          $ echo -e "hello\\nworld" | cat -n
               1\\thello
               2\\tworld

-b    Number only non-blank lines. This overrides ``-n`` if both are
      given. Blank lines are printed without a number.

-s    Squeeze multiple adjacent blank lines into a single blank line.
      This is useful for cleaning up files with excessive whitespace.

-E    Show a ``$`` at the end of each line. This makes trailing spaces
      and line endings visible — very useful for debugging.

-T    Display tab characters as ``^I`` instead of actual tabs. This uses
      the caret notation common in Unix tools.

-A    Equivalent to ``-vET``. Shows all non-printing characters, tabs,
      and line endings. This is the "show everything" mode.

-v    Show non-printing characters using ^ and M- notation, except for
      line feed (LFD) and tab (TAB). Control characters (0x00-0x1F)
      are shown as ^@ through ^_, and DEL (0x7F) as ^?. High bytes
      (0x80-0xFF) use M- prefix notation.

=== How Line Processing Works ===

The core algorithm processes input line by line:

1. Read all bytes from each input source (file or stdin).
2. Split into lines (keeping track of line boundaries).
3. For each line, apply transformations in order:
   a. Squeeze blank lines (if -s)
   b. Number the line (if -n or -b)
   c. Replace tabs with ^I (if -T)
   d. Show non-printing characters (if -v)
   e. Append $ at end (if -E)
4. Write the transformed line to stdout.

=== CLI Builder Integration ===

The entire CLI is defined in ``cat.json``. CLI Builder handles flag parsing,
help text, version output, and error messages. This file contains only the
concatenation and transformation logic.
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

SPEC_FILE = str(Path(__file__).parent / "cat.json")


def show_nonprinting(char: str) -> str:
    """Convert a non-printing character to ^ or M- notation.

    This function implements the standard Unix convention for displaying
    non-printing characters:

    - Control characters (0x00-0x1F) become ^@ through ^_
      For example: NUL=^@, BEL=^G, ESC=^[
    - DEL (0x7F) becomes ^?
    - High bytes (0x80-0xFF) use M- prefix, then apply the above rules
      to the low 7 bits. For example: 0x80=M-^@, 0xC1=M-A

    Tabs and newlines are NOT converted — they pass through unchanged
    so that line structure is preserved. Tabs are handled separately
    by the -T flag.

    Args:
        char: A single character to potentially convert.

    Returns:
        The character in printable notation, or unchanged if already printable.
    """
    code = ord(char)

    # --- Tab and newline pass through unchanged. ---
    if char in ("\t", "\n"):
        return char

    # --- High bytes (128-255): M- prefix + convert the low 7 bits. ---
    if code >= 128:
        # Strip the high bit and recursively convert.
        low = code - 128
        if low < 32:
            return f"M-^{chr(low + 64)}"
        if low == 127:
            return "M-^?"
        return f"M-{chr(low)}"

    # --- Control characters (0-31): caret notation. ---
    if code < 32:
        return f"^{chr(code + 64)}"

    # --- DEL (127): special case. ---
    if code == 127:
        return "^?"

    # --- Regular printable character: return as-is. ---
    return char


def process_line(
    line: str,
    *,
    show_ends: bool,
    show_tabs: bool,
    show_nonprinting_flag: bool,
) -> str:
    """Apply character-level transformations to a single line.

    This function handles three transformations that operate on individual
    characters within a line:

    1. Non-printing character conversion (-v): Replace control characters
       and high bytes with readable ^/M- notation.
    2. Tab display (-T): Replace tab characters with ^I.
    3. End-of-line marker (-E): Append $ before the newline.

    The order matters: we process non-printing characters first (which
    leaves tabs alone), then handle tabs separately, then add the end
    marker.

    Args:
        line: The raw line of text (may or may not end with newline).
        show_ends: Whether to show $ at end of each line.
        show_tabs: Whether to display tabs as ^I.
        show_nonprinting_flag: Whether to show non-printing chars as ^/M-.

    Returns:
        The transformed line.
    """
    # Separate the trailing newline (if any) from the content.
    # We need to handle the newline separately because:
    # - The $ marker goes before the newline
    # - The newline itself should not be converted by -v
    has_newline = line.endswith("\n")
    content = line[:-1] if has_newline else line

    # --- Step 1: Non-printing character conversion (-v) ---
    if show_nonprinting_flag:
        content = "".join(show_nonprinting(c) for c in content)

    # --- Step 2: Tab conversion (-T) ---
    if show_tabs:
        content = content.replace("\t", "^I")

    # --- Step 3: End-of-line marker (-E) ---
    if show_ends:
        content += "$"

    # Re-attach the newline if the original line had one.
    if has_newline:
        content += "\n"

    return content


def cat_stream(
    stream: list[str],
    *,
    number: bool,
    number_nonblank: bool,
    squeeze_blank: bool,
    show_ends: bool,
    show_tabs: bool,
    show_nonprinting_flag: bool,
    line_counter: int,
) -> int:
    """Process and output lines from a single input stream.

    This is the core processing function. It takes a list of lines and
    applies all requested transformations, writing each line to stdout.

    The line_counter parameter allows numbering to continue across
    multiple files — if you ``cat -n file1 file2``, line numbering
    doesn't restart at 1 for file2.

    Args:
        stream: List of lines to process.
        number: Whether to number all lines (-n).
        number_nonblank: Whether to number only non-blank lines (-b).
        squeeze_blank: Whether to squeeze consecutive blank lines (-s).
        show_ends: Whether to show $ at end of lines (-E).
        show_tabs: Whether to show tabs as ^I (-T).
        show_nonprinting_flag: Whether to show non-printing chars (-v).
        line_counter: Starting line number (for continuity across files).

    Returns:
        The updated line counter (so the next file can continue numbering).
    """
    # Track whether the previous line was blank, for squeeze logic.
    prev_blank = False

    for line in stream:
        # --- Squeeze blank lines (-s) ---
        # A "blank" line is one that contains only a newline (or is empty).
        is_blank = line == "\n" or line == ""

        if squeeze_blank and is_blank:
            if prev_blank:
                # Skip this line — we already printed one blank line.
                continue
            prev_blank = True
        else:
            prev_blank = False

        # --- Apply character-level transformations ---
        line = process_line(
            line,
            show_ends=show_ends,
            show_tabs=show_tabs,
            show_nonprinting_flag=show_nonprinting_flag,
        )

        # --- Line numbering (-n or -b) ---
        # -b overrides -n: if both are set, only non-blank lines are numbered.
        if number_nonblank:
            if not is_blank:
                line_counter += 1
                # Format: 6-character right-justified number, then tab.
                sys.stdout.write(f"{line_counter:6d}\t{line}")
            else:
                sys.stdout.write(line)
        elif number:
            line_counter += 1
            sys.stdout.write(f"{line_counter:6d}\t{line}")
        else:
            sys.stdout.write(line)

    return line_counter


def read_input(filename: str) -> list[str]:
    """Read all lines from a file or stdin.

    If the filename is ``-``, read from standard input. Otherwise, open
    the file and read its contents. The lines are returned with their
    newline characters intact (so we can distinguish the last line of a
    file that doesn't end with a newline).

    Args:
        filename: Path to the file, or ``-`` for stdin.

    Returns:
        A list of lines (each ending with ``\\n`` except possibly the last).

    Raises:
        SystemExit: If the file cannot be opened (prints error to stderr).
    """
    if filename == "-":
        # Read from stdin. We use sys.stdin.read() to get all content,
        # then splitlines(keepends=True) to preserve newline characters.
        try:
            content = sys.stdin.read()
        except KeyboardInterrupt:
            raise SystemExit(130) from None
        return content.splitlines(keepends=True) if content else []

    try:
        with open(filename) as f:
            return f.readlines()
    except FileNotFoundError:
        print(f"cat: {filename}: No such file or directory", file=sys.stderr)
        raise SystemExit(1) from None
    except PermissionError:
        print(f"cat: {filename}: Permission denied", file=sys.stderr)
        raise SystemExit(1) from None
    except IsADirectoryError:
        print(f"cat: {filename}: Is a directory", file=sys.stderr)
        raise SystemExit(1) from None


def main() -> None:
    """Entry point: parse args via CLI Builder, then concatenate files."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"cat: {error.message}", file=sys.stderr)
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

    # Extract flags. The -A flag is a shorthand for -vET.
    show_all = result.flags.get("show_all", False)
    number = result.flags.get("number", False)
    number_nonblank = result.flags.get("number_nonblank", False)
    squeeze_blank = result.flags.get("squeeze_blank", False)
    show_tabs = result.flags.get("show_tabs", False) or show_all
    show_ends = result.flags.get("show_ends", False) or show_all
    show_nonprinting_flag = result.flags.get("show_nonprinting", False) or show_all

    # Get the list of files to concatenate.
    files = result.arguments.get("files", ["-"])
    if isinstance(files, str):
        files = [files]
    if not files:
        files = ["-"]

    # Process each file, maintaining a continuous line counter.
    line_counter = 0
    for filename in files:
        lines = read_input(filename)
        line_counter = cat_stream(
            lines,
            number=number,
            number_nonblank=number_nonblank,
            squeeze_blank=squeeze_blank,
            show_ends=show_ends,
            show_tabs=show_tabs,
            show_nonprinting_flag=show_nonprinting_flag,
            line_counter=line_counter,
        )


if __name__ == "__main__":
    main()
