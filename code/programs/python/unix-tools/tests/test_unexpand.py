"""Tests for the unexpand tool.

=== What These Tests Verify ===

These tests exercise the unexpand implementation, including:

1. Spec loading and CLI Builder integration
2. Default behavior (initial blanks only)
3. The -a flag (all blanks)
4. Tab stop configuration
5. Business logic functions
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "unexpand.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from unexpand_tool import is_tab_stop, parse_tab_stops, unexpand_line


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["unexpand"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_all_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["unexpand", "-a"])
        assert isinstance(result, ParseResult)
        assert result.flags["all"] is True

    def test_tabs_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["unexpand", "-t", "4"])
        assert isinstance(result, ParseResult)
        assert result.flags["tabs"] == "4"


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["unexpand", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["unexpand", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestIsTabStop:
    def test_uniform_at_zero(self) -> None:
        assert is_tab_stop(0, 8) is True

    def test_uniform_at_eight(self) -> None:
        assert is_tab_stop(8, 8) is True

    def test_uniform_at_three(self) -> None:
        assert is_tab_stop(3, 8) is False

    def test_explicit_stop(self) -> None:
        assert is_tab_stop(4, [4, 8, 12]) is True

    def test_explicit_non_stop(self) -> None:
        assert is_tab_stop(5, [4, 8, 12]) is False


class TestUnexpandLine:
    def test_no_spaces(self) -> None:
        assert unexpand_line("hello\n", 8, convert_all=False) == "hello\n"

    def test_leading_spaces_converted(self) -> None:
        # 8 spaces at the start should become a tab.
        result = unexpand_line("        hello\n", 8, convert_all=False)
        assert result == "\thello\n"

    def test_fewer_spaces_not_converted(self) -> None:
        # 3 spaces don't reach a tab stop.
        result = unexpand_line("   hello\n", 8, convert_all=False)
        assert result == "   hello\n"

    def test_all_mode(self) -> None:
        # With -a, spaces after content are also converted.
        result = unexpand_line("hello        world\n", 8, convert_all=True)
        assert "\t" in result

    def test_empty_line(self) -> None:
        assert unexpand_line("\n", 8, convert_all=False) == "\n"

    def test_custom_tab_width(self) -> None:
        result = unexpand_line("    hello\n", 4, convert_all=False)
        assert result == "\thello\n"
