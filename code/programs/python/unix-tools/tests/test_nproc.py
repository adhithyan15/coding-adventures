"""Tests for the nproc tool.

=== What These Tests Verify ===

These tests exercise:

1. The ``get_available_cpus`` function — counting available processors
2. The ``get_installed_cpus`` function — counting total processors
3. The ``apply_ignore`` function — subtracting with a floor of 1
4. CLI Builder integration — spec loading, --help, --version, --all, --ignore
5. The main function — end-to-end output verification
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "nproc.json")

# Add the tool directory to sys.path for imports.
sys.path.insert(0, str(Path(__file__).parent.parent))


# ---------------------------------------------------------------------------
# Helper: parse argv through CLI Builder
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the nproc spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: apply_ignore function
# ---------------------------------------------------------------------------


class TestApplyIgnore:
    """Test the apply_ignore helper — subtracts N with a floor of 1."""

    def test_no_subtraction(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(8, 0) == 8

    def test_subtract_two(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(8, 2) == 6

    def test_subtract_all(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(4, 4) == 1  # Floor is 1, not 0

    def test_subtract_more_than_available(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(2, 5) == 1  # Can't go below 1

    def test_single_cpu_subtract_zero(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(1, 0) == 1

    def test_single_cpu_subtract_one(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(1, 1) == 1  # Floor is 1

    def test_large_numbers(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(128, 64) == 64

    def test_subtract_one(self) -> None:
        from nproc_tool import apply_ignore

        assert apply_ignore(4, 1) == 3


# ---------------------------------------------------------------------------
# Test: get_available_cpus function
# ---------------------------------------------------------------------------


class TestGetAvailableCpus:
    """Test the get_available_cpus function."""

    def test_returns_positive_integer(self) -> None:
        from nproc_tool import get_available_cpus

        result = get_available_cpus()
        assert isinstance(result, int)
        assert result >= 1

    def test_fallback_when_cpu_count_none(self) -> None:
        """When os.cpu_count() returns None, should return 1."""
        from nproc_tool import get_available_cpus

        # Only test fallback on platforms without sched_getaffinity
        if not hasattr(os, "sched_getaffinity"):
            with patch("nproc_tool.os.cpu_count", return_value=None):
                assert get_available_cpus() == 1

    def test_uses_cpu_count_on_macos(self) -> None:
        """On macOS (no sched_getaffinity), falls back to os.cpu_count()."""
        from nproc_tool import get_available_cpus

        if not hasattr(os, "sched_getaffinity"):
            with patch("nproc_tool.os.cpu_count", return_value=4):
                assert get_available_cpus() == 4


# ---------------------------------------------------------------------------
# Test: get_installed_cpus function
# ---------------------------------------------------------------------------


class TestGetInstalledCpus:
    """Test the get_installed_cpus function."""

    def test_returns_positive_integer(self) -> None:
        from nproc_tool import get_installed_cpus

        result = get_installed_cpus()
        assert isinstance(result, int)
        assert result >= 1

    def test_fallback_when_none(self) -> None:
        from nproc_tool import get_installed_cpus

        with patch("nproc_tool.os.cpu_count", return_value=None):
            assert get_installed_cpus() == 1

    def test_returns_count(self) -> None:
        from nproc_tool import get_installed_cpus

        with patch("nproc_tool.os.cpu_count", return_value=16):
            assert get_installed_cpus() == 16


# ---------------------------------------------------------------------------
# Test: main function
# ---------------------------------------------------------------------------


class TestMain:
    """Test the main function end-to-end."""

    def test_default_prints_number(self, capsys: pytest.CaptureFixture[str]) -> None:
        from nproc_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["nproc"]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        count = int(captured.out.strip())
        assert count >= 1

    def test_all_flag(self, capsys: pytest.CaptureFixture[str]) -> None:
        from nproc_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["nproc", "--all"]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        count = int(captured.out.strip())
        assert count >= 1

    def test_ignore_flag(self, capsys: pytest.CaptureFixture[str]) -> None:
        from nproc_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["nproc", "--ignore", "1"]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        count = int(captured.out.strip())
        assert count >= 1


# ---------------------------------------------------------------------------
# Test: CLI Builder integration
# ---------------------------------------------------------------------------


class TestAllFlag:
    """The ``--all`` flag should be recognized."""

    def test_all_flag_is_set(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nproc", "--all"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("all") is True

    def test_no_flags_all_is_false(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nproc"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("all") is not True


class TestIgnoreFlag:
    """The ``--ignore`` flag should accept an integer value."""

    def test_ignore_flag_value(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nproc", "--ignore", "2"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("ignore") == 2

    def test_ignore_flag_zero(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["nproc", "--ignore", "0"])
        assert isinstance(result, ParseResult)
        assert result.flags.get("ignore") == 0


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["nproc", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["nproc", "--help"])
        assert isinstance(result, HelpResult)
        assert "nproc" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult with "1.0.0"."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["nproc", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["nproc", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"
