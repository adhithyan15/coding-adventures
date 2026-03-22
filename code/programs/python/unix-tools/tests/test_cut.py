"""Tests for the cut tool.

=== What These Tests Verify ===

These tests exercise the cut implementation, including:

1. Spec loading and CLI Builder integration
2. Range list parsing
3. Byte selection (-b)
4. Character selection (-c)
5. Field selection (-f)
6. Custom delimiters (-d)
7. Only-delimited mode (-s)
8. Output delimiter (--output-delimiter)
9. Complement mode (--complement)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "cut.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from cut_tool import cut_line, parse_range_list


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# CLI Builder integration tests
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["cut", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["cut", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_bytes_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cut", "-b", "1-3"])
        assert isinstance(result, ParseResult)
        assert result.flags["bytes"] == "1-3"

    def test_fields_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cut", "-f", "1,3"])
        assert isinstance(result, ParseResult)
        assert result.flags["fields"] == "1,3"

    def test_delimiter_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cut", "-f", "1", "-d", ","])
        assert isinstance(result, ParseResult)
        assert result.flags["delimiter"] == ","


# ---------------------------------------------------------------------------
# Range list parsing tests
# ---------------------------------------------------------------------------


class TestParseRangeList:
    def test_single_index(self) -> None:
        assert parse_range_list("3", 10) == [3]

    def test_range(self) -> None:
        assert parse_range_list("2-5", 10) == [2, 3, 4, 5]

    def test_open_end_range(self) -> None:
        assert parse_range_list("8-", 10) == [8, 9, 10]

    def test_open_start_range(self) -> None:
        assert parse_range_list("-3", 10) == [1, 2, 3]

    def test_multiple_specs(self) -> None:
        assert parse_range_list("1,3,5", 10) == [1, 3, 5]

    def test_mixed_specs(self) -> None:
        assert parse_range_list("1-3,7-", 10) == [1, 2, 3, 7, 8, 9, 10]

    def test_overlapping_ranges(self) -> None:
        result = parse_range_list("1-5,3-7", 10)
        assert result == [1, 2, 3, 4, 5, 6, 7]

    def test_out_of_bounds(self) -> None:
        """Indices beyond max_index are ignored."""
        assert parse_range_list("8-15", 10) == [8, 9, 10]

    def test_empty_spec(self) -> None:
        assert parse_range_list("", 10) == []


# ---------------------------------------------------------------------------
# cut_line tests: character mode
# ---------------------------------------------------------------------------


class TestCutCharacters:
    def test_single_char(self) -> None:
        assert cut_line("hello", chars_list="1") == "h"

    def test_char_range(self) -> None:
        assert cut_line("hello", chars_list="1-3") == "hel"

    def test_multiple_chars(self) -> None:
        assert cut_line("hello", chars_list="1,3,5") == "hlo"

    def test_open_end(self) -> None:
        assert cut_line("hello", chars_list="3-") == "llo"

    def test_complement(self) -> None:
        """Complement selects everything except the specified chars."""
        assert cut_line("hello", chars_list="1", complement=True) == "ello"


# ---------------------------------------------------------------------------
# cut_line tests: byte mode
# ---------------------------------------------------------------------------


class TestCutBytes:
    def test_single_byte(self) -> None:
        assert cut_line("hello", bytes_list="1") == "h"

    def test_byte_range(self) -> None:
        assert cut_line("hello", bytes_list="1-3") == "hel"

    def test_complement_bytes(self) -> None:
        assert cut_line("hello", bytes_list="1-2", complement=True) == "llo"


# ---------------------------------------------------------------------------
# cut_line tests: field mode
# ---------------------------------------------------------------------------


class TestCutFields:
    def test_single_field_tab(self) -> None:
        assert cut_line("a\tb\tc", fields_list="2") == "b"

    def test_multiple_fields(self) -> None:
        assert cut_line("a\tb\tc", fields_list="1,3") == "a\tc"

    def test_field_range(self) -> None:
        assert cut_line("a\tb\tc\td", fields_list="2-3") == "b\tc"

    def test_custom_delimiter(self) -> None:
        assert cut_line("a,b,c", fields_list="2", delimiter=",") == "b"

    def test_no_delimiter_passthrough(self) -> None:
        """Lines without the delimiter are passed through unchanged."""
        assert cut_line("no tabs here", fields_list="1") == "no tabs here"

    def test_only_delimited(self) -> None:
        """With -s, lines without delimiter are suppressed (return None)."""
        result = cut_line(
            "no tabs here",
            fields_list="1",
            only_delimited=True,
        )
        assert result is None

    def test_output_delimiter(self) -> None:
        result = cut_line(
            "a\tb\tc",
            fields_list="1,3",
            output_delimiter=",",
        )
        assert result == "a,c"

    def test_complement_fields(self) -> None:
        result = cut_line("a\tb\tc", fields_list="2", complement=True)
        assert result == "a\tc"


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_empty_line_chars(self) -> None:
        assert cut_line("", chars_list="1") == ""

    def test_empty_line_fields(self) -> None:
        assert cut_line("", fields_list="1") == ""

    def test_missing_mode_raises(self) -> None:
        with pytest.raises(ValueError, match="must be provided"):
            cut_line("hello")
