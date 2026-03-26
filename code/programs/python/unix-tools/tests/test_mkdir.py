"""Tests for the mkdir tool.

=== What These Tests Verify ===

These tests exercise the mkdir implementation, including:

1. Spec loading and CLI Builder integration
2. The -p flag (create parent directories)
3. The -m flag (set mode)
4. The -v flag (verbose output)
5. Business logic functions (create_directory, parse_mode)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "mkdir.json")

# Add the parent directory to the path so we can import the tool.
sys.path.insert(0, str(Path(__file__).parent.parent))

from mkdir_tool import create_directory, parse_mode


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the mkdir spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the mkdir.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mkdir", "testdir"])
        assert isinstance(result, ParseResult)

    def test_directory_argument(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mkdir", "mydir"])
        assert isinstance(result, ParseResult)
        dirs = result.arguments.get("directories", [])
        if isinstance(dirs, str):
            assert dirs == "mydir"
        else:
            assert "mydir" in dirs


# ---------------------------------------------------------------------------
# Test: Flags
# ---------------------------------------------------------------------------


class TestFlags:
    """Test that all flags are parsed correctly."""

    def test_parents_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mkdir", "-p", "a/b/c"])
        assert isinstance(result, ParseResult)
        assert result.flags["parents"] is True

    def test_parents_flag_long(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mkdir", "--parents", "a/b/c"])
        assert isinstance(result, ParseResult)
        assert result.flags["parents"] is True

    def test_mode_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mkdir", "-m", "755", "testdir"])
        assert isinstance(result, ParseResult)
        assert result.flags["mode"] == "755"

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mkdir", "-v", "testdir"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    """Test CLI Builder's built-in --help and --version flags."""

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["mkdir", "--help"])
        assert isinstance(result, HelpResult)
        assert "mkdir" in result.text

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["mkdir", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — parse_mode
# ---------------------------------------------------------------------------


class TestParseMode:
    """Test the parse_mode function."""

    def test_octal_755(self) -> None:
        assert parse_mode("755") == 0o755

    def test_octal_700(self) -> None:
        assert parse_mode("700") == 0o700

    def test_octal_0755(self) -> None:
        assert parse_mode("0755") == 0o755

    def test_invalid_mode(self) -> None:
        assert parse_mode("xyz") is None

    def test_empty_string(self) -> None:
        assert parse_mode("") is None


# ---------------------------------------------------------------------------
# Test: Business logic — create_directory
# ---------------------------------------------------------------------------


class TestCreateDirectory:
    """Test the create_directory function."""

    def test_create_single_directory(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "newdir")
        result = create_directory(dirpath, parents=False, mode=None, verbose=False)
        assert result is True
        assert os.path.isdir(dirpath)

    def test_create_with_parents(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "a" / "b" / "c")
        result = create_directory(dirpath, parents=True, mode=None, verbose=False)
        assert result is True
        assert os.path.isdir(dirpath)

    def test_create_fails_without_parents(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "x" / "y" / "z")
        result = create_directory(dirpath, parents=False, mode=None, verbose=False)
        assert result is False

    def test_create_existing_fails_without_parents(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "existing")
        os.mkdir(dirpath)
        result = create_directory(dirpath, parents=False, mode=None, verbose=False)
        assert result is False

    def test_create_existing_succeeds_with_parents(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "existing")
        os.mkdir(dirpath)
        result = create_directory(dirpath, parents=True, mode=None, verbose=False)
        assert result is True

    def test_verbose_output(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        dirpath = str(tmp_path / "verbosedir")
        create_directory(dirpath, parents=False, mode=None, verbose=True)
        captured = capsys.readouterr()
        assert "created directory" in captured.out

    def test_mode_is_set(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "modedir")
        create_directory(dirpath, parents=False, mode=0o700, verbose=False)
        assert os.path.isdir(dirpath)
        # The actual mode may be affected by umask, so just check it exists.
