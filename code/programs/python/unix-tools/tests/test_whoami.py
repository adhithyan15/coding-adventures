"""Tests for the whoami tool.

=== What These Tests Verify ===

These tests exercise:

1. The ``get_effective_username`` function — retrieving the current username
2. CLI Builder integration — spec loading, --help, --version
3. The main function — end-to-end output verification
"""

from __future__ import annotations

import getpass
import sys
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "whoami.json")

# Add the tool directory to sys.path for imports.
sys.path.insert(0, str(Path(__file__).parent.parent))


# ---------------------------------------------------------------------------
# Helper: parse argv through CLI Builder
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the whoami spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: get_effective_username function
# ---------------------------------------------------------------------------


class TestGetEffectiveUsername:
    """Test the get_effective_username business logic."""

    def test_returns_string(self) -> None:
        from whoami_tool import get_effective_username

        result = get_effective_username()
        assert isinstance(result, str)

    def test_returns_nonempty(self) -> None:
        from whoami_tool import get_effective_username

        result = get_effective_username()
        assert len(result) > 0

    def test_matches_getpass(self) -> None:
        """The result should match what getpass.getuser() returns."""
        from whoami_tool import get_effective_username

        assert get_effective_username() == getpass.getuser()

    def test_with_mocked_user(self) -> None:
        """Verify we correctly use getpass.getuser()."""
        from whoami_tool import get_effective_username

        with patch("whoami_tool.getpass.getuser", return_value="testuser"):
            assert get_effective_username() == "testuser"


# ---------------------------------------------------------------------------
# Test: main function
# ---------------------------------------------------------------------------


class TestMain:
    """Test the main function end-to-end."""

    def test_main_prints_username(self, capsys: pytest.CaptureFixture[str]) -> None:
        from whoami_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["whoami"]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert len(captured.out.strip()) > 0

    def test_main_output_matches(self, capsys: pytest.CaptureFixture[str]) -> None:
        from whoami_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["whoami"]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out.strip() == getpass.getuser()


# ---------------------------------------------------------------------------
# Test: CLI Builder integration
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["whoami", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["whoami", "--help"])
        assert isinstance(result, HelpResult)
        assert "whoami" in result.text

    def test_help_text_contains_description(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["whoami", "--help"])
        assert isinstance(result, HelpResult)
        assert "user" in result.text.lower()


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["whoami", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["whoami", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestUnknownFlags:
    """Unknown flags should raise ParseErrors."""

    def test_unknown_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["whoami", "--unknown"])

    def test_unknown_short_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["whoami", "-x"])
