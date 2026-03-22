"""Tests for the tail tool.

=== What These Tests Verify ===

These tests exercise the tail implementation, including:

1. Spec loading and default behavior
2. The -n flag with regular and +N syntax
3. The -c flag (byte mode)
4. Header display logic
5. CLI Builder integration (--help, --version)
6. Business logic functions (tail_lines, tail_bytes, parse_count)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "tail.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the tail spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading and default behavior
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the tail.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tail"])
        assert isinstance(result, ParseResult)

    def test_default_lines_is_10(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tail"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("lines", "10") == "10"


# ---------------------------------------------------------------------------
# Test: -n flag
# ---------------------------------------------------------------------------


class TestLinesFlag:
    """The ``-n`` flag controls how many lines to output."""

    def test_short_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tail", "-n", "5"])
        assert isinstance(result, ParseResult)
        assert result.flags["lines"] == "5"

    def test_plus_syntax(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tail", "-n", "+5"])
        assert isinstance(result, ParseResult)
        assert result.flags["lines"] == "+5"


# ---------------------------------------------------------------------------
# Test: -f flag (follow)
# ---------------------------------------------------------------------------


class TestFollowFlag:
    """The ``-f`` flag enables follow mode."""

    def test_follow_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tail", "-f"])
        assert isinstance(result, ParseResult)
        assert result.flags["follow"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tail", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tail", "--help"])
        assert isinstance(result, HelpResult)
        assert "tail" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tail", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tail", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — parse_count
# ---------------------------------------------------------------------------


class TestParseCount:
    """Test the parse_count function that handles the +N syntax."""

    def test_plain_number(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import parse_count

        count, from_start = parse_count("10")
        assert count == 10
        assert from_start is False

    def test_plus_prefix(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import parse_count

        count, from_start = parse_count("+5")
        assert count == 5
        assert from_start is True

    def test_minus_prefix(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import parse_count

        count, from_start = parse_count("-3")
        assert count == 3
        assert from_start is False


# ---------------------------------------------------------------------------
# Test: Business logic — tail_lines
# ---------------------------------------------------------------------------


class TestTailLines:
    """Test the tail_lines function directly."""

    def test_last_3_lines(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_lines

        lines = [f"line {i}\n" for i in range(1, 11)]
        result = tail_lines(lines, 3, from_start=False)
        assert result == "line 8\nline 9\nline 10\n"

    def test_from_start_plus_3(self) -> None:
        """tail -n +3 should output from line 3 onwards."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_lines

        lines = ["one\n", "two\n", "three\n", "four\n", "five\n"]
        result = tail_lines(lines, 3, from_start=True)
        assert result == "three\nfour\nfive\n"

    def test_from_start_plus_1_is_entire_file(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_lines

        lines = ["one\n", "two\n"]
        result = tail_lines(lines, 1, from_start=True)
        assert result == "one\ntwo\n"

    def test_last_zero_lines(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_lines

        lines = ["one\n", "two\n"]
        result = tail_lines(lines, 0, from_start=False)
        assert result == ""

    def test_more_lines_than_available(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_lines

        lines = ["one\n"]
        result = tail_lines(lines, 100, from_start=False)
        assert result == "one\n"


class TestTailBytes:
    """Test the tail_bytes function directly."""

    def test_last_5_bytes(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_bytes

        data = b"Hello, World!"
        result = tail_bytes(data, 5, from_start=False)
        assert result == b"orld!"

    def test_from_start(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tail_tool import tail_bytes

        data = b"Hello, World!"
        result = tail_bytes(data, 8, from_start=True)
        assert result == b"World!"
