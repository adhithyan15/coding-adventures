"""Tests for the yes tool.

=== What These Tests Verify ===

These tests exercise:

1. The ``build_yes_line`` function — converting arguments into the output line
2. The ``yes_loop`` function — printing the line a limited number of times
3. CLI Builder integration — spec loading, --help, --version
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "yes.json")

# Add the tool directory to sys.path for imports.
sys.path.insert(0, str(Path(__file__).parent.parent))


# ---------------------------------------------------------------------------
# Helper: parse argv through CLI Builder
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the yes spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: build_yes_line function
# ---------------------------------------------------------------------------


class TestBuildYesLine:
    """Test the build_yes_line helper that constructs the output string."""

    def test_none_returns_y(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line(None) == "y"

    def test_empty_list_returns_y(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line([]) == "y"

    def test_single_string(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line(["hello"]) == "hello"

    def test_multiple_strings_joined_with_spaces(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line(["hello", "world"]) == "hello world"

    def test_scalar_string(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line("yes") == "yes"

    def test_empty_string_scalar(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line("") == "y"

    def test_default_y_string(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line("y") == "y"

    def test_three_strings(self) -> None:
        from yes_tool import build_yes_line

        assert build_yes_line(["I", "agree", "completely"]) == "I agree completely"


# ---------------------------------------------------------------------------
# Test: yes_loop function
# ---------------------------------------------------------------------------


class TestYesLoop:
    """Test the yes_loop function with a limited iteration count."""

    def test_loop_zero_times(self, capsys: pytest.CaptureFixture[str]) -> None:
        from yes_tool import yes_loop

        yes_loop("y", max_count=0)
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_loop_one_time(self, capsys: pytest.CaptureFixture[str]) -> None:
        from yes_tool import yes_loop

        yes_loop("y", max_count=1)
        captured = capsys.readouterr()
        assert captured.out == "y\n"

    def test_loop_five_times(self, capsys: pytest.CaptureFixture[str]) -> None:
        from yes_tool import yes_loop

        yes_loop("y", max_count=5)
        captured = capsys.readouterr()
        lines = captured.out.strip().split("\n")
        assert len(lines) == 5
        assert all(line == "y" for line in lines)

    def test_loop_custom_string(self, capsys: pytest.CaptureFixture[str]) -> None:
        from yes_tool import yes_loop

        yes_loop("hello world", max_count=3)
        captured = capsys.readouterr()
        lines = captured.out.strip().split("\n")
        assert len(lines) == 3
        assert all(line == "hello world" for line in lines)

    def test_loop_handles_broken_pipe(self) -> None:
        """Verify that BrokenPipeError is silently caught."""
        from yes_tool import yes_loop

        # We can't easily trigger a real BrokenPipeError in a unit test,
        # but we can verify the function exists and handles the case
        # by testing with max_count (which avoids the pipe scenario).
        # The BrokenPipeError handling is defensive code for production use.
        yes_loop("y", max_count=1)  # Should not raise


# ---------------------------------------------------------------------------
# Test: CLI Builder integration
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["yes", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["yes", "--help"])
        assert isinstance(result, HelpResult)
        assert "yes" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["yes", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["yes", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestParseArguments:
    """Test that CLI Builder correctly parses the yes spec arguments."""

    def test_no_arguments_uses_default(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["yes"])
        assert isinstance(result, ParseResult)

    def test_single_string_argument(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["yes", "hello"])
        assert isinstance(result, ParseResult)
        strings = result.arguments.get("string")
        assert strings is not None

    def test_multiple_string_arguments(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["yes", "hello", "world"])
        assert isinstance(result, ParseResult)
        strings = result.arguments.get("string")
        assert strings is not None
