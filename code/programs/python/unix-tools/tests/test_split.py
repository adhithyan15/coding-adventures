"""Tests for the split tool.

=== What These Tests Verify ===

These tests exercise the split implementation, including:

1. Spec loading and CLI Builder integration
2. Suffix generation (generate_suffix)
3. Filename construction (make_filename)
4. Size parsing (parse_size)
5. Splitting by lines (split_by_lines)
6. Splitting by bytes (split_by_bytes)
7. Splitting by number (split_by_number)
8. Numeric and hex suffixes
9. Custom suffix lengths
10. Additional suffixes
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "split.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from split_tool import (
    generate_suffix,
    make_filename,
    parse_size,
    split_by_bytes,
    split_by_lines,
    split_by_number,
)


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["split", "file.txt"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------


class TestFlags:
    def test_lines_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["split", "-l", "100", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["lines"] == 100

    def test_bytes_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["split", "-b", "1M", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["bytes"] == "1M"

    def test_numeric_suffixes(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["split", "-d", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["numeric_suffixes"] is True

    def test_suffix_length(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["split", "-a", "4", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["suffix_length"] == 4


# ---------------------------------------------------------------------------
# Help and version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["split", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["split", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# generate_suffix
# ---------------------------------------------------------------------------


class TestGenerateSuffix:
    def test_alphabetic_first(self) -> None:
        assert generate_suffix(0, 2) == "aa"

    def test_alphabetic_second(self) -> None:
        assert generate_suffix(1, 2) == "ab"

    def test_alphabetic_wrap(self) -> None:
        """Index 26 should wrap to 'ba'."""
        assert generate_suffix(26, 2) == "ba"

    def test_alphabetic_last_two(self) -> None:
        """Index 675 = 26*26 - 1 should be 'zz'."""
        assert generate_suffix(675, 2) == "zz"

    def test_alphabetic_three_chars(self) -> None:
        assert generate_suffix(0, 3) == "aaa"
        assert generate_suffix(1, 3) == "aab"

    def test_numeric_first(self) -> None:
        assert generate_suffix(0, 2, numeric=True) == "00"

    def test_numeric_padded(self) -> None:
        assert generate_suffix(5, 2, numeric=True) == "05"

    def test_numeric_large(self) -> None:
        assert generate_suffix(42, 2, numeric=True) == "42"

    def test_hex_first(self) -> None:
        assert generate_suffix(0, 2, hexadecimal=True) == "00"

    def test_hex_value(self) -> None:
        assert generate_suffix(15, 2, hexadecimal=True) == "0f"

    def test_hex_large(self) -> None:
        assert generate_suffix(255, 2, hexadecimal=True) == "ff"

    def test_overflow_raises(self) -> None:
        """Exceeding suffix space raises ValueError."""
        with pytest.raises(ValueError, match="exhausted"):
            generate_suffix(676, 2)  # 26*26 = 676 is too many.

    def test_numeric_overflow(self) -> None:
        with pytest.raises(ValueError, match="exhausted"):
            generate_suffix(100, 2, numeric=True)

    def test_hex_overflow(self) -> None:
        with pytest.raises(ValueError, match="exhausted"):
            generate_suffix(256, 2, hexadecimal=True)


# ---------------------------------------------------------------------------
# make_filename
# ---------------------------------------------------------------------------


class TestMakeFilename:
    def test_default(self) -> None:
        assert make_filename("x", 0) == "xaa"

    def test_custom_prefix(self) -> None:
        assert make_filename("chunk_", 0) == "chunk_aa"

    def test_numeric(self) -> None:
        assert make_filename("x", 3, numeric=True) == "x03"

    def test_additional_suffix(self) -> None:
        assert make_filename("x", 0, additional_suffix=".txt") == "xaa.txt"

    def test_combined(self) -> None:
        result = make_filename("out_", 5, 3, numeric=True, additional_suffix=".csv")
        assert result == "out_005.csv"


# ---------------------------------------------------------------------------
# parse_size
# ---------------------------------------------------------------------------


class TestParseSize:
    def test_bare_number(self) -> None:
        assert parse_size("1024") == 1024

    def test_kilobytes(self) -> None:
        assert parse_size("1K") == 1024

    def test_megabytes(self) -> None:
        assert parse_size("2M") == 2 * 1024 * 1024

    def test_gigabytes(self) -> None:
        assert parse_size("1G") == 1024 ** 3

    def test_kb_suffix(self) -> None:
        assert parse_size("1KB") == 1024

    def test_case_insensitive(self) -> None:
        assert parse_size("1k") == 1024
        assert parse_size("1m") == 1024 * 1024

    def test_fractional(self) -> None:
        assert parse_size("1.5K") == int(1.5 * 1024)


# ---------------------------------------------------------------------------
# split_by_lines
# ---------------------------------------------------------------------------


class TestSplitByLines:
    def test_basic_split(self) -> None:
        data = "a\nb\nc\nd\ne\n"
        result = split_by_lines(data, 2, "x")
        assert len(result) == 3
        assert result[0] == ("xaa", "a\nb\n")
        assert result[1] == ("xab", "c\nd\n")
        assert result[2] == ("xac", "e\n")

    def test_exact_split(self) -> None:
        """Data evenly divisible by lines_per_chunk."""
        data = "a\nb\nc\nd\n"
        result = split_by_lines(data, 2, "x")
        assert len(result) == 2

    def test_single_line_per_chunk(self) -> None:
        data = "a\nb\nc\n"
        result = split_by_lines(data, 1, "x")
        assert len(result) == 3

    def test_all_in_one_chunk(self) -> None:
        data = "a\nb\nc\n"
        result = split_by_lines(data, 100, "x")
        assert len(result) == 1

    def test_empty_data(self) -> None:
        result = split_by_lines("", 10, "x")
        assert result == []

    def test_numeric_suffixes(self) -> None:
        data = "a\nb\nc\n"
        result = split_by_lines(data, 1, "out_", numeric=True)
        assert result[0][0] == "out_00"
        assert result[1][0] == "out_01"

    def test_custom_suffix_length(self) -> None:
        data = "a\nb\n"
        result = split_by_lines(data, 1, "x", suffix_length=3)
        assert result[0][0] == "xaaa"

    def test_additional_suffix(self) -> None:
        data = "a\nb\n"
        result = split_by_lines(data, 1, "x", additional_suffix=".txt")
        assert result[0][0] == "xaa.txt"


# ---------------------------------------------------------------------------
# split_by_bytes
# ---------------------------------------------------------------------------


class TestSplitByBytes:
    def test_basic_split(self) -> None:
        data = b"abcdefgh"
        result = split_by_bytes(data, 3, "x")
        assert len(result) == 3
        assert result[0] == ("xaa", b"abc")
        assert result[1] == ("xab", b"def")
        assert result[2] == ("xac", b"gh")

    def test_exact_split(self) -> None:
        data = b"abcdef"
        result = split_by_bytes(data, 3, "x")
        assert len(result) == 2

    def test_single_byte_chunks(self) -> None:
        data = b"abc"
        result = split_by_bytes(data, 1, "x")
        assert len(result) == 3

    def test_empty_data(self) -> None:
        result = split_by_bytes(b"", 10, "x")
        assert result == []

    def test_chunk_larger_than_data(self) -> None:
        data = b"abc"
        result = split_by_bytes(data, 100, "x")
        assert len(result) == 1
        assert result[0] == ("xaa", b"abc")


# ---------------------------------------------------------------------------
# split_by_number
# ---------------------------------------------------------------------------


class TestSplitByNumber:
    def test_even_split(self) -> None:
        data = b"abcdef"
        result = split_by_number(data, 3, "x")
        assert len(result) == 3
        assert result[0][1] == b"ab"
        assert result[1][1] == b"cd"
        assert result[2][1] == b"ef"

    def test_uneven_split(self) -> None:
        """Extra bytes go to the first chunks."""
        data = b"abcdefg"  # 7 bytes into 3 chunks: 3+2+2
        result = split_by_number(data, 3, "x")
        assert len(result) == 3
        assert len(result[0][1]) == 3
        assert len(result[1][1]) == 2
        assert len(result[2][1]) == 2
        # Concatenation should equal original.
        assert b"".join(chunk for _, chunk in result) == data

    def test_more_chunks_than_bytes(self) -> None:
        """Some chunks will be empty if N > len(data)."""
        data = b"ab"
        result = split_by_number(data, 5, "x")
        assert len(result) == 5
        non_empty = [chunk for _, chunk in result if chunk]
        assert len(non_empty) == 2

    def test_single_chunk(self) -> None:
        data = b"hello"
        result = split_by_number(data, 1, "x")
        assert len(result) == 1
        assert result[0][1] == b"hello"

    def test_empty_data(self) -> None:
        result = split_by_number(b"", 3, "x")
        assert len(result) == 3
        assert all(chunk == b"" for _, chunk in result)


# ---------------------------------------------------------------------------
# Integration: split_by_lines with real file-like content
# ---------------------------------------------------------------------------


class TestIntegration:
    def test_csv_split(self) -> None:
        """Splitting a CSV-like file by lines."""
        data = "id,name\n1,Alice\n2,Bob\n3,Charlie\n4,Dave\n"
        result = split_by_lines(data, 2, "data_", numeric=True, additional_suffix=".csv")
        assert len(result) == 3
        assert result[0][0] == "data_00.csv"
        assert result[0][1] == "id,name\n1,Alice\n"
        assert result[1][0] == "data_01.csv"
        assert result[2][0] == "data_02.csv"

    def test_round_trip(self) -> None:
        """Splitting and concatenating produces the original data."""
        data = "line1\nline2\nline3\nline4\nline5\n"
        chunks = split_by_lines(data, 2, "x")
        reassembled = "".join(content for _, content in chunks)
        assert reassembled == data
