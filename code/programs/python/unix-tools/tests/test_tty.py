"""Tests for the tty tool.

=== What These Tests Verify ===

These tests exercise:

1. The ``get_tty_name`` function — detecting whether stdin is a terminal
2. The silent flag behavior — exit status without output
3. CLI Builder integration — spec loading, --help, --version, -s flag
4. The main function — end-to-end behavior for both tty and non-tty cases
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "tty.json")

# Add the tool directory to sys.path for imports.
sys.path.insert(0, str(Path(__file__).parent.parent))


# ---------------------------------------------------------------------------
# Helper: parse argv through CLI Builder
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the tty spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: get_tty_name function
# ---------------------------------------------------------------------------


class TestGetTtyName:
    """Test the get_tty_name business logic."""

    def test_returns_string_or_none(self) -> None:
        from tty_tool import get_tty_name

        result = get_tty_name()
        assert result is None or isinstance(result, str)

    def test_returns_none_when_not_a_tty(self) -> None:
        """When os.ttyname raises OSError, get_tty_name returns None."""
        from tty_tool import get_tty_name

        with patch("tty_tool.os.ttyname", side_effect=OSError("not a tty")):
            assert get_tty_name() is None

    def test_returns_path_when_tty(self) -> None:
        """When os.ttyname succeeds, get_tty_name returns the path."""
        from tty_tool import get_tty_name

        with patch("tty_tool.os.ttyname", return_value="/dev/ttys003"):
            assert get_tty_name() == "/dev/ttys003"

    def test_returns_absolute_path_when_tty(self) -> None:
        """The returned path should be absolute (starts with /)."""
        from tty_tool import get_tty_name

        with patch("tty_tool.os.ttyname", return_value="/dev/pts/0"):
            result = get_tty_name()
            assert result is not None
            assert result.startswith("/")


# ---------------------------------------------------------------------------
# Test: main function — tty connected
# ---------------------------------------------------------------------------


class TestMainWithTty:
    """Test main() when stdin IS a terminal."""

    def test_prints_tty_name(self, capsys: pytest.CaptureFixture[str]) -> None:
        from tty_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["tty"]
            with patch("tty_tool.get_tty_name", return_value="/dev/ttys003"):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out.strip() == "/dev/ttys003"

    def test_silent_prints_nothing(self, capsys: pytest.CaptureFixture[str]) -> None:
        from tty_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["tty", "-s"]
            with patch("tty_tool.get_tty_name", return_value="/dev/ttys003"):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_exit_code_zero(self) -> None:
        from tty_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["tty"]
            with patch("tty_tool.get_tty_name", return_value="/dev/ttys003"):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 0
        finally:
            sys.argv = old_argv


# ---------------------------------------------------------------------------
# Test: main function — no tty
# ---------------------------------------------------------------------------


class TestMainWithoutTty:
    """Test main() when stdin is NOT a terminal."""

    def test_prints_not_a_tty(self, capsys: pytest.CaptureFixture[str]) -> None:
        from tty_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["tty"]
            with patch("tty_tool.get_tty_name", return_value=None):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 1
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out.strip() == "not a tty"

    def test_silent_prints_nothing(self, capsys: pytest.CaptureFixture[str]) -> None:
        from tty_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["tty", "-s"]
            with patch("tty_tool.get_tty_name", return_value=None):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 1
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_exit_code_one(self) -> None:
        from tty_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["tty"]
            with patch("tty_tool.get_tty_name", return_value=None):
                with pytest.raises(SystemExit) as exc_info:
                    main()
                assert exc_info.value.code == 1
        finally:
            sys.argv = old_argv


# ---------------------------------------------------------------------------
# Test: CLI Builder integration
# ---------------------------------------------------------------------------


class TestSilentFlag:
    """The ``-s``/``--silent`` flag should be recognized."""

    def test_short_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tty", "-s"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("silent") is True

    def test_long_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tty", "--silent"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("silent") is True

    def test_no_flag_silent_is_false(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tty"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("silent") is not True


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tty", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tty", "--help"])
        assert isinstance(result, HelpResult)
        assert "tty" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tty", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tty", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestUnknownFlags:
    """Unknown flags should raise ParseErrors."""

    def test_unknown_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["tty", "--unknown"])
