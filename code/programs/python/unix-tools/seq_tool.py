"""seq — print a sequence of numbers.

=== What This Program Does ===

This is a reimplementation of the GNU ``seq`` utility. It generates a
sequence of numbers from FIRST to LAST with an optional INCREMENT, printing
each number on its own line (or separated by a custom string).

=== Invocation Forms ===

seq supports three argument patterns:

1. ``seq LAST``           — count from 1 to LAST by 1
2. ``seq FIRST LAST``     — count from FIRST to LAST by 1
3. ``seq FIRST INCR LAST`` — count from FIRST to LAST by INCR

For example::

    $ seq 5          # prints 1 2 3 4 5
    $ seq 2 5        # prints 2 3 4 5
    $ seq 1 2 10     # prints 1 3 5 7 9

=== Floating Point Support ===

seq handles both integers and floating-point numbers. If any argument
contains a decimal point, the output uses floating-point formatting::

    $ seq 0.5 0.5 2.5
    0.5
    1.0
    1.5
    2.0
    2.5

=== Equal Width (-w) ===

The ``-w`` flag pads numbers with leading zeros so they all have the
same width. This is useful for generating filenames::

    $ seq -w 1 100
    001
    002
    ...
    100

=== Custom Format (-f) ===

The ``-f`` flag accepts a printf-style format string::

    $ seq -f "file_%03g" 1 3
    file_001
    file_002
    file_003

=== Custom Separator (-s) ===

The ``-s`` flag changes the separator between numbers (default is newline)::

    $ seq -s ", " 1 5
    1, 2, 3, 4, 5

=== CLI Builder Integration ===

The entire CLI is defined in ``seq.json``. CLI Builder handles flag parsing,
help text, and version output. This file implements the sequence generation
and formatting logic.
"""

from __future__ import annotations

import math
import sys
from decimal import Decimal, InvalidOperation
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "seq.json")


def parse_number(value: str) -> Decimal:
    """Parse a string into a Decimal number.

    We use Decimal rather than float to avoid floating-point precision
    issues. For example, with floats, ``0.1 + 0.1 + 0.1`` might not
    equal ``0.3`` exactly. Decimal arithmetic preserves the precision
    that the user specified.

    Args:
        value: The string representation of the number.

    Returns:
        A Decimal value.

    Raises:
        SystemExit: If the value is not a valid number.
    """
    try:
        return Decimal(value)
    except InvalidOperation:
        print(f"seq: invalid floating point argument: '{value}'", file=sys.stderr)
        raise SystemExit(1) from None


def decimal_places(value: Decimal) -> int:
    """Count the number of decimal places in a Decimal value.

    This is used to determine output precision. If the user writes
    ``seq 0.5 0.5 2.0``, we want output like ``0.5``, ``1.0``, etc.

    The number of decimal places is determined by the exponent:
    - ``Decimal('1')`` has exponent 0 -> 0 decimal places
    - ``Decimal('1.5')`` has exponent -1 -> 1 decimal place
    - ``Decimal('1.50')`` has exponent -2 -> 2 decimal places

    Args:
        value: The Decimal to examine.

    Returns:
        The number of decimal places (0 if integer).
    """
    # The sign, digits, and exponent of the Decimal.
    _, _, exp = value.as_tuple()
    if isinstance(exp, str):
        # Special values like Infinity or NaN.
        return 0
    return max(0, -exp)


def format_number(
    value: Decimal,
    *,
    precision: int,
    width: int,
    equal_width: bool,
    fmt: str | None,
) -> str:
    """Format a number for output.

    This function handles three formatting modes:

    1. **Custom format** (``-f``): Use a printf-style format string.
       The format uses ``%g``, ``%f``, or ``%e`` specifiers.

    2. **Equal width** (``-w``): Pad with leading zeros to the specified
       width. The width is determined by the widest number in the sequence.

    3. **Default**: Use the natural precision of the input numbers.

    Args:
        value: The number to format.
        precision: Number of decimal places for default formatting.
        width: Minimum width for equal-width mode.
        equal_width: Whether to pad with leading zeros.
        fmt: Optional printf-style format string.

    Returns:
        The formatted number string.
    """
    if fmt is not None:
        # Convert printf-style format (%g, %f, %e) to Python format.
        # We replace %g/%f/%e with Python's equivalent.
        return fmt % float(value)

    if precision == 0:
        num_str = str(int(value))
    else:
        num_str = f"{float(value):.{precision}f}"

    if equal_width:
        # Pad with leading zeros. Handle negative numbers correctly:
        # the minus sign should come before the zeros.
        if value < 0:
            return "-" + num_str[1:].zfill(width - 1)
        return num_str.zfill(width)

    return num_str


def generate_sequence(
    first: Decimal,
    increment: Decimal,
    last: Decimal,
) -> list[Decimal]:
    """Generate a sequence of numbers from first to last by increment.

    The sequence includes ``first`` and includes ``last`` if the sequence
    reaches it exactly. The direction is determined by the sign of
    increment:

    - Positive increment: sequence goes up (first must be <= last)
    - Negative increment: sequence goes down (first must be >= last)
    - Zero increment: infinite loop (we cap at a reasonable limit)

    Args:
        first: The starting number.
        increment: The step between numbers.
        last: The ending number (inclusive if reached exactly).

    Returns:
        A list of Decimal values in the sequence.
    """
    result: list[Decimal] = []

    if increment == 0:
        # GNU seq with increment 0 runs forever. We don't do that.
        return result

    if increment > 0:
        current = first
        while current <= last:
            result.append(current)
            current += increment
    else:
        current = first
        while current >= last:
            result.append(current)
            current += increment

    return result


def main() -> None:
    """Entry point: parse args via CLI Builder, then generate sequence."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"seq: {error.message}", file=sys.stderr)
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

    separator = result.flags.get("separator", "\n")
    equal_width = result.flags.get("equal_width", False)
    fmt = result.flags.get("format")

    # Parse the positional arguments: LAST, or FIRST LAST, or FIRST INCR LAST.
    numbers = result.arguments.get("numbers", [])
    if isinstance(numbers, str):
        numbers = [numbers]

    if len(numbers) == 1:
        first = Decimal("1")
        increment = Decimal("1")
        last = parse_number(numbers[0])
    elif len(numbers) == 2:  # noqa: PLR2004
        first = parse_number(numbers[0])
        increment = Decimal("1")
        last = parse_number(numbers[1])
    elif len(numbers) == 3:  # noqa: PLR2004
        first = parse_number(numbers[0])
        increment = parse_number(numbers[1])
        last = parse_number(numbers[2])
    else:
        print("seq: missing operand", file=sys.stderr)
        raise SystemExit(1)

    # Determine output precision from the input values.
    precision = max(
        decimal_places(first),
        decimal_places(increment),
        decimal_places(last),
    )

    # Generate the sequence.
    sequence = generate_sequence(first, increment, last)

    if not sequence:
        return

    # Determine width for equal-width mode.
    width = 0
    if equal_width:
        width = max(len(format_number(n, precision=precision, width=0, equal_width=False, fmt=None)) for n in sequence)

    # Format and output.
    formatted = [
        format_number(n, precision=precision, width=width, equal_width=equal_width, fmt=fmt)
        for n in sequence
    ]

    print(separator.join(formatted))


if __name__ == "__main__":
    main()
