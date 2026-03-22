"""Tests for the rmdir tool.

=== What These Tests Verify ===

These tests exercise the rmdir implementation, including:

1. Spec loading and CLI Builder integration
2. The -p flag (remove parents)
3. The --ignore-fail-on-non-empty flag
4. The -v flag (verbose output)
5. Business logic functions (remove_directory, remove_with_parents)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "rmdir.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from rmdir_tool import remove_directory, remove_with_parents


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rmdir", "testdir"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Test: Flags
# ---------------------------------------------------------------------------


class TestFlags:
    def test_parents_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rmdir", "-p", "a/b/c"])
        assert isinstance(result, ParseResult)
        assert result.flags["parents"] is True

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rmdir", "-v", "testdir"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True

    def test_ignore_non_empty_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rmdir", "--ignore-fail-on-non-empty", "testdir"])
        assert isinstance(result, ParseResult)
        assert result.flags["ignore_fail_on_non_empty"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["rmdir", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["rmdir", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — remove_directory
# ---------------------------------------------------------------------------


class TestRemoveDirectory:
    def test_remove_empty_directory(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "empty")
        os.mkdir(dirpath)
        result = remove_directory(dirpath, verbose=False, ignore_non_empty=False)
        assert result is True
        assert not os.path.exists(dirpath)

    def test_remove_nonexistent_fails(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "nonexistent")
        result = remove_directory(dirpath, verbose=False, ignore_non_empty=False)
        assert result is False

    def test_remove_nonempty_fails(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "nonempty")
        os.mkdir(dirpath)
        (Path(dirpath) / "file.txt").write_text("content")
        result = remove_directory(dirpath, verbose=False, ignore_non_empty=False)
        assert result is False

    def test_remove_nonempty_ignored(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "nonempty")
        os.mkdir(dirpath)
        (Path(dirpath) / "file.txt").write_text("content")
        result = remove_directory(dirpath, verbose=False, ignore_non_empty=True)
        assert result is False  # Still fails, but no error message.

    def test_verbose_output(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        dirpath = str(tmp_path / "verbose")
        os.mkdir(dirpath)
        remove_directory(dirpath, verbose=True, ignore_non_empty=False)
        captured = capsys.readouterr()
        assert "removing directory" in captured.out


# ---------------------------------------------------------------------------
# Test: Business logic — remove_with_parents
# ---------------------------------------------------------------------------


class TestRemoveWithParents:
    def test_remove_chain(self, tmp_path: Path) -> None:
        chain = tmp_path / "a" / "b" / "c"
        chain.mkdir(parents=True)
        result = remove_with_parents(
            str(chain), verbose=False, ignore_non_empty=False
        )
        assert result is True
        assert not (tmp_path / "a").exists()

    def test_remove_chain_stops_at_nonempty(self, tmp_path: Path) -> None:
        chain = tmp_path / "a" / "b"
        chain.mkdir(parents=True)
        (tmp_path / "a" / "sibling.txt").write_text("data")
        result = remove_with_parents(
            str(chain), verbose=False, ignore_non_empty=False
        )
        # b is removed, but a fails because it contains sibling.txt.
        assert result is False
        assert not (tmp_path / "a" / "b").exists()
        assert (tmp_path / "a").exists()
