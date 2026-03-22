"""Tests for the mv tool.

=== What These Tests Verify ===

These tests exercise the mv implementation, including:

1. Spec loading and CLI Builder integration
2. Moving a file (move_file)
3. Moving into a directory
4. The -n flag (no-clobber)
5. The -u flag (update — only move if newer)
6. The -v flag (verbose)
7. Overwriting behavior
8. Moving directories
"""

from __future__ import annotations

import os
import sys
import time
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "mv.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from mv_tool import move_file


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mv", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------


class TestFlags:
    def test_force_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mv", "-f", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["force"] is True

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mv", "-v", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True

    def test_no_clobber_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mv", "-n", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["no_clobber"] is True

    def test_update_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["mv", "-u", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["update"] is True


# ---------------------------------------------------------------------------
# Help and version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["mv", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["mv", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# move_file — basic operations
# ---------------------------------------------------------------------------


class TestMoveFile:
    def test_move_basic(self, tmp_path: Path) -> None:
        """Moving a file removes the source and creates the destination."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("hello world")

        assert move_file(str(src), str(dst)) is True
        assert not src.exists()
        assert dst.read_text() == "hello world"

    def test_move_nonexistent_source(self, tmp_path: Path) -> None:
        """Moving a nonexistent file returns False."""
        src = tmp_path / "nonexistent.txt"
        dst = tmp_path / "dest.txt"

        assert move_file(str(src), str(dst)) is False

    def test_move_into_directory(self, tmp_path: Path) -> None:
        """When dst is a directory, the file is moved into it."""
        src = tmp_path / "source.txt"
        src.write_text("content")
        dest_dir = tmp_path / "subdir"
        dest_dir.mkdir()

        assert move_file(str(src), str(dest_dir)) is True
        assert not src.exists()
        assert (dest_dir / "source.txt").read_text() == "content"

    def test_move_overwrites_existing(self, tmp_path: Path) -> None:
        """By default, mv overwrites the destination."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("new content")
        dst.write_text("old content")

        assert move_file(str(src), str(dst)) is True
        assert dst.read_text() == "new content"
        assert not src.exists()

    def test_move_rename(self, tmp_path: Path) -> None:
        """Moving within the same directory is a rename."""
        src = tmp_path / "old_name.txt"
        dst = tmp_path / "new_name.txt"
        src.write_text("rename me")

        assert move_file(str(src), str(dst)) is True
        assert not src.exists()
        assert dst.read_text() == "rename me"


# ---------------------------------------------------------------------------
# move_file — overwrite modes
# ---------------------------------------------------------------------------


class TestOverwriteModes:
    def test_no_clobber_skips_existing(self, tmp_path: Path) -> None:
        """With no_clobber=True, existing files are not overwritten."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("new")
        dst.write_text("old")

        assert move_file(str(src), str(dst), no_clobber=True) is True
        assert dst.read_text() == "old"
        # Source should still exist since the move was skipped.
        assert src.exists()

    def test_no_clobber_moves_if_missing(self, tmp_path: Path) -> None:
        """With no_clobber=True, missing destinations still get moved to."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("content")

        assert move_file(str(src), str(dst), no_clobber=True) is True
        assert dst.read_text() == "content"
        assert not src.exists()

    def test_update_skips_older_source(self, tmp_path: Path) -> None:
        """With update=True, an older source does not overwrite."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        dst.write_text("newer")
        time.sleep(0.05)
        src.write_text("older")
        old_time = time.time() - 10
        os.utime(str(src), (old_time, old_time))

        assert move_file(str(src), str(dst), update=True) is True
        assert dst.read_text() == "newer"
        assert src.exists()  # Source not moved.

    def test_update_moves_newer_source(self, tmp_path: Path) -> None:
        """With update=True, a newer source does overwrite."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        dst.write_text("old")
        old_time = time.time() - 10
        os.utime(str(dst), (old_time, old_time))
        time.sleep(0.05)
        src.write_text("new")

        assert move_file(str(src), str(dst), update=True) is True
        assert dst.read_text() == "new"
        assert not src.exists()


# ---------------------------------------------------------------------------
# move_file — verbose output
# ---------------------------------------------------------------------------


class TestVerbose:
    def test_verbose_output(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """With verbose=True, mv prints the rename operation."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("content")

        move_file(str(src), str(dst), verbose=True)
        captured = capsys.readouterr()
        assert "renamed" in captured.out
        assert "->" in captured.out


# ---------------------------------------------------------------------------
# move_file — directory handling
# ---------------------------------------------------------------------------


class TestDirectoryMove:
    def test_move_directory(self, tmp_path: Path) -> None:
        """Moving a directory moves the entire tree."""
        src_dir = tmp_path / "srcdir"
        src_dir.mkdir()
        (src_dir / "file.txt").write_text("hello")
        (src_dir / "sub").mkdir()
        (src_dir / "sub" / "nested.txt").write_text("nested")
        dst = tmp_path / "dstdir"

        assert move_file(str(src_dir), str(dst)) is True
        assert not src_dir.exists()
        assert (dst / "file.txt").read_text() == "hello"
        assert (dst / "sub" / "nested.txt").read_text() == "nested"

    def test_move_directory_into_existing(self, tmp_path: Path) -> None:
        """Moving a directory into an existing directory."""
        src_dir = tmp_path / "srcdir"
        src_dir.mkdir()
        (src_dir / "a.txt").write_text("a")
        dst_dir = tmp_path / "dstdir"
        dst_dir.mkdir()

        assert move_file(str(src_dir), str(dst_dir)) is True
        assert (dst_dir / "srcdir" / "a.txt").read_text() == "a"
