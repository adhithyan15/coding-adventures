"""Tests for the cat tool.

=== What These Tests Verify ===

These tests exercise the cat implementation, including:

1. Basic file concatenation
2. The -n flag (number all lines)
3. The -b flag (number non-blank lines, overrides -n)
4. The -s flag (squeeze blank lines)
5. The -E flag (show $ at end of each line)
6. The -T flag (show tabs as ^I)
7. The -A flag (equivalent to -vET)
8. CLI Builder integration (--help, --version)
9. Business logic functions directly
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "cat.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder and parse argv
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the cat spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Default behavior
# ---------------------------------------------------------------------------


class TestDefaultBehavior:
    """When invoked with no flags, cat should return a ParseResult."""

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat"])
        assert isinstance(result, ParseResult)

    def test_default_file_is_stdin(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat"])
        assert isinstance(result, ParseResult)
        # Default file should be stdin ("-")
        files = result.arguments.get("files", ["-"])
        if isinstance(files, str):
            assert files == "-"
        else:
            assert files == ["-"] or files == []

    def test_file_argument_parsed(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat", "myfile.txt"])
        assert isinstance(result, ParseResult)
        files = result.arguments.get("files", [])
        assert "myfile.txt" in (files if isinstance(files, list) else [files])


# ---------------------------------------------------------------------------
# Test: Line numbering (-n)
# ---------------------------------------------------------------------------


class TestNumberFlag:
    """The ``-n`` flag should set the number flag."""

    def test_n_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat", "-n"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("number") is True

    def test_n_flag_long(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat", "--number"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("number") is True


# ---------------------------------------------------------------------------
# Test: Non-blank line numbering (-b)
# ---------------------------------------------------------------------------


class TestNumberNonblankFlag:
    """The ``-b`` flag should set number_nonblank."""

    def test_b_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat", "-b"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("number_nonblank") is True

    def test_b_flag_long(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cat", "--number-nonblank"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("number_nonblank") is True


# ---------------------------------------------------------------------------
# Test: Business logic functions
# ---------------------------------------------------------------------------


class TestProcessLine:
    """Test the process_line function directly."""

    def test_plain_line(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import process_line

        result = process_line(
            "hello\n",
            show_ends=False,
            show_tabs=False,
            show_nonprinting_flag=False,
        )
        assert result == "hello\n"

    def test_show_ends(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import process_line

        result = process_line(
            "hello\n",
            show_ends=True,
            show_tabs=False,
            show_nonprinting_flag=False,
        )
        assert result == "hello$\n"

    def test_show_tabs(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import process_line

        result = process_line(
            "hello\tworld\n",
            show_ends=False,
            show_tabs=True,
            show_nonprinting_flag=False,
        )
        assert result == "hello^Iworld\n"

    def test_show_ends_and_tabs(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import process_line

        result = process_line(
            "a\tb\n",
            show_ends=True,
            show_tabs=True,
            show_nonprinting_flag=False,
        )
        assert result == "a^Ib$\n"

    def test_line_without_newline(self) -> None:
        """A line without a trailing newline should not get one added."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import process_line

        result = process_line(
            "hello",
            show_ends=True,
            show_tabs=False,
            show_nonprinting_flag=False,
        )
        assert result == "hello$"  # $ but no newline


class TestCatStream:
    """Test the cat_stream function directly."""

    def test_basic_output(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import cat_stream

        lines = ["hello\n", "world\n"]
        cat_stream(
            lines,
            number=False,
            number_nonblank=False,
            squeeze_blank=False,
            show_ends=False,
            show_tabs=False,
            show_nonprinting_flag=False,
            line_counter=0,
        )
        captured = capsys.readouterr()
        assert captured.out == "hello\nworld\n"

    def test_number_lines(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import cat_stream

        lines = ["hello\n", "world\n"]
        counter = cat_stream(
            lines,
            number=True,
            number_nonblank=False,
            squeeze_blank=False,
            show_ends=False,
            show_tabs=False,
            show_nonprinting_flag=False,
            line_counter=0,
        )
        captured = capsys.readouterr()
        assert "     1\thello\n" in captured.out
        assert "     2\tworld\n" in captured.out
        assert counter == 2

    def test_number_nonblank_lines(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import cat_stream

        lines = ["hello\n", "\n", "world\n"]
        counter = cat_stream(
            lines,
            number=False,
            number_nonblank=True,
            squeeze_blank=False,
            show_ends=False,
            show_tabs=False,
            show_nonprinting_flag=False,
            line_counter=0,
        )
        captured = capsys.readouterr()
        assert "     1\thello\n" in captured.out
        assert "     2\tworld\n" in captured.out
        # The blank line should NOT be numbered
        assert counter == 2

    def test_squeeze_blank(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import cat_stream

        lines = ["hello\n", "\n", "\n", "\n", "world\n"]
        cat_stream(
            lines,
            number=False,
            number_nonblank=False,
            squeeze_blank=True,
            show_ends=False,
            show_tabs=False,
            show_nonprinting_flag=False,
            line_counter=0,
        )
        captured = capsys.readouterr()
        # Three blank lines should be squeezed to one
        assert captured.out == "hello\n\nworld\n"

    def test_line_counter_continuity(self, capsys: pytest.CaptureFixture[str]) -> None:
        """Line counter should continue from where it left off."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import cat_stream

        lines = ["line\n"]
        counter = cat_stream(
            lines,
            number=True,
            number_nonblank=False,
            squeeze_blank=False,
            show_ends=False,
            show_tabs=False,
            show_nonprinting_flag=False,
            line_counter=5,
        )
        captured = capsys.readouterr()
        assert "     6\tline\n" in captured.out
        assert counter == 6


class TestShowNonprinting:
    """Test the show_nonprinting function directly."""

    def test_regular_char(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import show_nonprinting

        assert show_nonprinting("A") == "A"

    def test_nul_char(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import show_nonprinting

        assert show_nonprinting("\x00") == "^@"

    def test_del_char(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import show_nonprinting

        assert show_nonprinting("\x7f") == "^?"

    def test_tab_passthrough(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import show_nonprinting

        assert show_nonprinting("\t") == "\t"

    def test_newline_passthrough(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import show_nonprinting

        assert show_nonprinting("\n") == "\n"

    def test_high_byte(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from cat_tool import show_nonprinting

        # 0x80 = M-^@
        assert show_nonprinting(chr(128)) == "M-^@"


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["cat", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["cat", "--help"])
        assert isinstance(result, HelpResult)
        assert "cat" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["cat", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["cat", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"
