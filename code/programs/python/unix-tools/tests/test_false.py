"""Tests for the false tool.

=== What These Tests Verify ===

The ``false`` utility is the counterpart to ``true``: it does nothing and
exits 1 (failure). We verify:

1. CLI Builder integration: does the JSON spec parse correctly?
2. Help and version output: do --help and --version work (and exit 0)?
3. Exit behavior: does the bare program exit with status 1?

The key subtlety: ``--help`` and ``--version`` exit 0 even for ``false``,
because the *request* (show help) succeeded. Only a bare invocation of
``false`` exits 1.
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "false.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder and parse argv
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the false spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Default behavior (no flags) returns ParseResult
# ---------------------------------------------------------------------------


class TestDefaultBehavior:
    """When invoked with no flags, false should return a ParseResult.
    The program should then exit 1."""

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["false"])
        assert isinstance(result, ParseResult)

    def test_no_flags_no_flags_set(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["false"])
        assert isinstance(result, ParseResult)
        assert len(result.flags) == 0

    def test_no_arguments_expected(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["false"])
        assert isinstance(result, ParseResult)
        assert len(result.arguments) == 0


# ---------------------------------------------------------------------------
# Test: --help flag
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult with non-empty text."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["false", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["false", "--help"])
        assert isinstance(result, HelpResult)
        assert "false" in result.text

    def test_help_text_contains_description(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["false", "--help"])
        assert isinstance(result, HelpResult)
        assert "nothing" in result.text.lower() or "unsuccess" in result.text.lower()


# ---------------------------------------------------------------------------
# Test: --version flag
# ---------------------------------------------------------------------------


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["false", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["false", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Exit behavior
# ---------------------------------------------------------------------------


class TestExitBehavior:
    """The main function should exit with code 1 for bare invocation,
    but code 0 for --help and --version."""

    def test_main_exits_one(self) -> None:
        """Calling main() with no arguments should raise SystemExit(1)."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from false_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["false"]
            with pytest.raises(SystemExit) as exc_info:
                main()
            assert exc_info.value.code == 1
        finally:
            sys.argv = old_argv

    def test_main_help_exits_zero(self) -> None:
        """Calling main() with --help should exit 0 (the help request succeeded)."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from false_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["false", "--help"]
            with pytest.raises(SystemExit) as exc_info:
                main()
            assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv

    def test_main_version_exits_zero(self) -> None:
        """Calling main() with --version should exit 0."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from false_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["false", "--version"]
            with pytest.raises(SystemExit) as exc_info:
                main()
            assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv
