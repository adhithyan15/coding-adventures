"""Tests for the logname tool.

=== What These Tests Verify ===

These tests exercise:

1. The ``get_login_name`` function — retrieving the login user name
2. Error handling when no login name is available
3. CLI Builder integration — spec loading, --help, --version
4. The main function — end-to-end behavior including error paths
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "logname.json")

# Add the tool directory to sys.path for imports.
sys.path.insert(0, str(Path(__file__).parent.parent))


# ---------------------------------------------------------------------------
# Helper: parse argv through CLI Builder
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the logname spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: get_login_name function
# ---------------------------------------------------------------------------


class TestGetLoginName:
    """Test the get_login_name business logic."""

    def test_returns_string(self) -> None:
        from logname_tool import get_login_name

        # os.getlogin() may fail in CI — handle gracefully.
        try:
            result = get_login_name()
            assert isinstance(result, str)
            assert len(result) > 0
        except OSError:
            # Expected in environments without a controlling terminal.
            pass

    def test_matches_os_getlogin(self) -> None:
        """The result should match os.getlogin() when available."""
        from logname_tool import get_login_name

        try:
            expected = os.getlogin()
            assert get_login_name() == expected
        except OSError:
            pass

    def test_raises_oserror_when_no_terminal(self) -> None:
        """When os.getlogin() fails, get_login_name should propagate the error."""
        from logname_tool import get_login_name

        with (
            patch("logname_tool.os.getlogin", side_effect=OSError("no login")),
            pytest.raises(OSError),
        ):
            get_login_name()


# ---------------------------------------------------------------------------
# Test: main function
# ---------------------------------------------------------------------------


class TestMain:
    """Test the main function end-to-end."""

    def test_main_success_prints_name(self, capsys: pytest.CaptureFixture[str]) -> None:
        from logname_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["logname"]
            with patch("logname_tool.get_login_name", return_value="alice"):
                main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out.strip() == "alice"

    def test_main_failure_prints_err(self, capsys: pytest.CaptureFixture[str]) -> None:
        from logname_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["logname"]
            with patch("logname_tool.get_login_name", side_effect=OSError("no login")):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 1
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert "no login name" in captured.err

    def test_main_failure_exit_code(self) -> None:
        from logname_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["logname"]
            with patch("logname_tool.get_login_name", side_effect=OSError("no login")):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 1
        finally:
            sys.argv = old_argv


# ---------------------------------------------------------------------------
# Test: CLI Builder integration
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["logname", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["logname", "--help"])
        assert isinstance(result, HelpResult)
        assert "logname" in result.text

    def test_help_text_contains_description(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["logname", "--help"])
        assert isinstance(result, HelpResult)
        assert "login" in result.text.lower()


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["logname", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["logname", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestUnknownFlags:
    """Unknown flags should raise ParseErrors."""

    def test_unknown_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["logname", "--unknown"])
