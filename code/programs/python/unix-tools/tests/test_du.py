"""Tests for the du tool.

=== What These Tests Verify ===

These tests exercise the du implementation, including:

1. Spec loading and CLI Builder integration
2. disk_usage on files and directories
3. format_size helper
4. Summarize mode (-s)
5. Max depth (-d)
6. All files mode (-a)
"""

from __future__ import annotations

import os
import sys
import tempfile
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "du.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from du_tool import _relative_depth, disk_usage, format_size


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# CLI Builder integration tests
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["du"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["du", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["du", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_summarize_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["du", "-s"])
        assert isinstance(result, ParseResult)
        assert result.flags["summarize"] is True

    def test_human_readable_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["du", "-h"])
        assert isinstance(result, ParseResult)
        assert result.flags["human_readable"] is True

    def test_all_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["du", "-a"])
        assert isinstance(result, ParseResult)
        assert result.flags["all"] is True


# ---------------------------------------------------------------------------
# format_size tests
# ---------------------------------------------------------------------------


class TestFormatSize:
    def test_default_1k_blocks(self) -> None:
        assert format_size(2048) == "2"

    def test_small_file(self) -> None:
        """Files smaller than 1K should show as at least 1 block."""
        assert format_size(100) == "1"

    def test_zero(self) -> None:
        assert format_size(0) == "0"

    def test_human_kilobytes(self) -> None:
        result = format_size(2048, human=True)
        assert "K" in result

    def test_human_megabytes(self) -> None:
        result = format_size(5 * 1024 * 1024, human=True)
        assert "M" in result

    def test_human_zero(self) -> None:
        assert format_size(0, human=True) == "0"


# ---------------------------------------------------------------------------
# _relative_depth tests
# ---------------------------------------------------------------------------


class TestRelativeDepth:
    def test_same_path(self) -> None:
        assert _relative_depth("/home", "/home") == 0

    def test_one_level_deep(self) -> None:
        assert _relative_depth("/home", "/home/user") == 1

    def test_two_levels_deep(self) -> None:
        assert _relative_depth("/home", "/home/user/docs") == 2


# ---------------------------------------------------------------------------
# disk_usage tests (using temp directories)
# ---------------------------------------------------------------------------


class TestDiskUsage:
    def test_single_file(self) -> None:
        """disk_usage on a single file returns one entry."""
        with tempfile.NamedTemporaryFile(delete=False) as f:
            f.write(b"hello world")
            f.flush()
            fname = f.name

        try:
            entries = disk_usage(fname)
            assert len(entries) == 1
            assert entries[0][0] > 0
            assert entries[0][1] == fname
        finally:
            os.unlink(fname)

    def test_empty_directory(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            entries = disk_usage(tmpdir)
            assert len(entries) >= 1
            # The directory itself should be in the results.
            paths = [e[1] for e in entries]
            assert tmpdir in paths

    def test_directory_with_files(self) -> None:
        with tempfile.TemporaryDirectory() as tmpdir:
            # Create some files.
            for name in ["a.txt", "b.txt"]:
                with open(os.path.join(tmpdir, name), "w") as f:
                    f.write("content " * 100)

            entries = disk_usage(tmpdir)
            assert len(entries) >= 1
            # Total size should be positive.
            total = sum(e[0] for e in entries if e[1] == tmpdir)
            assert total > 0

    def test_all_files_mode(self) -> None:
        """With all_files=True, individual files appear in output."""
        with tempfile.TemporaryDirectory() as tmpdir:
            filepath = os.path.join(tmpdir, "test.txt")
            with open(filepath, "w") as f:
                f.write("test content")

            entries = disk_usage(tmpdir, all_files=True)
            paths = [e[1] for e in entries]
            assert filepath in paths

    def test_summarize_mode(self) -> None:
        """With summarize=True, only the top-level total is returned."""
        with tempfile.TemporaryDirectory() as tmpdir:
            subdir = os.path.join(tmpdir, "subdir")
            os.makedirs(subdir)
            with open(os.path.join(subdir, "file.txt"), "w") as f:
                f.write("data")

            entries = disk_usage(tmpdir, summarize=True)
            assert len(entries) == 1
            assert entries[0][1] == tmpdir

    def test_max_depth(self) -> None:
        """With max_depth=0, only the top-level is shown."""
        with tempfile.TemporaryDirectory() as tmpdir:
            subdir = os.path.join(tmpdir, "sub")
            os.makedirs(subdir)
            with open(os.path.join(subdir, "f.txt"), "w") as f:
                f.write("data")

            entries = disk_usage(tmpdir, max_depth=0)
            paths = [e[1] for e in entries]
            assert tmpdir in paths
            assert subdir not in paths

    def test_nested_directories(self) -> None:
        """Sizes propagate up from nested directories."""
        with tempfile.TemporaryDirectory() as tmpdir:
            subdir = os.path.join(tmpdir, "level1")
            os.makedirs(subdir)
            with open(os.path.join(subdir, "file.txt"), "w") as f:
                f.write("x" * 1000)

            entries = disk_usage(tmpdir)
            # The parent directory's size should include the file.
            parent_entry = [e for e in entries if e[1] == tmpdir]
            assert len(parent_entry) == 1
            assert parent_entry[0][0] >= 1000
