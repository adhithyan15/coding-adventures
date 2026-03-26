"""Tests for the sort tool.

=== What These Tests Verify ===

These tests exercise the sort implementation, including:

1. Spec loading and CLI Builder integration
2. Default lexicographic sorting
3. Reverse sorting (-r)
4. Numeric sorting (-n)
5. Case-insensitive sorting (-f)
6. Unique filtering (-u)
7. Key-based sorting (-k)
8. Human-readable numeric sorting (-h)
9. Month sorting (-M)
10. Version sorting (-V)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "sort.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from sort_tool import sort_lines


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# CLI Builder integration tests
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sort"])
        assert isinstance(result, ParseResult)


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["sort", "--help"])
        assert isinstance(result, HelpResult)
        assert "sort" in result.text.lower()

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["sort", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_reverse_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sort", "-r"])
        assert isinstance(result, ParseResult)
        assert result.flags["reverse"] is True

    def test_numeric_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sort", "-n"])
        assert isinstance(result, ParseResult)
        assert result.flags["numeric_sort"] is True

    def test_ignore_case_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sort", "-f"])
        assert isinstance(result, ParseResult)
        assert result.flags["ignore_case"] is True

    def test_unique_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sort", "-u"])
        assert isinstance(result, ParseResult)
        assert result.flags["unique"] is True

    def test_unknown_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["sort", "--nonexistent"])


# ---------------------------------------------------------------------------
# Business logic tests
# ---------------------------------------------------------------------------


class TestDefaultSort:
    """Default lexicographic sort."""

    def test_already_sorted(self) -> None:
        lines = ["a", "b", "c"]
        assert sort_lines(lines) == ["a", "b", "c"]

    def test_unsorted(self) -> None:
        lines = ["banana", "apple", "cherry"]
        assert sort_lines(lines) == ["apple", "banana", "cherry"]

    def test_empty_input(self) -> None:
        assert sort_lines([]) == []

    def test_single_line(self) -> None:
        assert sort_lines(["only"]) == ["only"]

    def test_duplicate_lines(self) -> None:
        lines = ["b", "a", "b", "a"]
        assert sort_lines(lines) == ["a", "a", "b", "b"]

    def test_numeric_strings_sorted_lexically(self) -> None:
        """Without -n, "10" sorts before "9" (lexicographic)."""
        lines = ["10", "9", "1"]
        result = sort_lines(lines)
        assert result == ["1", "10", "9"]


class TestReverseSort:
    def test_reverse(self) -> None:
        lines = ["a", "c", "b"]
        assert sort_lines(lines, reverse=True) == ["c", "b", "a"]

    def test_reverse_numeric(self) -> None:
        lines = ["1", "10", "2"]
        result = sort_lines(lines, reverse=True, numeric=True)
        assert result == ["10", "2", "1"]


class TestNumericSort:
    def test_numeric(self) -> None:
        lines = ["10", "9", "1", "100"]
        result = sort_lines(lines, numeric=True)
        assert result == ["1", "9", "10", "100"]

    def test_numeric_with_text(self) -> None:
        """Non-numeric lines sort before numeric ones."""
        lines = ["10", "abc", "2"]
        result = sort_lines(lines, numeric=True)
        # "abc" has no numeric value, sorts as (0, 0.0, ...)
        assert result[0] == "abc"

    def test_numeric_with_negative(self) -> None:
        lines = ["5", "-3", "0", "10"]
        result = sort_lines(lines, numeric=True)
        assert result == ["-3", "0", "5", "10"]

    def test_numeric_with_decimal(self) -> None:
        lines = ["1.5", "1.1", "2.0"]
        result = sort_lines(lines, numeric=True)
        assert result == ["1.1", "1.5", "2.0"]


class TestIgnoreCaseSort:
    def test_case_insensitive(self) -> None:
        lines = ["Banana", "apple", "Cherry"]
        result = sort_lines(lines, ignore_case=True)
        assert result == ["apple", "Banana", "Cherry"]


class TestUniqueSort:
    def test_unique(self) -> None:
        lines = ["b", "a", "b", "a", "c"]
        result = sort_lines(lines, unique=True)
        assert result == ["a", "b", "c"]

    def test_unique_case_insensitive(self) -> None:
        lines = ["Hello", "hello", "HELLO", "world"]
        result = sort_lines(lines, unique=True, ignore_case=True)
        assert len(result) == 2


class TestKeySort:
    def test_sort_by_second_field(self) -> None:
        lines = ["b 1", "a 3", "c 2"]
        result = sort_lines(lines, key_defs=["2,2"])
        assert result == ["b 1", "c 2", "a 3"]

    def test_sort_by_second_field_numeric(self) -> None:
        lines = ["x 10", "y 2", "z 1"]
        result = sort_lines(lines, key_defs=["2,2n"])
        assert result == ["z 1", "y 2", "x 10"]

    def test_sort_with_separator(self) -> None:
        lines = ["b:1", "a:3", "c:2"]
        result = sort_lines(lines, key_defs=["2,2"], field_sep=":")
        assert result == ["b:1", "c:2", "a:3"]


class TestHumanNumericSort:
    def test_human_numeric(self) -> None:
        lines = ["1K", "1M", "1G"]
        result = sort_lines(lines, human_numeric=True)
        assert result == ["1K", "1M", "1G"]

    def test_human_numeric_reverse(self) -> None:
        lines = ["1K", "1M", "1G"]
        result = sort_lines(lines, human_numeric=True, reverse=True)
        assert result == ["1G", "1M", "1K"]


class TestMonthSort:
    def test_month_sort(self) -> None:
        lines = ["MAR", "JAN", "FEB"]
        result = sort_lines(lines, month=True)
        assert result == ["JAN", "FEB", "MAR"]

    def test_month_sort_unknown(self) -> None:
        """Unknown months sort before JAN."""
        lines = ["FEB", "XYZ", "JAN"]
        result = sort_lines(lines, month=True)
        assert result[0] == "XYZ"


class TestVersionSort:
    def test_version_sort(self) -> None:
        lines = ["file10", "file2", "file1"]
        result = sort_lines(lines, version=True)
        assert result == ["file1", "file2", "file10"]
