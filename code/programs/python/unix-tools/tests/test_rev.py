"""Tests for the rev tool.

=== What These Tests Verify ===

These tests exercise the rev implementation, including:

1. Spec loading and default behavior
2. Character reversal with and without trailing newlines
3. Multi-line input
4. Empty input
5. CLI Builder integration (--help, --version)
6. Business logic function (reverse_line)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "rev.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the rev spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the rev.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_no_args_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rev"])
        assert isinstance(result, ParseResult)

    def test_with_file_argument(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rev", "input.txt"])
        assert isinstance(result, ParseResult)
        files = result.arguments.get("files", [])
        if isinstance(files, str):
            assert files == "input.txt"
        else:
            assert "input.txt" in files


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["rev", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["rev", "--help"])
        assert isinstance(result, HelpResult)
        assert "rev" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["rev", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["rev", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — reverse_line
# ---------------------------------------------------------------------------


class TestReverseLine:
    """Test the reverse_line function directly."""

    def test_simple_string(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("hello\n") == "olleh\n"

    def test_without_newline(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("hello") == "olleh"

    def test_single_char(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("a\n") == "a\n"

    def test_empty_line(self) -> None:
        """An empty line (just a newline) stays the same."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("\n") == "\n"

    def test_empty_string(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("") == ""

    def test_palindrome(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("racecar\n") == "racecar\n"

    def test_with_spaces(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("Hello, World!\n") == "!dlroW ,olleH\n"

    def test_numbers(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import reverse_line

        assert reverse_line("12345\n") == "54321\n"


# ---------------------------------------------------------------------------
# Test: File-based reversal
# ---------------------------------------------------------------------------


class TestReadAndReverse:
    """Test the read_and_reverse function with actual files."""

    def test_reverse_file(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import read_and_reverse

        test_file = tmp_path / "test.txt"
        test_file.write_text("hello\nworld\n")

        read_and_reverse(str(test_file))
        captured = capsys.readouterr()
        assert captured.out == "olleh\ndlrow\n"

    def test_reverse_empty_file(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from rev_tool import read_and_reverse

        test_file = tmp_path / "empty.txt"
        test_file.write_text("")

        read_and_reverse(str(test_file))
        captured = capsys.readouterr()
        assert captured.out == ""
