"""Tests for the nl tool.

=== What These Tests Verify ===

These tests exercise the nl implementation, including:

1. Spec loading and CLI Builder integration
2. Default body numbering (non-empty lines)
3. Number all lines (-b a)
4. Number format (ln, rn, rz)
5. Custom width and separator
6. Section detection
7. Business logic functions
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "nl.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from nl_tool import (
    detect_section,
    format_number,
    number_lines,
    should_number_line,
)


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nl"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_body_numbering_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nl", "-b", "a"])
        assert isinstance(result, ParseResult)
        assert result.flags["body_numbering"] == "a"

    def test_number_format_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nl", "-n", "rz"])
        assert isinstance(result, ParseResult)
        assert result.flags["number_format"] == "rz"

    def test_width_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nl", "-w", "3"])
        assert isinstance(result, ParseResult)
        assert result.flags["number_width"] == 3

    def test_increment_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nl", "-i", "2"])
        assert isinstance(result, ParseResult)
        assert result.flags["line_increment"] == 2


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["nl", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["nl", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestShouldNumberLine:
    def test_all_style(self) -> None:
        assert should_number_line("hello", "a") is True
        assert should_number_line("", "a") is True

    def test_non_empty_style(self) -> None:
        assert should_number_line("hello", "t") is True
        assert should_number_line("", "t") is False
        assert should_number_line("   ", "t") is False

    def test_none_style(self) -> None:
        assert should_number_line("hello", "n") is False

    def test_regex_style(self) -> None:
        assert should_number_line("ERROR: something", "pERROR") is True
        assert should_number_line("info: ok", "pERROR") is False


class TestFormatNumber:
    def test_right_justified(self) -> None:
        assert format_number(1, "rn", 6) == "     1"

    def test_left_justified(self) -> None:
        assert format_number(1, "ln", 6) == "1     "

    def test_right_zero(self) -> None:
        assert format_number(1, "rz", 6) == "000001"

    def test_large_number(self) -> None:
        assert format_number(999, "rn", 6) == "   999"


class TestDetectSection:
    def test_header(self) -> None:
        assert detect_section("\\:\\:\\:", "\\:") == "header"

    def test_body(self) -> None:
        assert detect_section("\\:\\:", "\\:") == "body"

    def test_footer(self) -> None:
        assert detect_section("\\:", "\\:") == "footer"

    def test_not_section(self) -> None:
        assert detect_section("hello", "\\:") is None


class TestNumberLines:
    def test_default_numbering(self) -> None:
        lines = ["hello", "world", ""]
        result = number_lines(
            lines, body_style="t", header_style="n", footer_style="n",
            start_number=1, increment=1, number_format="rn",
            number_width=6, separator="\t", section_delimiter="\\:",
        )
        # "hello" and "world" should be numbered, blank line should not.
        assert "1" in result[0]
        assert "2" in result[1]

    def test_all_lines_numbered(self) -> None:
        lines = ["hello", "", "world"]
        result = number_lines(
            lines, body_style="a", header_style="n", footer_style="n",
            start_number=1, increment=1, number_format="rn",
            number_width=6, separator="\t", section_delimiter="\\:",
        )
        assert "1" in result[0]
        assert "2" in result[1]
        assert "3" in result[2]

    def test_custom_increment(self) -> None:
        lines = ["a", "b", "c"]
        result = number_lines(
            lines, body_style="a", header_style="n", footer_style="n",
            start_number=1, increment=5, number_format="rn",
            number_width=6, separator="\t", section_delimiter="\\:",
        )
        assert "1" in result[0]
        assert "6" in result[1]
        assert "11" in result[2]

    def test_custom_start(self) -> None:
        lines = ["a", "b"]
        result = number_lines(
            lines, body_style="a", header_style="n", footer_style="n",
            start_number=10, increment=1, number_format="rn",
            number_width=6, separator="\t", section_delimiter="\\:",
        )
        assert "10" in result[0]
        assert "11" in result[1]

    def test_empty_input(self) -> None:
        result = number_lines(
            [], body_style="t", header_style="n", footer_style="n",
            start_number=1, increment=1, number_format="rn",
            number_width=6, separator="\t", section_delimiter="\\:",
        )
        assert result == []
