"""sleep — delay for a specified amount of time.

=== What This Program Does ===

This is a reimplementation of the GNU ``sleep`` utility. It pauses
execution for the specified duration(s). Unlike the POSIX version (which
only accepts whole seconds), this implementation supports fractional
values and time suffixes.

=== Duration Format ===

Each duration argument is a number optionally followed by a suffix:

    s    seconds (the default if no suffix is given)
    m    minutes (multiply by 60)
    h    hours   (multiply by 3600)
    d    days    (multiply by 86400)

Examples::

    sleep 5        # sleep 5 seconds
    sleep 0.5      # sleep half a second
    sleep 2m       # sleep 2 minutes (120 seconds)
    sleep 1h30m    # ERROR — each duration is a separate argument
    sleep 1h 30m   # sleep 1 hour and 30 minutes (durations are summed)

=== Multiple Durations ===

When multiple duration arguments are given, they are summed::

    sleep 1 2 3     # sleep 6 seconds (1 + 2 + 3)
    sleep 1m 30s    # sleep 90 seconds (60 + 30)
    sleep 1d 12h    # sleep 36 hours (24 + 12) * 3600

This is useful for expressing complex durations in a readable way.

=== CLI Builder Integration ===

The JSON spec ``sleep.json`` defines a single variadic required argument
called ``duration``. CLI Builder handles parsing the argument list, and
we implement the duration-string parsing and the actual sleep logic.
"""

from __future__ import annotations

import re
import sys
import time
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "sleep.json")


# ---------------------------------------------------------------------------
# Duration parsing
# ---------------------------------------------------------------------------

# Multipliers for each suffix. The key is the suffix character, the value
# is how many seconds one unit of that suffix represents.
SUFFIX_MULTIPLIERS: dict[str, float] = {
    "s": 1.0,       # seconds
    "m": 60.0,      # minutes
    "h": 3600.0,    # hours
    "d": 86400.0,   # days
}

# Regex pattern that matches a number (integer or float) optionally
# followed by a suffix letter. The number can be:
#   - "5"     (integer)
#   - "0.5"   (decimal)
#   - ".5"    (leading dot, like ".5s")
#
# The suffix is optional and must be one of s, m, h, d.
DURATION_PATTERN = re.compile(r"^(\d+\.?\d*|\.\d+)([smhd])?$")


def parse_duration(duration_str: str) -> float:
    """Parse a single duration string into seconds.

    A duration string is a number optionally followed by a suffix:
    ``s`` (seconds), ``m`` (minutes), ``h`` (hours), ``d`` (days).
    If no suffix is given, seconds is assumed.

    Args:
        duration_str: The duration string to parse (e.g., "5", "2.5m",
                      "1h", ".5s").

    Returns:
        The duration in seconds as a float.

    Raises:
        ValueError: If the string cannot be parsed as a valid duration.

    Examples:
        >>> parse_duration("5")
        5.0
        >>> parse_duration("2.5m")
        150.0
        >>> parse_duration("1h")
        3600.0
        >>> parse_duration("0.5s")
        0.5
        >>> parse_duration("1d")
        86400.0
        >>> parse_duration(".5")
        0.5
    """
    match = DURATION_PATTERN.match(duration_str)
    if match is None:
        raise ValueError(f"invalid time interval '{duration_str}'")

    number_str, suffix = match.groups()
    number = float(number_str)

    if number < 0:
        raise ValueError(f"invalid time interval '{duration_str}'")

    # If no suffix was given, default to seconds.
    multiplier = SUFFIX_MULTIPLIERS.get(suffix or "s", 1.0)
    return number * multiplier


def parse_durations(duration_strings: list[str]) -> float:
    """Parse multiple duration strings and return the total in seconds.

    Each string is parsed individually, and the results are summed.

    Args:
        duration_strings: A list of duration strings (e.g., ["1m", "30s"]).

    Returns:
        The total duration in seconds.

    Raises:
        ValueError: If any string cannot be parsed.

    Examples:
        >>> parse_durations(["1m", "30s"])
        90.0
        >>> parse_durations(["1", "2", "3"])
        6.0
        >>> parse_durations(["1h", "30m"])
        5400.0
    """
    return sum(parse_duration(s) for s in duration_strings)


def main() -> None:
    """Entry point: parse args via CLI Builder, then sleep."""
    # Import CLI Builder. This import will fail if the package is not
    # installed — see the BUILD file for how dependencies are set up.
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"sleep: {error.message}", file=sys.stderr)
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

    # Get the duration argument(s). CLI Builder returns a list because
    # the argument is variadic.
    durations = result.arguments.get("duration", [])
    if isinstance(durations, str):
        durations = [durations]

    try:
        total_seconds = parse_durations(durations)
    except ValueError as exc:
        print(f"sleep: {exc}", file=sys.stderr)
        raise SystemExit(1) from None

    time.sleep(total_seconds)


if __name__ == "__main__":
    main()
