"""Tests for the head tool.

=== What These Tests Verify ===

These tests exercise the head implementation, including:

1. Spec loading and default behavior
2. The -n flag (number of lines)
3. The -c flag (number of bytes)
4. Header display logic (single file vs multiple files)
5. CLI Builder integration (--help, --version)
6. Business logic functions (head_lines, head_bytes, should_print_header)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "head.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder and parse argv
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the head spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading and default behavior
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the head.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head"])
        assert isinstance(result, ParseResult)

    def test_default_lines_is_10(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("lines", 10) == 10


# ---------------------------------------------------------------------------
# Test: -n flag (line count)
# ---------------------------------------------------------------------------


class TestLinesFlag:
    """The ``-n`` flag controls how many lines to output."""

    def test_short_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head", "-n", "5"])
        assert isinstance(result, ParseResult)
        assert result.flags["lines"] == 5

    def test_long_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head", "--lines", "20"])
        assert isinstance(result, ParseResult)
        assert result.flags["lines"] == 20


# ---------------------------------------------------------------------------
# Test: -c flag (byte count)
# ---------------------------------------------------------------------------


class TestBytesFlag:
    """The ``-c`` flag switches to byte-counting mode."""

    def test_short_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head", "-c", "100"])
        assert isinstance(result, ParseResult)
        assert result.flags["bytes"] == 100


# ---------------------------------------------------------------------------
# Test: -q and -v flags
# ---------------------------------------------------------------------------


class TestHeaderFlags:
    """The ``-q`` and ``-v`` flags control header display."""

    def test_quiet_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head", "-q"])
        assert isinstance(result, ParseResult)
        assert result.flags["quiet"] is True

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["head", "-v"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["head", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["head", "--help"])
        assert isinstance(result, HelpResult)
        assert "head" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["head", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["head", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic functions
# ---------------------------------------------------------------------------


class TestHeadLines:
    """Test the head_lines function directly."""

    def test_head_first_5_lines(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import head_lines

        lines = [f"line {i}\n" for i in range(1, 21)]
        result = head_lines(lines, 5)
        assert result == "line 1\nline 2\nline 3\nline 4\nline 5\n"

    def test_head_more_than_available(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import head_lines

        lines = ["one\n", "two\n"]
        result = head_lines(lines, 10)
        assert result == "one\ntwo\n"

    def test_head_zero_lines(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import head_lines

        lines = ["one\n", "two\n"]
        result = head_lines(lines, 0)
        assert result == ""

    def test_head_empty_input(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import head_lines

        result = head_lines([], 10)
        assert result == ""


class TestHeadBytes:
    """Test the head_bytes function directly."""

    def test_head_first_5_bytes(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import head_bytes

        data = b"Hello, World!"
        result = head_bytes(data, 5)
        assert result == b"Hello"

    def test_head_more_bytes_than_available(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import head_bytes

        data = b"Hi"
        result = head_bytes(data, 100)
        assert result == b"Hi"


class TestShouldPrintHeader:
    """Test the should_print_header logic."""

    def test_single_file_no_header(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import should_print_header

        assert should_print_header(num_files=1, quiet=False, verbose=False) is False

    def test_multiple_files_shows_header(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import should_print_header

        assert should_print_header(num_files=2, quiet=False, verbose=False) is True

    def test_quiet_suppresses_header(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import should_print_header

        assert should_print_header(num_files=3, quiet=True, verbose=False) is False

    def test_verbose_forces_header(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from head_tool import should_print_header

        assert should_print_header(num_files=1, quiet=False, verbose=True) is True
