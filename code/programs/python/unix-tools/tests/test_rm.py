"""Tests for the rm tool.

=== What These Tests Verify ===

These tests exercise the rm implementation, including:

1. Spec loading and CLI Builder integration
2. Removing files
3. The -r flag (recursive)
4. The -f flag (force)
5. The -v flag (verbose)
6. The -d flag (empty directories)
7. Safety checks (--preserve-root)
8. Business logic function (remove_file)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "rm.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from rm_tool import remove_file


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rm", "file.txt"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_force_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rm", "-f", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["force"] is True

    def test_recursive_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rm", "-r", "dir"])
        assert isinstance(result, ParseResult)
        assert result.flags["recursive"] is True

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["rm", "-v", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["rm", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["rm", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestRemoveFile:
    def test_remove_regular_file(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "file.txt")
        Path(filepath).write_text("data")
        result = remove_file(
            filepath, force=False, interactive=False, recursive=False,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is True
        assert not os.path.exists(filepath)

    def test_remove_nonexistent_fails(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "nonexistent")
        result = remove_file(
            filepath, force=False, interactive=False, recursive=False,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is False

    def test_remove_nonexistent_force_ok(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "nonexistent")
        result = remove_file(
            filepath, force=True, interactive=False, recursive=False,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is True

    def test_remove_directory_fails_without_r(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "mydir")
        os.mkdir(dirpath)
        result = remove_file(
            dirpath, force=False, interactive=False, recursive=False,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is False

    def test_remove_directory_recursive(self, tmp_path: Path) -> None:
        dirpath = tmp_path / "mydir"
        dirpath.mkdir()
        (dirpath / "file.txt").write_text("data")
        (dirpath / "subdir").mkdir()
        (dirpath / "subdir" / "nested.txt").write_text("nested")
        result = remove_file(
            str(dirpath), force=False, interactive=False, recursive=True,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is True
        assert not dirpath.exists()

    def test_remove_empty_dir_with_d_flag(self, tmp_path: Path) -> None:
        dirpath = str(tmp_path / "emptydir")
        os.mkdir(dirpath)
        result = remove_file(
            dirpath, force=False, interactive=False, recursive=False,
            verbose=False, dir_flag=True, preserve_root=True,
        )
        assert result is True
        assert not os.path.exists(dirpath)

    def test_verbose_output(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        filepath = str(tmp_path / "verbose.txt")
        Path(filepath).write_text("data")
        remove_file(
            filepath, force=False, interactive=False, recursive=False,
            verbose=True, dir_flag=False, preserve_root=True,
        )
        captured = capsys.readouterr()
        assert "removed" in captured.out

    def test_preserve_root(self) -> None:
        result = remove_file(
            "/", force=False, interactive=False, recursive=True,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is False

    def test_remove_symlink(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("content")
        link = tmp_path / "link.txt"
        os.symlink(str(target), str(link))
        result = remove_file(
            str(link), force=False, interactive=False, recursive=False,
            verbose=False, dir_flag=False, preserve_root=True,
        )
        assert result is True
        assert not os.path.lexists(str(link))
        assert target.exists()  # Target should still exist.
