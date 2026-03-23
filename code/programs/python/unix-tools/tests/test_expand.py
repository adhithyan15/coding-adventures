"""Tests for the expand tool.

=== What These Tests Verify ===

These tests exercise the expand implementation, including:

1. Spec loading and CLI Builder integration
2. Default tab expansion (8 spaces)
3. Custom tab width (-t N)
4. Explicit tab stop list (-t N1,N2,...)
5. Initial-only mode (-i)
6. Business logic functions
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "expand.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from expand_tool import expand_line, parse_tab_stops, spaces_to_next_stop


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["expand"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_tabs_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["expand", "-t", "4"])
        assert isinstance(result, ParseResult)
        assert result.flags["tabs"] == "4"

    def test_initial_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["expand", "-i"])
        assert isinstance(result, ParseResult)
        assert result.flags["initial"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["expand", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["expand", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestParseTabStops:
    def test_default(self) -> None:
        assert parse_tab_stops(None) == 8

    def test_single_number(self) -> None:
        assert parse_tab_stops("4") == 4

    def test_comma_list(self) -> None:
        result = parse_tab_stops("4,8,12")
        assert result == [4, 8, 12]


class TestSpacesToNextStop:
    def test_uniform_at_zero(self) -> None:
        assert spaces_to_next_stop(0, 8) == 8

    def test_uniform_at_three(self) -> None:
        assert spaces_to_next_stop(3, 8) == 5

    def test_uniform_at_eight(self) -> None:
        assert spaces_to_next_stop(8, 8) == 8

    def test_explicit_stops(self) -> None:
        stops = [4, 8, 12]
        assert spaces_to_next_stop(0, stops) == 4
        assert spaces_to_next_stop(4, stops) == 4
        assert spaces_to_next_stop(6, stops) == 2

    def test_past_explicit_stops(self) -> None:
        stops = [4, 8]
        assert spaces_to_next_stop(10, stops) == 1


class TestExpandLine:
    def test_no_tabs(self) -> None:
        assert expand_line("hello\n", 8, initial_only=False) == "hello\n"

    def test_single_tab_at_start(self) -> None:
        result = expand_line("\thello\n", 8, initial_only=False)
        assert result == "        hello\n"

    def test_tab_at_column_3(self) -> None:
        result = expand_line("abc\tdef\n", 8, initial_only=False)
        assert result == "abc     def\n"

    def test_custom_tab_width(self) -> None:
        result = expand_line("\thello\n", 4, initial_only=False)
        assert result == "    hello\n"

    def test_initial_only_expands_leading(self) -> None:
        result = expand_line("\thello\tworld\n", 8, initial_only=True)
        assert result.startswith("        hello")
        assert "\t" in result  # Second tab should be preserved.

    def test_empty_line(self) -> None:
        assert expand_line("\n", 8, initial_only=False) == "\n"

    def test_multiple_tabs(self) -> None:
        result = expand_line("\t\thello\n", 4, initial_only=False)
        assert result == "        hello\n"
