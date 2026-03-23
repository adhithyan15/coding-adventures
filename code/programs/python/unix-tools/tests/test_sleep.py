"""Tests for the sleep tool.

=== What These Tests Verify ===

These tests exercise:

1. The ``parse_duration`` function — parsing individual duration strings
2. The ``parse_durations`` function — summing multiple durations
3. Edge cases — invalid input, fractional values, all suffixes
4. CLI Builder integration — spec loading, --help, --version
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "sleep.json")

# Add the tool directory to sys.path for imports.
sys.path.insert(0, str(Path(__file__).parent.parent))


# ---------------------------------------------------------------------------
# Helper: parse argv through CLI Builder
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the sleep spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: parse_duration function — basic cases
# ---------------------------------------------------------------------------


class TestParseDurationBasic:
    """Test parsing individual duration strings."""

    def test_plain_integer(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("5") == 5.0

    def test_plain_float(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("2.5") == 2.5

    def test_leading_dot(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration(".5") == 0.5

    def test_zero(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("0") == 0.0

    def test_large_number(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("1000") == 1000.0


# ---------------------------------------------------------------------------
# Test: parse_duration function — suffixes
# ---------------------------------------------------------------------------


class TestParseDurationSuffixes:
    """Test that each suffix produces the correct multiplier."""

    def test_seconds_suffix(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("5s") == 5.0

    def test_minutes_suffix(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("2m") == 120.0

    def test_hours_suffix(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("1h") == 3600.0

    def test_days_suffix(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("1d") == 86400.0

    def test_fractional_minutes(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("0.5m") == 30.0

    def test_fractional_hours(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("1.5h") == 5400.0

    def test_fractional_days(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("0.5d") == 43200.0

    def test_zero_with_suffix(self) -> None:
        from sleep_tool import parse_duration

        assert parse_duration("0s") == 0.0
        assert parse_duration("0m") == 0.0
        assert parse_duration("0h") == 0.0
        assert parse_duration("0d") == 0.0


# ---------------------------------------------------------------------------
# Test: parse_duration function — error cases
# ---------------------------------------------------------------------------


class TestParseDurationErrors:
    """Test that invalid duration strings raise ValueError."""

    def test_empty_string(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("")

    def test_only_suffix(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("s")

    def test_unknown_suffix(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("5x")

    def test_letters_only(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("abc")

    def test_multiple_dots(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("1.2.3")

    def test_negative_number(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("-5")

    def test_spaces(self) -> None:
        from sleep_tool import parse_duration

        with pytest.raises(ValueError, match="invalid time interval"):
            parse_duration("5 s")


# ---------------------------------------------------------------------------
# Test: parse_durations function
# ---------------------------------------------------------------------------


class TestParseDurations:
    """Test the parse_durations function — summing multiple durations."""

    def test_single_duration(self) -> None:
        from sleep_tool import parse_durations

        assert parse_durations(["5"]) == 5.0

    def test_multiple_seconds(self) -> None:
        from sleep_tool import parse_durations

        assert parse_durations(["1", "2", "3"]) == 6.0

    def test_mixed_suffixes(self) -> None:
        from sleep_tool import parse_durations

        assert parse_durations(["1m", "30s"]) == 90.0

    def test_hour_and_minutes(self) -> None:
        from sleep_tool import parse_durations

        assert parse_durations(["1h", "30m"]) == 5400.0

    def test_empty_list(self) -> None:
        from sleep_tool import parse_durations

        assert parse_durations([]) == 0.0

    def test_invalid_in_list_raises(self) -> None:
        from sleep_tool import parse_durations

        with pytest.raises(ValueError):
            parse_durations(["1", "abc", "3"])

    def test_all_suffixes(self) -> None:
        from sleep_tool import parse_durations

        # 1d + 1h + 1m + 1s = 86400 + 3600 + 60 + 1 = 90061
        assert parse_durations(["1d", "1h", "1m", "1s"]) == 90061.0


# ---------------------------------------------------------------------------
# Test: CLI Builder integration
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["sleep", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["sleep", "--help"])
        assert isinstance(result, HelpResult)
        assert "sleep" in result.text

    def test_help_text_contains_description(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["sleep", "--help"])
        assert isinstance(result, HelpResult)
        assert "delay" in result.text.lower() or "time" in result.text.lower()


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["sleep", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["sleep", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestDurationArgument:
    """CLI Builder should parse the duration argument correctly."""

    def test_single_duration_arg(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sleep", "5"])
        assert isinstance(result, ParseResult)
        duration = result.arguments.get("duration")
        assert duration is not None

    def test_multiple_duration_args(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sleep", "1", "2", "3"])
        assert isinstance(result, ParseResult)
        duration = result.arguments.get("duration")
        assert duration is not None

    def test_missing_duration_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["sleep"])
