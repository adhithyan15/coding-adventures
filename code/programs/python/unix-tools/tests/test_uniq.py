"""Tests for the uniq tool.

=== What These Tests Verify ===

These tests exercise the uniq implementation, including:

1. Spec loading and CLI Builder integration
2. Default behavior (adjacent deduplication)
3. The -c flag (count)
4. The -d flag (duplicated only)
5. The -u flag (unique only)
6. The -i flag (ignore case)
7. Field and character skipping (-f, -s, -w)
8. Business logic functions
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "uniq.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from uniq_tool import get_comparison_key, uniq_lines


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uniq"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_count_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uniq", "-c"])
        assert isinstance(result, ParseResult)
        assert result.flags["count"] is True

    def test_repeated_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uniq", "-d"])
        assert isinstance(result, ParseResult)
        assert result.flags["repeated"] is True

    def test_unique_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uniq", "-u"])
        assert isinstance(result, ParseResult)
        assert result.flags["unique"] is True

    def test_ignore_case_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uniq", "-i"])
        assert isinstance(result, ParseResult)
        assert result.flags["ignore_case"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["uniq", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["uniq", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestGetComparisonKey:
    def test_basic(self) -> None:
        key = get_comparison_key("hello", skip_fields=0, skip_chars=0,
                                 check_chars=None, ignore_case=False)
        assert key == "hello"

    def test_ignore_case(self) -> None:
        key = get_comparison_key("Hello", skip_fields=0, skip_chars=0,
                                 check_chars=None, ignore_case=True)
        assert key == "hello"

    def test_skip_fields(self) -> None:
        key = get_comparison_key("field1 field2 field3", skip_fields=1,
                                 skip_chars=0, check_chars=None, ignore_case=False)
        assert "field2" in key

    def test_skip_chars(self) -> None:
        key = get_comparison_key("hello", skip_fields=0, skip_chars=2,
                                 check_chars=None, ignore_case=False)
        assert key == "llo"

    def test_check_chars(self) -> None:
        key = get_comparison_key("hello", skip_fields=0, skip_chars=0,
                                 check_chars=3, ignore_case=False)
        assert key == "hel"


class TestUniqLines:
    def test_no_duplicates(self) -> None:
        lines = ["a", "b", "c"]
        result = uniq_lines(lines, count=False, repeated=False, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == ["a", "b", "c"]

    def test_adjacent_duplicates(self) -> None:
        lines = ["a", "a", "b", "b", "c"]
        result = uniq_lines(lines, count=False, repeated=False, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == ["a", "b", "c"]

    def test_non_adjacent_duplicates(self) -> None:
        # uniq only removes adjacent duplicates.
        lines = ["a", "b", "a"]
        result = uniq_lines(lines, count=False, repeated=False, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == ["a", "b", "a"]

    def test_count_flag(self) -> None:
        lines = ["a", "a", "b"]
        result = uniq_lines(lines, count=True, repeated=False, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert "2 a" in result[0]
        assert "1 b" in result[1]

    def test_repeated_flag(self) -> None:
        lines = ["a", "a", "b", "c", "c"]
        result = uniq_lines(lines, count=False, repeated=True, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == ["a", "c"]

    def test_unique_flag(self) -> None:
        lines = ["a", "a", "b", "c", "c"]
        result = uniq_lines(lines, count=False, repeated=False, unique=True,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == ["b"]

    def test_ignore_case(self) -> None:
        lines = ["Hello", "hello", "HELLO"]
        result = uniq_lines(lines, count=False, repeated=False, unique=False,
                            ignore_case=True, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert len(result) == 1

    def test_empty_input(self) -> None:
        result = uniq_lines([], count=False, repeated=False, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == []

    def test_single_line(self) -> None:
        result = uniq_lines(["only"], count=False, repeated=False, unique=False,
                            ignore_case=False, skip_fields=0, skip_chars=0,
                            check_chars=None)
        assert result == ["only"]
