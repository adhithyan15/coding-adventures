"""Tests for the seq tool.

=== What These Tests Verify ===

These tests exercise the seq implementation, including:

1. Spec loading and basic parsing
2. One-argument, two-argument, and three-argument forms
3. The -s flag (custom separator)
4. The -w flag (equal width)
5. CLI Builder integration (--help, --version)
6. Business logic functions (generate_sequence, parse_number, format_number)
"""

from __future__ import annotations

import sys
from decimal import Decimal
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "seq.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the seq spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the seq.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["seq", "5"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Test: Argument forms
# ---------------------------------------------------------------------------


class TestArgumentForms:
    """seq supports 1, 2, or 3 positional arguments."""

    def test_one_arg(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["seq", "5"])
        assert isinstance(result, ParseResult)
        nums = result.arguments.get("numbers", [])
        if isinstance(nums, str):
            assert nums == "5"
        else:
            assert nums == ["5"]

    def test_two_args(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["seq", "3", "7"])
        assert isinstance(result, ParseResult)
        nums = result.arguments.get("numbers", [])
        assert len(nums) == 2

    def test_three_args(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["seq", "1", "2", "10"])
        assert isinstance(result, ParseResult)
        nums = result.arguments.get("numbers", [])
        assert len(nums) == 3


# ---------------------------------------------------------------------------
# Test: Flags
# ---------------------------------------------------------------------------


class TestSeparatorFlag:
    """The ``-s`` flag changes the separator."""

    def test_s_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["seq", "-s", ", ", "3"])
        assert isinstance(result, ParseResult)
        assert result.flags["separator"] == ", "


class TestEqualWidthFlag:
    """The ``-w`` flag enables zero-padded equal width."""

    def test_w_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["seq", "-w", "100"])
        assert isinstance(result, ParseResult)
        assert result.flags["equal_width"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["seq", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["seq", "--help"])
        assert isinstance(result, HelpResult)
        assert "seq" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["seq", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["seq", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — generate_sequence
# ---------------------------------------------------------------------------


class TestGenerateSequence:
    """Test the generate_sequence function directly."""

    def test_simple_ascending(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import generate_sequence

        result = generate_sequence(Decimal("1"), Decimal("1"), Decimal("5"))
        assert result == [Decimal(str(i)) for i in range(1, 6)]

    def test_with_increment(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import generate_sequence

        result = generate_sequence(Decimal("1"), Decimal("2"), Decimal("9"))
        assert result == [Decimal("1"), Decimal("3"), Decimal("5"), Decimal("7"), Decimal("9")]

    def test_descending(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import generate_sequence

        result = generate_sequence(Decimal("5"), Decimal("-1"), Decimal("1"))
        assert result == [Decimal("5"), Decimal("4"), Decimal("3"), Decimal("2"), Decimal("1")]

    def test_single_number(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import generate_sequence

        result = generate_sequence(Decimal("3"), Decimal("1"), Decimal("3"))
        assert result == [Decimal("3")]

    def test_empty_sequence(self) -> None:
        """When first > last with positive increment, result is empty."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import generate_sequence

        result = generate_sequence(Decimal("5"), Decimal("1"), Decimal("3"))
        assert result == []

    def test_zero_increment(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import generate_sequence

        result = generate_sequence(Decimal("1"), Decimal("0"), Decimal("5"))
        assert result == []


class TestParseNumber:
    """Test the parse_number function."""

    def test_integer(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import parse_number

        assert parse_number("42") == Decimal("42")

    def test_float(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import parse_number

        assert parse_number("3.14") == Decimal("3.14")

    def test_negative(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import parse_number

        assert parse_number("-5") == Decimal("-5")


class TestFormatNumber:
    """Test the format_number function."""

    def test_integer_format(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import format_number

        result = format_number(Decimal("5"), precision=0, width=0, equal_width=False, fmt=None)
        assert result == "5"

    def test_equal_width_padding(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from seq_tool import format_number

        result = format_number(Decimal("5"), precision=0, width=3, equal_width=True, fmt=None)
        assert result == "005"
