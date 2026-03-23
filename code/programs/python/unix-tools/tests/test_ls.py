"""Tests for the ls tool.

=== What These Tests Verify ===

These tests exercise the ls implementation, including:

1. Spec loading and CLI Builder integration
2. Listing directory contents (list_directory)
3. Hidden file filtering (-a, -A)
4. Long format output (-l)
5. Human-readable sizes (-h, --si)
6. Sorting modes (-S, -t, -X, -v, -U)
7. Reverse sort (-r)
8. Classify indicator (-F)
9. Permission formatting (format_permissions)
10. Size formatting (format_size)
11. Directory-only listing (-d)
12. Inode display (-i)
"""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "ls.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from ls_tool import (
    LsOptions,
    classify_suffix,
    format_entry,
    format_permissions,
    format_size,
    format_time,
    list_directory,
)


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

        result = parse_argv(["ls"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Help and version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["ls", "--help"])
        assert isinstance(result, HelpResult)
        assert "ls" in result.text.lower()

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["ls", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------


class TestFlags:
    def test_all_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-a"])
        assert isinstance(result, ParseResult)
        assert result.flags["all"] is True

    def test_almost_all_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-A"])
        assert isinstance(result, ParseResult)
        assert result.flags["almost_all"] is True

    def test_long_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-l"])
        assert isinstance(result, ParseResult)
        assert result.flags["long"] is True

    def test_reverse_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-r"])
        assert isinstance(result, ParseResult)
        assert result.flags["reverse"] is True

    def test_recursive_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-R"])
        assert isinstance(result, ParseResult)
        assert result.flags["recursive"] is True

    def test_sort_by_size_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-S"])
        assert isinstance(result, ParseResult)
        assert result.flags["sort_by_size"] is True

    def test_classify_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ls", "-F"])
        assert isinstance(result, ParseResult)
        assert result.flags["classify"] is True


# ---------------------------------------------------------------------------
# format_size
# ---------------------------------------------------------------------------


class TestFormatSize:
    def test_plain_bytes(self) -> None:
        """Without flags, sizes are shown as plain numbers."""
        assert format_size(4096) == "4096"

    def test_human_readable_kb(self) -> None:
        """1024 bytes = 1.0K in human-readable mode."""
        assert format_size(1024, human_readable=True) == "1.0K"

    def test_human_readable_mb(self) -> None:
        """1048576 bytes = 1.0M in human-readable mode."""
        assert format_size(1048576, human_readable=True) == "1.0M"

    def test_si_kb(self) -> None:
        """1000 bytes = 1.0k in SI mode."""
        assert format_size(1000, si=True) == "1.0k"

    def test_small_size(self) -> None:
        """Sizes less than 1K remain as plain numbers."""
        assert format_size(500, human_readable=True) == "500"

    def test_zero(self) -> None:
        """Zero is shown as 0."""
        assert format_size(0) == "0"


# ---------------------------------------------------------------------------
# format_permissions
# ---------------------------------------------------------------------------


class TestFormatPermissions:
    def test_regular_file(self) -> None:
        """A regular file with mode 644."""
        mode = stat.S_IFREG | 0o644
        result = format_permissions(mode)
        assert result == "-rw-r--r--"

    def test_directory(self) -> None:
        """A directory with mode 755."""
        mode = stat.S_IFDIR | 0o755
        result = format_permissions(mode)
        assert result == "drwxr-xr-x"

    def test_executable(self) -> None:
        """A regular file with mode 755."""
        mode = stat.S_IFREG | 0o755
        result = format_permissions(mode)
        assert result == "-rwxr-xr-x"

    def test_no_permissions(self) -> None:
        """A regular file with mode 000."""
        mode = stat.S_IFREG | 0o000
        result = format_permissions(mode)
        assert result == "----------"

    def test_setuid(self) -> None:
        """A file with setuid bit set."""
        mode = stat.S_IFREG | 0o4755
        result = format_permissions(mode)
        assert result[3] == "s"  # Owner execute becomes 's'

    def test_sticky(self) -> None:
        """A directory with sticky bit set."""
        mode = stat.S_IFDIR | 0o1755
        result = format_permissions(mode)
        assert result[9] == "t"  # Other execute becomes 't'


# ---------------------------------------------------------------------------
# classify_suffix
# ---------------------------------------------------------------------------


class TestClassifySuffix:
    def test_directory(self) -> None:
        mode = stat.S_IFDIR | 0o755
        assert classify_suffix(mode) == "/"

    def test_executable(self) -> None:
        mode = stat.S_IFREG | 0o755
        assert classify_suffix(mode) == "*"

    def test_regular_file(self) -> None:
        mode = stat.S_IFREG | 0o644
        assert classify_suffix(mode) == ""

    def test_fifo(self) -> None:
        mode = stat.S_IFIFO | 0o644
        assert classify_suffix(mode) == "|"

    def test_socket(self) -> None:
        mode = stat.S_IFSOCK | 0o755
        assert classify_suffix(mode) == "="


# ---------------------------------------------------------------------------
# format_time
# ---------------------------------------------------------------------------


class TestFormatTime:
    def test_recent_file(self) -> None:
        """Recent files show time of day."""
        import time

        now = time.time()
        result = format_time(now)
        # Should contain HH:MM format.
        assert ":" in result

    def test_old_file(self) -> None:
        """Old files show the year."""
        import time

        # A year ago.
        old = time.time() - 365 * 24 * 3600
        result = format_time(old)
        # Should contain a year number.
        year = time.strftime("%Y", time.localtime(old))
        assert year in result


# ---------------------------------------------------------------------------
# list_directory — basic listing
# ---------------------------------------------------------------------------


class TestListDirectory:
    def test_list_empty_directory(self, tmp_path: Path) -> None:
        """An empty directory produces no output."""
        opts = LsOptions()
        result = list_directory(str(tmp_path), opts)
        assert result == []

    def test_list_files(self, tmp_path: Path) -> None:
        """Files are listed by name."""
        (tmp_path / "alpha.txt").write_text("a")
        (tmp_path / "beta.txt").write_text("b")
        opts = LsOptions()
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert "alpha.txt" in names
        assert "beta.txt" in names

    def test_hidden_files_omitted_by_default(self, tmp_path: Path) -> None:
        """Dot-files are hidden by default."""
        (tmp_path / ".hidden").write_text("h")
        (tmp_path / "visible").write_text("v")
        opts = LsOptions()
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert "visible" in names
        assert ".hidden" not in names

    def test_show_all_includes_dot_and_dotdot(self, tmp_path: Path) -> None:
        """With -a, '.' and '..' are included."""
        (tmp_path / "file.txt").write_text("data")
        opts = LsOptions(show_all=True)
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert "." in names
        assert ".." in names

    def test_almost_all_includes_dotfiles(self, tmp_path: Path) -> None:
        """With -A, dot-files are shown but not '.' and '..'."""
        (tmp_path / ".hidden").write_text("h")
        (tmp_path / "visible").write_text("v")
        opts = LsOptions(almost_all=True)
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert ".hidden" in names
        assert "." not in names
        assert ".." not in names


# ---------------------------------------------------------------------------
# list_directory — sorting
# ---------------------------------------------------------------------------


class TestSorting:
    def test_default_alphabetical(self, tmp_path: Path) -> None:
        """Default sort is alphabetical."""
        (tmp_path / "cherry").write_text("c")
        (tmp_path / "apple").write_text("a")
        (tmp_path / "banana").write_text("b")
        opts = LsOptions()
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert names == ["apple", "banana", "cherry"]

    def test_reverse_sort(self, tmp_path: Path) -> None:
        """With -r, the sort is reversed."""
        (tmp_path / "a").write_text("a")
        (tmp_path / "b").write_text("b")
        (tmp_path / "c").write_text("c")
        opts = LsOptions(reverse=True)
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert names == ["c", "b", "a"]

    def test_sort_by_size(self, tmp_path: Path) -> None:
        """With -S, files are sorted by size, largest first."""
        (tmp_path / "small").write_text("a")
        (tmp_path / "large").write_text("a" * 1000)
        (tmp_path / "medium").write_text("a" * 100)
        opts = LsOptions(sort_by_size=True)
        result = list_directory(str(tmp_path), opts)
        names = [line.strip() for line in result]
        assert names[0] == "large"

    def test_unsorted(self, tmp_path: Path) -> None:
        """With -U, entries are not sorted (directory order)."""
        for name in ["z", "a", "m"]:
            (tmp_path / name).write_text(name)
        opts = LsOptions(unsorted=True)
        result = list_directory(str(tmp_path), opts)
        # We can't predict directory order, but there should be 3 entries.
        assert len(result) == 3


# ---------------------------------------------------------------------------
# list_directory — long format
# ---------------------------------------------------------------------------


class TestLongFormat:
    def test_long_format_has_permissions(self, tmp_path: Path) -> None:
        """Long format includes a permission string."""
        (tmp_path / "file.txt").write_text("data")
        opts = LsOptions(long_format=True)
        result = list_directory(str(tmp_path), opts)
        assert len(result) >= 1
        # Permission string starts with '-' for a regular file.
        assert result[0].startswith("-")

    def test_long_format_has_filename(self, tmp_path: Path) -> None:
        """Long format ends with the filename."""
        (tmp_path / "myfile.txt").write_text("data")
        opts = LsOptions(long_format=True)
        result = list_directory(str(tmp_path), opts)
        assert any("myfile.txt" in line for line in result)

    def test_human_readable_in_long(self, tmp_path: Path) -> None:
        """With -lh, sizes use K/M/G suffixes."""
        (tmp_path / "big.dat").write_bytes(b"x" * 2048)
        opts = LsOptions(long_format=True, human_readable=True)
        result = list_directory(str(tmp_path), opts)
        assert any("K" in line for line in result)


# ---------------------------------------------------------------------------
# list_directory — classify
# ---------------------------------------------------------------------------


class TestClassify:
    def test_classify_directory(self, tmp_path: Path) -> None:
        """With -F, directories get a '/' suffix."""
        (tmp_path / "subdir").mkdir()
        opts = LsOptions(classify=True)
        result = list_directory(str(tmp_path), opts)
        assert any("subdir/" in line for line in result)

    def test_classify_regular_file(self, tmp_path: Path) -> None:
        """With -F, regular files have no suffix."""
        (tmp_path / "file.txt").write_text("data")
        opts = LsOptions(classify=True)
        result = list_directory(str(tmp_path), opts)
        # file.txt should NOT have a trailing indicator.
        assert any(line.strip() == "file.txt" for line in result)


# ---------------------------------------------------------------------------
# list_directory — inode
# ---------------------------------------------------------------------------


class TestInode:
    def test_inode_shown(self, tmp_path: Path) -> None:
        """With -i, the inode number is shown."""
        (tmp_path / "file.txt").write_text("data")
        opts = LsOptions(inode=True)
        result = list_directory(str(tmp_path), opts)
        assert len(result) >= 1
        # The inode number is a digit string.
        parts = result[0].split()
        assert parts[0].isdigit()


# ---------------------------------------------------------------------------
# list_directory — directory flag
# ---------------------------------------------------------------------------


class TestDirectoryFlag:
    def test_directory_flag_lists_dir_itself(self, tmp_path: Path) -> None:
        """With -d, the directory itself is listed, not its contents."""
        (tmp_path / "file.txt").write_text("data")
        opts = LsOptions(directory=True)
        result = list_directory(str(tmp_path), opts)
        assert len(result) == 1
        assert str(tmp_path) in result[0]


# ---------------------------------------------------------------------------
# list_directory — error handling
# ---------------------------------------------------------------------------


class TestErrorHandling:
    def test_nonexistent_directory(self, tmp_path: Path) -> None:
        """Listing a nonexistent path prints an error."""
        opts = LsOptions()
        result = list_directory(str(tmp_path / "nonexistent"), opts)
        assert any("cannot access" in line for line in result)


# ---------------------------------------------------------------------------
# list_directory — recursive
# ---------------------------------------------------------------------------


class TestRecursive:
    def test_recursive_lists_subdirs(self, tmp_path: Path) -> None:
        """With -R, subdirectories are listed."""
        sub = tmp_path / "subdir"
        sub.mkdir()
        (sub / "nested.txt").write_text("nested")
        (tmp_path / "top.txt").write_text("top")
        opts = LsOptions(recursive=True)
        result = list_directory(str(tmp_path), opts)
        all_text = "\n".join(result)
        assert "nested.txt" in all_text
        assert "subdir" in all_text


# ---------------------------------------------------------------------------
# format_entry
# ---------------------------------------------------------------------------


class TestFormatEntry:
    def test_short_format(self, tmp_path: Path) -> None:
        """Short format returns just the filename."""
        f = tmp_path / "test.txt"
        f.write_text("data")
        st = os.lstat(str(f))
        opts = LsOptions()
        result = format_entry("test.txt", st, opts)
        assert result == "test.txt"

    def test_short_format_with_classify(self, tmp_path: Path) -> None:
        """Short format with classify appends indicator."""
        d = tmp_path / "mydir"
        d.mkdir()
        st = os.lstat(str(d))
        opts = LsOptions(classify=True)
        result = format_entry("mydir", st, opts)
        assert result == "mydir/"

    def test_long_format_entry(self, tmp_path: Path) -> None:
        """Long format includes permissions, size, etc."""
        f = tmp_path / "test.txt"
        f.write_text("data")
        st = os.lstat(str(f))
        opts = LsOptions(long_format=True)
        result = format_entry("test.txt", st, opts)
        assert "test.txt" in result
        assert result.startswith("-")  # Regular file
