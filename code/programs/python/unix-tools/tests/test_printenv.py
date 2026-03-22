"""Tests for the printenv tool.

=== What These Tests Verify ===

These tests exercise the printenv implementation, including:

1. Spec loading and default behavior
2. Printing specific variables
3. The -0 flag (NUL terminator)
4. Exit status for missing variables
5. CLI Builder integration (--help, --version)
6. Business logic functions (print_all_env, print_specific_vars)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "printenv.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the printenv spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the printenv.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_no_args_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["printenv"])
        assert isinstance(result, ParseResult)

    def test_with_variable_argument(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["printenv", "HOME"])
        assert isinstance(result, ParseResult)
        variables = result.arguments.get("variables", [])
        if isinstance(variables, str):
            assert variables == "HOME"
        else:
            assert "HOME" in variables


# ---------------------------------------------------------------------------
# Test: -0 flag (null terminator)
# ---------------------------------------------------------------------------


class TestNullFlag:
    """The ``-0`` flag changes the terminator to NUL."""

    def test_null_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["printenv", "-0"])
        assert isinstance(result, ParseResult)
        assert result.flags["null"] is True

    def test_null_flag_long(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["printenv", "--null"])
        assert isinstance(result, ParseResult)
        assert result.flags["null"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["printenv", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["printenv", "--help"])
        assert isinstance(result, HelpResult)
        assert "printenv" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["printenv", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["printenv", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — print_all_env
# ---------------------------------------------------------------------------


class TestPrintAllEnv:
    """Test printing all environment variables."""

    def test_prints_home(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from printenv_tool import print_all_env

        print_all_env(terminator="\n")
        captured = capsys.readouterr()
        # HOME should appear in the output as NAME=VALUE.
        assert "HOME=" in captured.out

    def test_prints_path(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from printenv_tool import print_all_env

        print_all_env(terminator="\n")
        captured = capsys.readouterr()
        assert "PATH=" in captured.out


# ---------------------------------------------------------------------------
# Test: Business logic — print_specific_vars
# ---------------------------------------------------------------------------


class TestPrintSpecificVars:
    """Test printing specific environment variables."""

    def test_existing_variable(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from printenv_tool import print_specific_vars

        # Set a known variable for testing.
        os.environ["TEST_PRINTENV_VAR"] = "test_value_42"
        try:
            exit_code = print_specific_vars(["TEST_PRINTENV_VAR"], terminator="\n")
            captured = capsys.readouterr()
            assert captured.out == "test_value_42\n"
            assert exit_code == 0
        finally:
            del os.environ["TEST_PRINTENV_VAR"]

    def test_missing_variable(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from printenv_tool import print_specific_vars

        exit_code = print_specific_vars(["DEFINITELY_NOT_SET_12345"], terminator="\n")
        captured = capsys.readouterr()
        assert captured.out == ""
        assert exit_code == 1

    def test_mixed_existing_and_missing(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from printenv_tool import print_specific_vars

        os.environ["TEST_PRINTENV_EXISTS"] = "found"
        try:
            exit_code = print_specific_vars(
                ["TEST_PRINTENV_EXISTS", "DOES_NOT_EXIST_99"],
                terminator="\n",
            )
            captured = capsys.readouterr()
            assert "found" in captured.out
            assert exit_code == 1  # One variable was missing.
        finally:
            del os.environ["TEST_PRINTENV_EXISTS"]

    def test_null_terminator(self, capsys: pytest.CaptureFixture[str]) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from printenv_tool import print_specific_vars

        os.environ["TEST_NULL_TERM"] = "value"
        try:
            exit_code = print_specific_vars(["TEST_NULL_TERM"], terminator="\0")
            captured = capsys.readouterr()
            assert captured.out == "value\0"
            assert exit_code == 0
        finally:
            del os.environ["TEST_NULL_TERM"]
