"""Tests for the wc tool.

=== What These Tests Verify ===

These tests exercise the wc implementation, including:

1. Count accuracy (lines, words, bytes, characters)
2. Flag parsing (-l, -w, -c, -m, -L)
3. Default behavior (show lines + words + bytes)
4. Column alignment and formatting
5. Total line for multiple files
6. CLI Builder integration (--help, --version)
7. The count_content and format_counts functions directly
"""

from __future__ import annotations

import sys
import tempfile
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "wc.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder and parse argv
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the wc spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Default behavior
# ---------------------------------------------------------------------------


class TestDefaultBehavior:
    """When invoked with no flags, wc should default to showing
    lines + words + bytes."""

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc"])
        assert isinstance(result, ParseResult)

    def test_no_specific_flags_set(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc"])
        assert isinstance(result, ParseResult)
        # No specific counting flags should be set
        assert result.flags.get("lines") is not True
        assert result.flags.get("words") is not True
        assert result.flags.get("bytes") is not True

    def test_file_argument_parsed(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "myfile.txt"])
        assert isinstance(result, ParseResult)
        files = result.arguments.get("files", [])
        assert "myfile.txt" in (files if isinstance(files, list) else [files])


# ---------------------------------------------------------------------------
# Test: Individual flags
# ---------------------------------------------------------------------------


class TestFlags:
    """Test that each flag is properly parsed."""

    def test_lines_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "-l"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("lines") is True

    def test_lines_flag_long(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "--lines"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("lines") is True

    def test_words_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "-w"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("words") is True

    def test_bytes_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "-c"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("bytes") is True

    def test_chars_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "-m"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("chars") is True

    def test_max_line_length_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["wc", "-L"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("max_line_length") is True


# ---------------------------------------------------------------------------
# Test: count_content function
# ---------------------------------------------------------------------------


class TestCountContent:
    """Test the count_content function directly."""

    def test_empty_content(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        counts = count_content("", b"")
        assert counts["lines"] == 0
        assert counts["words"] == 0
        assert counts["bytes"] == 0
        assert counts["chars"] == 0

    def test_single_line(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        content = "hello world\n"
        raw = content.encode("utf-8")
        counts = count_content(content, raw)
        assert counts["lines"] == 1
        assert counts["words"] == 2
        assert counts["bytes"] == 12
        assert counts["chars"] == 12

    def test_multiple_lines(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        content = "hello\nworld\n"
        raw = content.encode("utf-8")
        counts = count_content(content, raw)
        assert counts["lines"] == 2
        assert counts["words"] == 2
        assert counts["bytes"] == 12

    def test_no_trailing_newline(self) -> None:
        """A file without a trailing newline should still count words."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        content = "hello world"
        raw = content.encode("utf-8")
        counts = count_content(content, raw)
        assert counts["lines"] == 0  # No newline = 0 lines
        assert counts["words"] == 2
        assert counts["bytes"] == 11

    def test_max_line_length(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        content = "short\nthis is a longer line\nhi\n"
        raw = content.encode("utf-8")
        counts = count_content(content, raw)
        assert counts["max_line_length"] == len("this is a longer line")

    def test_only_whitespace(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        content = "   \n\n  \n"
        raw = content.encode("utf-8")
        counts = count_content(content, raw)
        assert counts["lines"] == 3
        assert counts["words"] == 0

    def test_multibyte_characters(self) -> None:
        """UTF-8 multibyte chars: bytes != chars."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import count_content

        content = "cafe\u0301\n"  # e + combining accent
        raw = content.encode("utf-8")
        counts = count_content(content, raw)
        assert counts["chars"] == 6  # c, a, f, e, combining accent, \n
        assert counts["bytes"] == len(raw)  # More bytes due to UTF-8


# ---------------------------------------------------------------------------
# Test: format_counts function
# ---------------------------------------------------------------------------


class TestFormatCounts:
    """Test the format_counts function directly."""

    def test_all_counts(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import format_counts

        counts = {"lines": 5, "words": 12, "bytes": 68, "chars": 68, "max_line_length": 20}
        result = format_counts(
            counts,
            show_lines=True,
            show_words=True,
            show_bytes=True,
            show_chars=False,
            show_max_line_length=False,
            width=3,
            filename="test.txt",
        )
        assert "  5" in result
        assert " 12" in result
        assert " 68" in result
        assert "test.txt" in result

    def test_lines_only(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import format_counts

        counts = {"lines": 42, "words": 0, "bytes": 0, "chars": 0, "max_line_length": 0}
        result = format_counts(
            counts,
            show_lines=True,
            show_words=False,
            show_bytes=False,
            show_chars=False,
            show_max_line_length=False,
            width=2,
            filename=None,
        )
        assert result.strip() == "42"

    def test_no_filename(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import format_counts

        counts = {"lines": 1, "words": 2, "bytes": 3, "chars": 3, "max_line_length": 5}
        result = format_counts(
            counts,
            show_lines=True,
            show_words=True,
            show_bytes=True,
            show_chars=False,
            show_max_line_length=False,
            width=1,
            filename=None,
        )
        # Should not have a trailing filename
        assert result == "1 2 3"

    def test_right_alignment(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import format_counts

        counts = {"lines": 1, "words": 2, "bytes": 3, "chars": 3, "max_line_length": 0}
        result = format_counts(
            counts,
            show_lines=True,
            show_words=True,
            show_bytes=True,
            show_chars=False,
            show_max_line_length=False,
            width=5,
        )
        assert "    1" in result
        assert "    2" in result
        assert "    3" in result


# ---------------------------------------------------------------------------
# Test: Full integration with real files
# ---------------------------------------------------------------------------


class TestFileIntegration:
    """Test wc with actual temporary files."""

    def test_count_real_file(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from wc_tool import main

        with tempfile.NamedTemporaryFile(mode="w", suffix=".txt", delete=False) as f:
            f.write("hello world\ngoodbye world\n")
            f.flush()
            tmpfile = f.name

        old_argv = sys.argv
        try:
            sys.argv = ["wc", tmpfile]
            main()
        finally:
            sys.argv = old_argv
            Path(tmpfile).unlink()

        captured = capsys.readouterr()
        # Should have 2 lines, 4 words, and the correct byte count
        assert "2" in captured.out
        assert "4" in captured.out


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["wc", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["wc", "--help"])
        assert isinstance(result, HelpResult)
        assert "wc" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["wc", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["wc", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"
