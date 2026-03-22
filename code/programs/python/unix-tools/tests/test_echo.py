"""Tests for the echo tool.

=== What These Tests Verify ===

These tests exercise the full echo implementation, including:

1. Basic string output (joining arguments with spaces)
2. The -n flag (suppress trailing newline)
3. The -e flag (interpret backslash escapes)
4. The -E flag (disable backslash escapes, the default)
5. CLI Builder integration (--help, --version, error handling)
6. Edge cases (empty input, special characters, octal escapes)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "echo.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder and parse argv
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the echo spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Default behavior (no flags)
# ---------------------------------------------------------------------------


class TestDefaultBehavior:
    """When invoked with arguments and no flags, echo should join them
    with spaces and print with a trailing newline."""

    def test_single_argument(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["echo", "hello"])
        assert isinstance(result, ParseResult)
        strings = result.arguments.get("strings", [])
        assert "hello" in (strings if isinstance(strings, list) else [strings])

    def test_multiple_arguments(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["echo", "hello", "world"])
        assert isinstance(result, ParseResult)
        strings = result.arguments.get("strings", [])
        assert isinstance(strings, list)
        assert strings == ["hello", "world"]

    def test_no_arguments_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["echo"])
        assert isinstance(result, ParseResult)

    def test_escapes_disabled_by_default(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["echo", "hello"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("enable_escapes") is not True


# ---------------------------------------------------------------------------
# Test: -n flag (suppress trailing newline)
# ---------------------------------------------------------------------------


class TestNoNewlineFlag:
    """The ``-n`` flag should suppress the trailing newline."""

    def test_n_flag_is_set(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["echo", "-n", "hello"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("no_newline") is True

    def test_main_n_flag_output(self, capsys: pytest.CaptureFixture[str]) -> None:
        """With -n, output should not end with a newline."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["echo", "-n", "hello"]
            main()
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out == "hello"  # No trailing newline


# ---------------------------------------------------------------------------
# Test: -e flag (enable backslash escapes)
# ---------------------------------------------------------------------------


class TestEnableEscapes:
    """The ``-e`` flag enables interpretation of backslash escapes."""

    def test_e_flag_is_set(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["echo", "-e", "hello"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("enable_escapes") is True

    def test_newline_escape(self, capsys: pytest.CaptureFixture[str]) -> None:
        """\\n should be converted to an actual newline."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["echo", "-e", "hello\\nworld"]
            main()
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out == "hello\nworld\n"

    def test_tab_escape(self, capsys: pytest.CaptureFixture[str]) -> None:
        """\\t should be converted to a tab."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["echo", "-e", "hello\\tworld"]
            main()
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out == "hello\tworld\n"

    def test_backslash_escape(self, capsys: pytest.CaptureFixture[str]) -> None:
        """\\\\  should be converted to a single backslash."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["echo", "-e", "back\\\\slash"]
            main()
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out == "back\\slash\n"


# ---------------------------------------------------------------------------
# Test: Backslash escape processing function
# ---------------------------------------------------------------------------


class TestProcessEscapes:
    """Test the process_escapes function directly for edge cases."""

    def test_no_escapes(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        assert process_escapes("hello world") == "hello world"

    def test_alert_escape(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        assert process_escapes("\\a") == "\a"

    def test_backspace_escape(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        assert process_escapes("\\b") == "\b"

    def test_form_feed_escape(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        assert process_escapes("\\f") == "\f"

    def test_carriage_return_escape(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        assert process_escapes("\\r") == "\r"

    def test_octal_escape(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        # \0101 is octal for 65 = 'A'
        assert process_escapes("\\0101") == "A"

    def test_octal_nul(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        # \0 with no digits = NUL character
        assert process_escapes("\\0") == "\x00"

    def test_unknown_escape_passthrough(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        # Unknown escapes pass through unchanged
        assert process_escapes("\\z") == "\\z"

    def test_trailing_backslash(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        # A trailing backslash is output literally
        assert process_escapes("hello\\") == "hello\\"

    def test_multiple_escapes(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from echo_tool import process_escapes

        assert process_escapes("a\\nb\\tc") == "a\nb\tc"


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["echo", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["echo", "--help"])
        assert isinstance(result, HelpResult)
        assert "echo" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["echo", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["echo", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"
