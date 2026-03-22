"""Tests for the true tool.

=== What These Tests Verify ===

The ``true`` utility is the simplest possible program: it does nothing and
exits 0. But even a no-op needs tests! We verify:

1. CLI Builder integration: does the JSON spec parse correctly?
2. Help and version output: do --help and --version work?
3. Exit behavior: does the program exit with status 0?

=== Why Test Something So Simple? ===

Testing ``true`` might seem silly, but it validates that:
- The JSON spec file is well-formed and CLI Builder can load it.
- The builtins (--help, --version) are properly configured.
- The exit code is correct (a subtle bug here would be hard to find).
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "true.json")


# ---------------------------------------------------------------------------
# Helper: import cli_builder and parse argv
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the true spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Default behavior (no flags) returns ParseResult
# ---------------------------------------------------------------------------


class TestDefaultBehavior:
    """When invoked with no flags, true should return a ParseResult.
    The program should then exit 0."""

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["true"])
        assert isinstance(result, ParseResult)

    def test_no_flags_no_flags_set(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["true"])
        assert isinstance(result, ParseResult)
        # true has no custom flags, so the flags dict should be empty
        # or contain only defaults.
        assert len(result.flags) == 0

    def test_no_arguments_expected(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["true"])
        assert isinstance(result, ParseResult)
        assert len(result.arguments) == 0


# ---------------------------------------------------------------------------
# Test: --help flag
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult with non-empty text."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["true", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["true", "--help"])
        assert isinstance(result, HelpResult)
        assert "true" in result.text

    def test_help_text_contains_description(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["true", "--help"])
        assert isinstance(result, HelpResult)
        assert "nothing" in result.text.lower() or "success" in result.text.lower()


# ---------------------------------------------------------------------------
# Test: --version flag
# ---------------------------------------------------------------------------


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["true", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["true", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Exit behavior
# ---------------------------------------------------------------------------


class TestExitBehavior:
    """The main function should exit with code 0."""

    def test_main_exits_zero(self) -> None:
        """Calling main() with no arguments should raise SystemExit(0)."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from true_tool import main

        # Override sys.argv to simulate a bare invocation.
        old_argv = sys.argv
        try:
            sys.argv = ["true"]
            with pytest.raises(SystemExit) as exc_info:
                main()
            assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv

    def test_main_help_exits_zero(self) -> None:
        """Calling main() with --help should also exit 0."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from true_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["true", "--help"]
            with pytest.raises(SystemExit) as exc_info:
                main()
            assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv
