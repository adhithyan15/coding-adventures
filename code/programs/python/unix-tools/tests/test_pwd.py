"""Tests for the pwd tool.

=== What These Tests Verify ===

These tests exercise the full CLI Builder integration. We construct a
``Parser`` with our ``pwd.json`` spec and various argv values, then
verify that the parser returns the correct result type and that the
business logic produces the expected output.

=== Why We Test Through CLI Builder ===

The point of CLI Builder is that developers don't write parsing code.
So our tests verify the *integration*: does our JSON spec, combined with
CLI Builder's parser, produce the right behavior? This catches spec
errors (wrong flag names, missing fields) as well as logic errors.
"""

from __future__ import annotations

import os
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "pwd.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder (installed via BUILD file)
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the pwd spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Default behavior (no flags) returns ParseResult
# ---------------------------------------------------------------------------


class TestDefaultBehavior:
    """When invoked with no flags, pwd should return a ParseResult
    with ``physical`` set to ``False`` (the default)."""

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd"])
        assert isinstance(result, ParseResult)

    def test_no_flags_physical_is_false(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("physical") is not True

    def test_no_flags_logical_is_false(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd"])
        assert isinstance(result, ParseResult)
        # When neither flag is given, both default to False.
        assert result.flags.get("logical") is not True


# ---------------------------------------------------------------------------
# Test: -P flag
# ---------------------------------------------------------------------------


class TestPhysicalFlag:
    """The ``-P`` flag should set ``physical`` to ``True``."""

    def test_short_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd", "-P"])
        assert isinstance(result, ParseResult)
        assert result.flags["physical"] is True

    def test_long_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd", "--physical"])
        assert isinstance(result, ParseResult)
        assert result.flags["physical"] is True


# ---------------------------------------------------------------------------
# Test: -L flag
# ---------------------------------------------------------------------------


class TestLogicalFlag:
    """The ``-L`` flag should set ``logical`` to ``True``."""

    def test_short_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd", "-L"])
        assert isinstance(result, ParseResult)
        assert result.flags["logical"] is True

    def test_long_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["pwd", "--logical"])
        assert isinstance(result, ParseResult)
        assert result.flags["logical"] is True


# ---------------------------------------------------------------------------
# Test: --help flag
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult with non-empty text."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["pwd", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["pwd", "--help"])
        assert isinstance(result, HelpResult)
        assert "pwd" in result.text

    def test_help_text_contains_description(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["pwd", "--help"])
        assert isinstance(result, HelpResult)
        assert "working directory" in result.text.lower()


# ---------------------------------------------------------------------------
# Test: --version flag
# ---------------------------------------------------------------------------


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["pwd", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["pwd", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Unknown flags produce errors
# ---------------------------------------------------------------------------


class TestUnknownFlags:
    """Unknown flags should raise ParseErrors."""

    def test_unknown_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["pwd", "--unknown"])

    def test_unknown_short_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["pwd", "-x"])


# ---------------------------------------------------------------------------
# Test: Business logic functions
# ---------------------------------------------------------------------------


class TestBusinessLogic:
    """Test the pwd business logic functions directly."""

    def test_get_physical_pwd_returns_string(self) -> None:
        # Import from the main module
        import sys

        sys.path.insert(0, str(Path(__file__).parent.parent))
        from pwd_tool import get_physical_pwd

        result = get_physical_pwd()
        assert isinstance(result, str)
        assert os.path.isabs(result)

    def test_get_logical_pwd_returns_string(self) -> None:
        import sys

        sys.path.insert(0, str(Path(__file__).parent.parent))
        from pwd_tool import get_logical_pwd

        result = get_logical_pwd()
        assert isinstance(result, str)
        assert os.path.isabs(result)

    def test_logical_pwd_uses_env_when_valid(self) -> None:
        """When $PWD matches the real cwd, get_logical_pwd should return it."""
        import sys

        sys.path.insert(0, str(Path(__file__).parent.parent))
        from pwd_tool import get_logical_pwd

        # Set PWD to the real cwd — get_logical_pwd should return it.
        real = os.path.realpath(".")
        old_pwd = os.environ.get("PWD")
        try:
            os.environ["PWD"] = real
            result = get_logical_pwd()
            assert result == real
        finally:
            if old_pwd is not None:
                os.environ["PWD"] = old_pwd
            elif "PWD" in os.environ:
                del os.environ["PWD"]
