"""Tests for the cp tool.

=== What These Tests Verify ===

These tests exercise the cp implementation, including:

1. Spec loading and CLI Builder integration
2. Copying a single file (copy_file)
3. Copying directories recursively (copy_directory)
4. The -n flag (no-clobber)
5. The -u flag (update — only copy if newer)
6. The -v flag (verbose)
7. The -l flag (hard link)
8. The -s flag (symbolic link)
9. Destination directory handling
"""

from __future__ import annotations

import os
import sys
import time
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "cp.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from cp_tool import copy_directory, copy_file


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

        result = parse_argv(["cp", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------


class TestFlags:
    def test_force_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cp", "-f", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["force"] is True

    def test_recursive_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cp", "-R", "src", "dst"])
        assert isinstance(result, ParseResult)
        assert result.flags["recursive"] is True

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cp", "-v", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True

    def test_no_clobber_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cp", "-n", "src.txt", "dst.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["no_clobber"] is True

    def test_archive_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["cp", "-a", "src", "dst"])
        assert isinstance(result, ParseResult)
        assert result.flags["archive"] is True


# ---------------------------------------------------------------------------
# Help and version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["cp", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["cp", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# copy_file — basic operations
# ---------------------------------------------------------------------------


class TestCopyFile:
    def test_copy_basic(self, tmp_path: Path) -> None:
        """Copying a file creates an identical copy at the destination."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("hello world")

        assert copy_file(str(src), str(dst)) is True
        assert dst.read_text() == "hello world"

    def test_copy_preserves_content(self, tmp_path: Path) -> None:
        """The copied file has the same content as the source."""
        src = tmp_path / "data.bin"
        content = b"\x00\x01\x02\x03" * 1000
        src.write_bytes(content)
        dst = tmp_path / "copy.bin"

        assert copy_file(str(src), str(dst)) is True
        assert dst.read_bytes() == content

    def test_copy_nonexistent_source(self, tmp_path: Path) -> None:
        """Copying a nonexistent file returns False."""
        src = tmp_path / "nonexistent.txt"
        dst = tmp_path / "dest.txt"

        assert copy_file(str(src), str(dst)) is False

    def test_copy_into_directory(self, tmp_path: Path) -> None:
        """When dst is a directory, the file is copied into it."""
        src = tmp_path / "source.txt"
        src.write_text("content")
        dest_dir = tmp_path / "subdir"
        dest_dir.mkdir()

        assert copy_file(str(src), str(dest_dir)) is True
        assert (dest_dir / "source.txt").read_text() == "content"

    def test_copy_overwrites_existing(self, tmp_path: Path) -> None:
        """By default, cp overwrites the destination."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("new content")
        dst.write_text("old content")

        assert copy_file(str(src), str(dst)) is True
        assert dst.read_text() == "new content"


# ---------------------------------------------------------------------------
# copy_file — overwrite modes
# ---------------------------------------------------------------------------


class TestOverwriteModes:
    def test_no_clobber_skips_existing(self, tmp_path: Path) -> None:
        """With no_clobber=True, existing files are not overwritten."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("new")
        dst.write_text("old")

        assert copy_file(str(src), str(dst), no_clobber=True) is True
        assert dst.read_text() == "old"

    def test_no_clobber_copies_if_missing(self, tmp_path: Path) -> None:
        """With no_clobber=True, missing files are still copied."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("content")

        assert copy_file(str(src), str(dst), no_clobber=True) is True
        assert dst.read_text() == "content"

    def test_update_skips_older_source(self, tmp_path: Path) -> None:
        """With update=True, an older source does not overwrite."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        dst.write_text("newer")
        time.sleep(0.05)
        # Make src older by setting its mtime to the past.
        src.write_text("older")
        old_time = time.time() - 10
        os.utime(str(src), (old_time, old_time))

        assert copy_file(str(src), str(dst), update=True) is True
        assert dst.read_text() == "newer"

    def test_update_copies_newer_source(self, tmp_path: Path) -> None:
        """With update=True, a newer source does overwrite."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        dst.write_text("old")
        old_time = time.time() - 10
        os.utime(str(dst), (old_time, old_time))
        time.sleep(0.05)
        src.write_text("new")

        assert copy_file(str(src), str(dst), update=True) is True
        assert dst.read_text() == "new"


# ---------------------------------------------------------------------------
# copy_file — verbose output
# ---------------------------------------------------------------------------


class TestVerbose:
    def test_verbose_output(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """With verbose=True, cp prints what it did."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("content")

        copy_file(str(src), str(dst), verbose=True)
        captured = capsys.readouterr()
        assert "->" in captured.out


# ---------------------------------------------------------------------------
# copy_file — link modes
# ---------------------------------------------------------------------------


class TestLinkModes:
    def test_hard_link(self, tmp_path: Path) -> None:
        """With link=True, a hard link is created instead of a copy."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "dest.txt"
        src.write_text("content")

        assert copy_file(str(src), str(dst), link=True) is True
        # Hard links share the same inode.
        assert os.stat(str(src)).st_ino == os.stat(str(dst)).st_ino

    def test_symbolic_link(self, tmp_path: Path) -> None:
        """With symbolic_link=True, a symlink is created."""
        src = tmp_path / "source.txt"
        dst = tmp_path / "link.txt"
        src.write_text("content")

        assert copy_file(str(src), str(dst), symbolic_link=True) is True
        assert os.path.islink(str(dst))
        assert dst.read_text() == "content"


# ---------------------------------------------------------------------------
# copy_file — directory handling
# ---------------------------------------------------------------------------


class TestDirectoryHandling:
    def test_directory_without_recursive_fails(self, tmp_path: Path) -> None:
        """Copying a directory without -R returns False."""
        src_dir = tmp_path / "srcdir"
        src_dir.mkdir()
        dst = tmp_path / "dstdir"

        assert copy_file(str(src_dir), str(dst), recursive=False) is False

    def test_directory_with_recursive(self, tmp_path: Path) -> None:
        """Copying a directory with -R copies the entire tree."""
        src_dir = tmp_path / "srcdir"
        src_dir.mkdir()
        (src_dir / "file.txt").write_text("hello")
        (src_dir / "sub").mkdir()
        (src_dir / "sub" / "nested.txt").write_text("nested")
        dst = tmp_path / "dstdir"

        assert copy_file(str(src_dir), str(dst), recursive=True) is True
        assert (dst / "file.txt").read_text() == "hello"
        assert (dst / "sub" / "nested.txt").read_text() == "nested"


# ---------------------------------------------------------------------------
# copy_directory — directly
# ---------------------------------------------------------------------------


class TestCopyDirectory:
    def test_copy_into_existing_dir(self, tmp_path: Path) -> None:
        """When dst exists, the source is copied inside it."""
        src_dir = tmp_path / "src"
        src_dir.mkdir()
        (src_dir / "a.txt").write_text("a")
        dst_dir = tmp_path / "dst"
        dst_dir.mkdir()

        assert copy_directory(str(src_dir), str(dst_dir)) is True
        assert (dst_dir / "src" / "a.txt").read_text() == "a"

    def test_copy_to_new_dir(self, tmp_path: Path) -> None:
        """When dst does not exist, it becomes the copy."""
        src_dir = tmp_path / "src"
        src_dir.mkdir()
        (src_dir / "b.txt").write_text("b")
        dst_dir = tmp_path / "new_dst"

        assert copy_directory(str(src_dir), str(dst_dir)) is True
        assert (dst_dir / "b.txt").read_text() == "b"

    def test_copy_verbose(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        """Verbose mode prints what was copied."""
        src_dir = tmp_path / "src"
        src_dir.mkdir()
        (src_dir / "c.txt").write_text("c")
        dst_dir = tmp_path / "dst_v"

        copy_directory(str(src_dir), str(dst_dir), verbose=True)
        captured = capsys.readouterr()
        assert "->" in captured.out


# ---------------------------------------------------------------------------
# copy_file — archive mode
# ---------------------------------------------------------------------------


class TestArchiveMode:
    def test_archive_copies_directory(self, tmp_path: Path) -> None:
        """The -a flag enables recursive copy for directories."""
        src_dir = tmp_path / "srcdir"
        src_dir.mkdir()
        (src_dir / "file.txt").write_text("archive me")
        dst = tmp_path / "backup"

        assert copy_file(str(src_dir), str(dst), archive=True) is True
        assert (dst / "file.txt").read_text() == "archive me"
