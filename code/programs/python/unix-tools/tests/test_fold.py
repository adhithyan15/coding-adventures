"""Tests for the fold tool.

=== What These Tests Verify ===

These tests exercise the fold implementation, including:

1. Spec loading and CLI Builder integration
2. Default width (80)
3. Custom width (-w)
4. Break at spaces (-s)
5. Count bytes (-b)
6. Business logic function (fold_line)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "fold.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from fold_tool import fold_line


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["fold"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_width_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["fold", "-w", "40"])
        assert isinstance(result, ParseResult)
        assert result.flags["width"] == 40

    def test_spaces_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["fold", "-s"])
        assert isinstance(result, ParseResult)
        assert result.flags["spaces"] is True

    def test_bytes_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["fold", "-b"])
        assert isinstance(result, ParseResult)
        assert result.flags["bytes"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["fold", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["fold", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFoldLine:
    def test_short_line_unchanged(self) -> None:
        result = fold_line("hello", 80, break_at_spaces=False, count_bytes=False)
        assert result == "hello"

    def test_exact_width(self) -> None:
        result = fold_line("a" * 10, 10, break_at_spaces=False, count_bytes=False)
        assert result == "a" * 10

    def test_long_line_wrapped(self) -> None:
        result = fold_line("a" * 20, 10, break_at_spaces=False, count_bytes=False)
        assert "\n" in result
        lines = result.split("\n")
        assert len(lines[0]) == 10

    def test_break_at_spaces(self) -> None:
        result = fold_line("hello world foo", 12, break_at_spaces=True,
                           count_bytes=False)
        assert "\n" in result
        # Should break at a space, not in the middle of a word.
        parts = result.split("\n")
        for part in parts:
            assert len(part) <= 12

    def test_empty_string(self) -> None:
        result = fold_line("", 80, break_at_spaces=False, count_bytes=False)
        assert result == ""

    def test_width_1(self) -> None:
        result = fold_line("abc", 1, break_at_spaces=False, count_bytes=False)
        assert result == "a\nb\nc"

    def test_no_break_needed(self) -> None:
        result = fold_line("short", 100, break_at_spaces=False, count_bytes=False)
        assert result == "short"
