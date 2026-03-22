"""Tests for the join tool.

=== What These Tests Verify ===

These tests exercise the join implementation, including:

1. Spec loading and CLI Builder integration
2. Field splitting (split_line)
3. Field extraction (get_field, get_key)
4. Output formatting (format_output_line, format_unpaired_line)
5. Merge-join algorithm (join_files)
6. Custom join fields (-1, -2, -j)
7. Custom separator (-t)
8. Custom output format (-o)
9. Unpaired lines (-a, -v)
10. Case-insensitive joining (-i)
11. Header handling (--header)
12. Empty field replacement (-e)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "join.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from join_tool import (
    JoinOptions,
    format_output_line,
    format_unpaired_line,
    get_field,
    get_key,
    join_files,
    split_line,
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

        result = parse_argv(["join", "file1.txt", "file2.txt"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------


class TestFlags:
    def test_ignore_case_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["join", "-i", "f1", "f2"])
        assert isinstance(result, ParseResult)
        assert result.flags["ignore_case"] is True

    def test_header_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["join", "--header", "f1", "f2"])
        assert isinstance(result, ParseResult)
        assert result.flags["header"] is True

    def test_separator_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["join", "-t", ",", "f1", "f2"])
        assert isinstance(result, ParseResult)
        assert result.flags["separator"] == ","


# ---------------------------------------------------------------------------
# Help and version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["join", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["join", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# split_line
# ---------------------------------------------------------------------------


class TestSplitLine:
    def test_whitespace_default(self) -> None:
        """Default split uses whitespace."""
        assert split_line("Alice  Bob  Charlie") == ["Alice", "Bob", "Charlie"]

    def test_custom_separator(self) -> None:
        """Custom separator splits on that character."""
        assert split_line("a:b:c", separator=":") == ["a", "b", "c"]

    def test_tab_separator(self) -> None:
        assert split_line("a\tb\tc", separator="\t") == ["a", "b", "c"]

    def test_empty_line(self) -> None:
        assert split_line("") == []

    def test_single_field(self) -> None:
        assert split_line("hello") == ["hello"]

    def test_csv_separator(self) -> None:
        assert split_line("1,Alice,A+", separator=",") == ["1", "Alice", "A+"]


# ---------------------------------------------------------------------------
# get_field / get_key
# ---------------------------------------------------------------------------


class TestGetField:
    def test_valid_field(self) -> None:
        assert get_field(["a", "b", "c"], 1) == "a"
        assert get_field(["a", "b", "c"], 2) == "b"
        assert get_field(["a", "b", "c"], 3) == "c"

    def test_out_of_range(self) -> None:
        assert get_field(["a", "b"], 5) == ""

    def test_zero_field(self) -> None:
        """Field 0 is out of range (1-based)."""
        assert get_field(["a", "b"], 0) == ""


class TestGetKey:
    def test_basic_key(self) -> None:
        assert get_key(["1", "Alice"], 1) == "1"

    def test_case_insensitive(self) -> None:
        assert get_key(["Alice", "data"], 1, ignore_case=True) == "alice"


# ---------------------------------------------------------------------------
# format_output_line
# ---------------------------------------------------------------------------


class TestFormatOutputLine:
    def test_default_format(self) -> None:
        """Default format: join key + remaining fields from both files."""
        opts = JoinOptions()
        result = format_output_line(
            "1", ["1", "Alice"], ["1", "A+"], opts
        )
        assert result == "1 Alice A+"

    def test_custom_separator(self) -> None:
        opts = JoinOptions(separator=",")
        result = format_output_line(
            "1", ["1", "Alice"], ["1", "A+"], opts
        )
        assert result == "1,Alice,A+"

    def test_output_format(self) -> None:
        """Custom -o format selects specific fields."""
        opts = JoinOptions(output_format="1.2 2.2")
        result = format_output_line(
            "1", ["1", "Alice"], ["1", "A+"], opts
        )
        assert result == "Alice A+"

    def test_output_format_with_zero(self) -> None:
        """Format spec '0' outputs the join field."""
        opts = JoinOptions(output_format="0 1.2 2.2")
        result = format_output_line(
            "1", ["1", "Alice"], ["1", "A+"], opts
        )
        assert result == "1 Alice A+"


# ---------------------------------------------------------------------------
# format_unpaired_line
# ---------------------------------------------------------------------------


class TestFormatUnpairedLine:
    def test_default(self) -> None:
        opts = JoinOptions()
        result = format_unpaired_line(["1", "Alice"], opts, 1)
        assert result == "1 Alice"

    def test_with_format_and_empty(self) -> None:
        opts = JoinOptions(output_format="0 1.2 2.2", empty="---")
        result = format_unpaired_line(["1", "Alice"], opts, 1)
        assert "Alice" in result
        assert "---" in result


# ---------------------------------------------------------------------------
# join_files — basic merge join
# ---------------------------------------------------------------------------


class TestJoinFiles:
    def test_basic_join(self) -> None:
        """Join two files on the first field."""
        lines1 = ["1 Alice", "2 Bob", "3 Charlie"]
        lines2 = ["1 A+", "2 B", "4 C"]
        opts = JoinOptions()

        result = join_files(lines1, lines2, opts)
        assert len(result) == 2
        assert "1 Alice A+" in result
        assert "2 Bob B" in result

    def test_no_common_keys(self) -> None:
        """No matching keys produces empty output."""
        lines1 = ["1 Alice"]
        lines2 = ["2 Bob"]
        opts = JoinOptions()

        result = join_files(lines1, lines2, opts)
        assert result == []

    def test_all_keys_match(self) -> None:
        """All keys match — all lines appear in output."""
        lines1 = ["a X", "b Y"]
        lines2 = ["a 1", "b 2"]
        opts = JoinOptions()

        result = join_files(lines1, lines2, opts)
        assert len(result) == 2

    def test_duplicate_keys_cartesian(self) -> None:
        """Duplicate keys produce the Cartesian product."""
        lines1 = ["1 A", "1 B"]
        lines2 = ["1 X", "1 Y"]
        opts = JoinOptions()

        result = join_files(lines1, lines2, opts)
        assert len(result) == 4  # 2 x 2 = 4
        assert "1 A X" in result
        assert "1 A Y" in result
        assert "1 B X" in result
        assert "1 B Y" in result

    def test_empty_files(self) -> None:
        """Joining empty files produces empty output."""
        opts = JoinOptions()
        assert join_files([], [], opts) == []

    def test_one_empty_file(self) -> None:
        """Joining with one empty file produces empty output."""
        lines1 = ["1 Alice", "2 Bob"]
        opts = JoinOptions()
        assert join_files(lines1, [], opts) == []


# ---------------------------------------------------------------------------
# join_files — custom fields
# ---------------------------------------------------------------------------


class TestCustomFields:
    def test_join_on_field_2(self) -> None:
        """Join on the second field of both files."""
        lines1 = ["Alice 1", "Bob 2"]
        lines2 = ["A+ 1", "B 2"]
        opts = JoinOptions(field1=2, field2=2)

        result = join_files(lines1, lines2, opts)
        assert len(result) == 2

    def test_different_fields(self) -> None:
        """Join on field 1 of file1 and field 2 of file2."""
        lines1 = ["1 Alice"]
        lines2 = ["A+ 1"]
        opts = JoinOptions(field1=1, field2=2)

        result = join_files(lines1, lines2, opts)
        assert len(result) == 1


# ---------------------------------------------------------------------------
# join_files — unpaired lines
# ---------------------------------------------------------------------------


class TestUnpairedLines:
    def test_unpaired_file1(self) -> None:
        """With -a 1, show unpaired lines from file 1."""
        lines1 = ["1 Alice", "2 Bob", "3 Charlie"]
        lines2 = ["1 A+", "2 B"]
        opts = JoinOptions(unpaired=["1"])

        result = join_files(lines1, lines2, opts)
        assert len(result) == 3  # 2 paired + 1 unpaired
        assert any("Charlie" in line for line in result)

    def test_unpaired_file2(self) -> None:
        """With -a 2, show unpaired lines from file 2."""
        lines1 = ["1 Alice"]
        lines2 = ["1 A+", "2 B"]
        opts = JoinOptions(unpaired=["2"])

        result = join_files(lines1, lines2, opts)
        assert len(result) == 2  # 1 paired + 1 unpaired
        assert any("B" in line for line in result)

    def test_only_unpaired_v1(self) -> None:
        """With -v 1, show ONLY unpaired lines from file 1."""
        lines1 = ["1 Alice", "2 Bob", "3 Charlie"]
        lines2 = ["1 A+", "2 B"]
        opts = JoinOptions(only_unpaired="1")

        result = join_files(lines1, lines2, opts)
        assert len(result) == 1
        assert "Charlie" in result[0]

    def test_only_unpaired_v2(self) -> None:
        """With -v 2, show ONLY unpaired lines from file 2."""
        lines1 = ["1 Alice"]
        lines2 = ["1 A+", "2 B"]
        opts = JoinOptions(only_unpaired="2")

        result = join_files(lines1, lines2, opts)
        assert len(result) == 1
        assert "B" in result[0]


# ---------------------------------------------------------------------------
# join_files — case insensitive
# ---------------------------------------------------------------------------


class TestCaseInsensitive:
    def test_ignore_case(self) -> None:
        """With ignore_case=True, keys are compared case-insensitively."""
        lines1 = ["Alice data1"]
        lines2 = ["alice data2"]
        opts = JoinOptions(ignore_case=True)

        result = join_files(lines1, lines2, opts)
        assert len(result) == 1


# ---------------------------------------------------------------------------
# join_files — custom separator
# ---------------------------------------------------------------------------


class TestCustomSeparator:
    def test_comma_separator(self) -> None:
        """With separator=',', fields are split on commas."""
        lines1 = ["1,Alice", "2,Bob"]
        lines2 = ["1,A+", "2,B"]
        opts = JoinOptions(separator=",")

        result = join_files(lines1, lines2, opts)
        assert len(result) == 2
        assert "1,Alice,A+" in result

    def test_tab_separator(self) -> None:
        lines1 = ["1\tAlice", "2\tBob"]
        lines2 = ["1\tA+", "2\tB"]
        opts = JoinOptions(separator="\t")

        result = join_files(lines1, lines2, opts)
        assert len(result) == 2


# ---------------------------------------------------------------------------
# join_files — header
# ---------------------------------------------------------------------------


class TestHeader:
    def test_header_line(self) -> None:
        """With header=True, the first line is treated as a header."""
        lines1 = ["ID Name", "1 Alice", "2 Bob"]
        lines2 = ["ID Grade", "1 A+", "2 B"]
        opts = JoinOptions(header=True)

        result = join_files(lines1, lines2, opts)
        # Header should be joined and appear first.
        assert result[0] == "ID Name Grade"
        assert len(result) == 3  # header + 2 data lines
