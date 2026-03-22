"""echo — display a line of text.

=== What This Program Does ===

This is a reimplementation of the ``echo`` utility. It writes its arguments
to standard output, separated by spaces, followed by a newline. Simple as
that — most of the time.

=== The Three Flags ===

``echo`` supports three flags that modify its output:

-n    Suppress the trailing newline. Normally ``echo hello`` prints
      ``hello\\n``. With ``-n``, it prints just ``hello`` with no newline
      at the end. This is useful when building prompts or partial output::

          echo -n "Enter your name: "
          read name

-e    Enable interpretation of backslash escapes. When this flag is set,
      sequences like ``\\n`` (newline), ``\\t`` (tab), and ``\\\\``
      (literal backslash) are interpreted. Without ``-e``, the string
      ``\\n`` prints as the two characters ``\\`` and ``n``.

-E    Disable interpretation of backslash escapes (the default). This is
      the explicit way to say "don't interpret escapes." It exists so
      you can override ``-e`` if both appear in an alias.

=== Backslash Escape Reference ===

When ``-e`` is active, these escape sequences are recognized:

    \\\\     backslash
    \\a     alert (bell) — ASCII 0x07
    \\b     backspace — ASCII 0x08
    \\f     form feed — ASCII 0x0C
    \\n     newline — ASCII 0x0A
    \\r     carriage return — ASCII 0x0D
    \\t     horizontal tab — ASCII 0x09
    \\0nnn  octal value (1-3 digits after the zero)

=== CLI Builder Integration ===

The entire CLI — flags ``-n``, ``-e``, ``-E``, help text, version output —
is defined in ``echo.json``. This program never parses a single argument
by hand. CLI Builder handles all of that, and we just implement the
output logic.
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

SPEC_FILE = str(Path(__file__).parent / "echo.json")


# ---------------------------------------------------------------------------
# Backslash escape processing
# ---------------------------------------------------------------------------

# This mapping defines simple one-character escape sequences.
# Each maps a single character after a backslash to its replacement.
ESCAPE_MAP: dict[str, str] = {
    "\\": "\\",  # \\ -> literal backslash
    "a": "\a",   # \a -> alert (bell)
    "b": "\b",   # \b -> backspace
    "f": "\f",   # \f -> form feed
    "n": "\n",   # \n -> newline
    "r": "\r",   # \r -> carriage return
    "t": "\t",   # \t -> horizontal tab
}


def process_escapes(text: str) -> str:
    r"""Interpret backslash escape sequences in the given text.

    This function walks through the string character by character. When it
    encounters a backslash, it looks at the next character to decide what
    to output:

    - If the next character is in ESCAPE_MAP, output the mapped value.
    - If the next character is '0', read up to 3 octal digits and output
      the corresponding character (e.g., ``\\0101`` -> ``A``).
    - If the next character is anything else, output the backslash and
      the character as-is.

    This manual parsing approach (rather than using str.translate or regex)
    is deliberate: it matches the exact behavior of GNU echo, which
    processes escapes left-to-right in a single pass.

    Args:
        text: The raw string potentially containing backslash sequences.

    Returns:
        A new string with escape sequences replaced by their values.
    """
    result: list[str] = []
    i = 0
    length = len(text)

    while i < length:
        char = text[i]

        if char != "\\":
            # --- Normal character: just append it. ---
            result.append(char)
            i += 1
            continue

        # --- We found a backslash. Look at the next character. ---
        if i + 1 >= length:
            # Backslash at end of string — output it literally.
            result.append("\\")
            i += 1
            continue

        next_char = text[i + 1]

        if next_char in ESCAPE_MAP:
            # --- Simple escape: look up the replacement. ---
            result.append(ESCAPE_MAP[next_char])
            i += 2

        elif next_char == "0":
            # --- Octal escape: \0 followed by up to 3 octal digits. ---
            # Examples:
            #   \0    -> NUL (octal 0)
            #   \012  -> newline (octal 12 = decimal 10)
            #   \0101 -> 'A' (octal 101 = decimal 65)
            octal_digits = ""
            j = i + 2  # Start after \0
            while j < length and len(octal_digits) < 3 and text[j] in "01234567":
                octal_digits += text[j]
                j += 1
            # Convert octal string to integer, then to character.
            # If no digits follow \0, the value is 0 (NUL character).
            octal_value = int(octal_digits, 8) if octal_digits else 0
            result.append(chr(octal_value))
            i = j

        else:
            # --- Unknown escape: output backslash + character as-is. ---
            result.append("\\")
            result.append(next_char)
            i += 2

    return "".join(result)


def main() -> None:
    """Entry point: parse args via CLI Builder, then echo the output."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    # Hand the spec file and sys.argv to CLI Builder. The parser reads the
    # JSON spec, validates the flags, enforces mutual exclusivity of -e/-E,
    # and returns one of three result types.
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"echo: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    # CLI Builder returns one of:
    #   - HelpResult:    user passed --help
    #   - VersionResult: user passed --version
    #   - ParseResult:   normal invocation; flags and arguments are populated

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    # Join all positional arguments with spaces. Then optionally process
    # escape sequences and optionally suppress the trailing newline.

    assert isinstance(result, ParseResult)

    # Get the positional arguments (the strings to echo).
    # If no arguments were provided, we output an empty string.
    strings = result.arguments.get("strings", [])
    if isinstance(strings, list):
        output = " ".join(strings)
    else:
        output = str(strings) if strings else ""

    # Check whether escape processing is enabled.
    # -e enables escapes, -E disables them (default).
    enable_escapes = result.flags.get("enable_escapes", False)

    if enable_escapes:
        output = process_escapes(output)

    # Check whether the trailing newline should be suppressed.
    no_newline = result.flags.get("no_newline", False)

    # Write the output. The ``end`` parameter controls whether a
    # newline is appended.
    end = "" if no_newline else "\n"
    print(output, end=end)


if __name__ == "__main__":
    main()
