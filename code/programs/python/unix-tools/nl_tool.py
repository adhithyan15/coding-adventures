"""nl — number lines of files.

=== What This Program Does ===

This is a reimplementation of the GNU ``nl`` utility. It reads text
and writes it to standard output with line numbers added to the left
margin.

=== Numbering Styles ===

nl supports four numbering styles, specified for each logical section
(header, body, footer):

- ``a`` — Number all lines, including blank ones.
- ``t`` — Number only non-empty lines (default for body).
- ``n`` — Don't number any lines (default for header/footer).
- ``pREGEX`` — Number only lines matching the regular expression.

For example, ``-b a`` numbers all body lines, while ``-b pERROR``
numbers only body lines containing "ERROR".

=== Number Format ===

The ``-n`` flag controls the format of line numbers:

- ``ln`` — Left justified, no leading zeros.
- ``rn`` — Right justified, no leading zeros (default).
- ``rz`` — Right justified, with leading zeros.

The width of the number field is controlled by ``-w`` (default 6),
and the separator between the number and the line is controlled by
``-s`` (default: tab).

=== Logical Pages ===

nl divides input into logical pages, each consisting of header, body,
and footer sections. Section boundaries are marked by delimiter lines:

- ``\\:\\:\\:`` — Start of header.
- ``\\:\\:`` — Start of body.
- ``\\:`` — Start of footer.

The delimiter characters can be changed with ``-d``.

Most files don't use logical pages, so nl treats the entire input as
a single body section.

=== CLI Builder Integration ===

The entire CLI is defined in ``nl.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import re
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "nl.json")


def should_number_line(line: str, style: str) -> bool:
    """Determine whether a line should be numbered.

    This function implements the four numbering styles:
    - 'a': number all lines
    - 't': number non-empty lines only
    - 'n': number no lines
    - 'pREGEX': number lines matching the regex

    Args:
        line: The line content (without trailing newline).
        style: The numbering style specifier.

    Returns:
        True if the line should be numbered.
    """
    if style == "a":
        return True
    if style == "t":
        return len(line.strip()) > 0
    if style == "n":
        return False
    if style.startswith("p"):
        pattern = style[1:]
        return bool(re.search(pattern, line))
    # Unknown style: treat as 'n' (no numbering).
    return False


def format_number(num: int, fmt: str, width: int) -> str:
    """Format a line number according to the specified format.

    Args:
        num: The line number.
        fmt: The format specifier ('ln', 'rn', or 'rz').
        width: The field width for the number.

    Returns:
        The formatted number string.
    """
    num_str = str(num)

    if fmt == "ln":
        # Left justified, no leading zeros.
        return num_str.ljust(width)
    elif fmt == "rz":
        # Right justified, with leading zeros.
        return num_str.zfill(width)
    else:
        # 'rn' — Right justified, no leading zeros (default).
        return num_str.rjust(width)


def detect_section(
    line: str, delim: str
) -> str | None:
    """Detect if a line is a section delimiter.

    Section delimiters are formed by repeating the delimiter string:
    - 3 repetitions = header
    - 2 repetitions = body
    - 1 repetition = footer

    Args:
        line: The line to check (without trailing newline).
        delim: The section delimiter string (default '\\\\:').

    Returns:
        'header', 'body', 'footer', or None.
    """
    stripped = line.rstrip("\n")
    if stripped == delim * 3:
        return "header"
    if stripped == delim * 2:
        return "body"
    if stripped == delim:
        return "footer"
    return None


def number_lines(
    lines: list[str],
    *,
    body_style: str,
    header_style: str,
    footer_style: str,
    start_number: int,
    increment: int,
    number_format: str,
    number_width: int,
    separator: str,
    section_delimiter: str,
) -> list[str]:
    """Number the lines according to all the configuration options.

    This is the core algorithm. It processes each line, checking for
    section boundaries and applying the appropriate numbering style.

    Args:
        lines: The input lines (with trailing newlines stripped).
        body_style: Numbering style for body sections.
        header_style: Numbering style for header sections.
        footer_style: Numbering style for footer sections.
        start_number: The first line number.
        increment: How much to increment the line number.
        number_format: Format for the number ('ln', 'rn', 'rz').
        number_width: Width of the number field.
        separator: String between number and line content.
        section_delimiter: The section boundary marker.

    Returns:
        The numbered lines.
    """
    result: list[str] = []
    current_number = start_number
    current_section = "body"  # Default: everything is body.

    # Map section names to their numbering styles.
    style_map = {
        "header": header_style,
        "body": body_style,
        "footer": footer_style,
    }

    for line in lines:
        # Check for section boundaries.
        section = detect_section(line, section_delimiter)
        if section is not None:
            current_section = section
            # Reset line number at the start of each logical page.
            if section == "header":
                current_number = start_number
            # Section delimiter lines are not printed.
            result.append("")
            continue

        # Determine the current numbering style.
        style = style_map[current_section]

        if should_number_line(line, style):
            # Format the number and prepend it to the line.
            num_str = format_number(current_number, number_format, number_width)
            result.append(f"{num_str}{separator}{line}")
            current_number += increment
        else:
            # No number: emit blank space of the appropriate width.
            blank = " " * number_width
            result.append(f"{blank}{separator}{line}")

    return result


def main() -> None:
    """Entry point: parse args via CLI Builder, then number lines."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"nl: {error.message}", file=sys.stderr)
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

    body_style = result.flags.get("body_numbering", "t")
    header_style = result.flags.get("header_numbering", "n")
    footer_style = result.flags.get("footer_numbering", "n")
    start_number = result.flags.get("starting_line_number", 1)
    line_increment = result.flags.get("line_increment", 1)
    number_format = result.flags.get("number_format", "rn")
    number_width = result.flags.get("number_width", 6)
    separator = result.flags.get("number_separator", "\t")
    section_delimiter = result.flags.get("section_delimiter", "\\:")

    # Get the input file.
    input_file = result.arguments.get("file")

    # Read input.
    try:
        if input_file and input_file != "-":
            with open(input_file) as f:
                raw_lines = f.readlines()
        else:
            raw_lines = sys.stdin.readlines()
    except FileNotFoundError:
        print(
            f"nl: {input_file}: No such file or directory",
            file=sys.stderr,
        )
        raise SystemExit(1) from None
    except KeyboardInterrupt:
        raise SystemExit(130) from None

    # Strip trailing newlines for processing.
    lines = [line.rstrip("\n") for line in raw_lines]

    # Number the lines.
    output = number_lines(
        lines,
        body_style=body_style,
        header_style=header_style,
        footer_style=footer_style,
        start_number=start_number,
        increment=line_increment,
        number_format=number_format,
        number_width=number_width,
        separator=separator,
        section_delimiter=section_delimiter,
    )

    # Write output.
    try:
        for line in output:
            print(line)
    except BrokenPipeError:
        raise SystemExit(0) from None


if __name__ == "__main__":
    main()
