"""Tests for the diff tool.

=== What These Tests Verify ===

These tests exercise the diff implementation, including:

1. Normal diff output format
2. Unified diff output format
3. Context diff output format
4. Line preprocessing (-i, -b, -w, -B)
5. Brief mode (-q)
6. Recursive directory comparison (-r)
7. File handling (missing files, identical files)
8. Spec loading and CLI Builder integration
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

SPEC_FILE = str(Path(__file__).parent.parent / "diff.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from diff_tool import (
    _preprocess_line,
    _preprocess_lines,
    brief_diff,
    context_diff,
    diff_directories,
    diff_files,
    normal_diff,
    unified_diff,
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

        result = parse_argv(["diff", "file1", "file2"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["diff", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["diff", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# Line preprocessing
# ---------------------------------------------------------------------------


class TestPreprocessing:
    def test_ignore_case(self) -> None:
        """The -i flag folds case for comparison."""
        result = _preprocess_line("Hello World", ignore_case=True)
        assert result == "hello world"

    def test_ignore_space_change(self) -> None:
        """The -b flag collapses whitespace runs."""
        result = _preprocess_line("hello   world  ", ignore_space_change=True)
        assert result == "hello world"

    def test_ignore_all_space(self) -> None:
        """The -w flag removes all whitespace."""
        result = _preprocess_line("h e l l o", ignore_all_space=True)
        assert result == "hello"

    def test_ignore_blank_lines(self) -> None:
        """The -B flag removes blank lines."""
        lines = ["hello", "", "world", "   ", "!"]
        result = _preprocess_lines(lines, ignore_blank_lines=True)
        assert result == ["hello", "world", "!"]

    def test_combined_preprocessing(self) -> None:
        """Multiple flags can be combined."""
        result = _preprocess_line(
            "  Hello  WORLD  ",
            ignore_case=True,
            ignore_space_change=True,
        )
        assert result == "hello world"

    def test_all_space_takes_priority_over_space_change(self) -> None:
        """The -w flag takes priority over -b."""
        result = _preprocess_line(
            "h e l l o",
            ignore_all_space=True,
            ignore_space_change=True,
        )
        assert result == "hello"


# ---------------------------------------------------------------------------
# Normal diff
# ---------------------------------------------------------------------------


class TestNormalDiff:
    def test_identical_files(self) -> None:
        """Identical files produce no output."""
        lines = ["line1", "line2", "line3"]
        result = normal_diff(lines, lines)
        assert result == []

    def test_added_line(self) -> None:
        """An added line shows an 'a' command."""
        lines1 = ["line1", "line3"]
        lines2 = ["line1", "line2", "line3"]
        result = normal_diff(lines1, lines2)
        assert any("a" in line for line in result if not line.startswith(">"))
        assert any(line.startswith("> line2") for line in result)

    def test_deleted_line(self) -> None:
        """A deleted line shows a 'd' command."""
        lines1 = ["line1", "line2", "line3"]
        lines2 = ["line1", "line3"]
        result = normal_diff(lines1, lines2)
        assert any("d" in line for line in result if not line.startswith("<"))
        assert any(line.startswith("< line2") for line in result)

    def test_changed_line(self) -> None:
        """A changed line shows a 'c' command."""
        lines1 = ["line1", "old", "line3"]
        lines2 = ["line1", "new", "line3"]
        result = normal_diff(lines1, lines2)
        skip = ("<", ">", "-")
        assert any("c" in line for line in result if not line.startswith(skip))
        assert any(line.startswith("< old") for line in result)
        assert any(line.startswith("> new") for line in result)

    def test_empty_vs_nonempty(self) -> None:
        """Comparing empty and non-empty produces output."""
        result = normal_diff([], ["hello"])
        assert len(result) > 0

    def test_multiple_changes(self) -> None:
        """Multiple changes produce multiple commands."""
        lines1 = ["a", "b", "c", "d"]
        lines2 = ["a", "B", "c", "D"]
        result = normal_diff(lines1, lines2)
        assert len(result) > 0


# ---------------------------------------------------------------------------
# Unified diff
# ---------------------------------------------------------------------------


class TestUnifiedDiff:
    def test_identical_files(self) -> None:
        """Identical files produce no output."""
        lines = ["line1", "line2"]
        result = unified_diff(lines, lines, "a.txt", "b.txt")
        assert result == []

    def test_header_lines(self) -> None:
        """Unified diff starts with --- and +++ headers."""
        lines1 = ["hello"]
        lines2 = ["world"]
        result = unified_diff(lines1, lines2, "a.txt", "b.txt")
        assert any(line.startswith("---") for line in result)
        assert any(line.startswith("+++") for line in result)

    def test_change_markers(self) -> None:
        """Changed lines are marked with - and +."""
        lines1 = ["old line"]
        lines2 = ["new line"]
        result = unified_diff(lines1, lines2, "a.txt", "b.txt")
        assert any(line.startswith("-old") for line in result)
        assert any(line.startswith("+new") for line in result)

    def test_context_lines(self) -> None:
        """Context lines are included around changes."""
        lines1 = ["a", "b", "c", "d", "e"]
        lines2 = ["a", "b", "X", "d", "e"]
        result = unified_diff(lines1, lines2, "a.txt", "b.txt", context=1)
        # Should contain the context line "b" before the change.
        assert any(line == " b" for line in result)


# ---------------------------------------------------------------------------
# Context diff
# ---------------------------------------------------------------------------


class TestContextDiff:
    def test_identical_files(self) -> None:
        """Identical files produce no output."""
        lines = ["line1", "line2"]
        result = context_diff(lines, lines, "a.txt", "b.txt")
        assert result == []

    def test_header_lines(self) -> None:
        """Context diff starts with *** and --- headers."""
        lines1 = ["hello"]
        lines2 = ["world"]
        result = context_diff(lines1, lines2, "a.txt", "b.txt")
        assert any(line.startswith("***") for line in result)
        assert any(line.startswith("---") for line in result)


# ---------------------------------------------------------------------------
# Brief diff
# ---------------------------------------------------------------------------


class TestBriefDiff:
    def test_identical_files(self, tmp_path: Path) -> None:
        """Identical files produce no output."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("hello\n")
        result = brief_diff(str(f1), str(f2))
        assert result is None

    def test_different_files(self, tmp_path: Path) -> None:
        """Different files produce a 'differ' message."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("world\n")
        result = brief_diff(str(f1), str(f2))
        assert result is not None
        assert "differ" in result


# ---------------------------------------------------------------------------
# diff_files (main function)
# ---------------------------------------------------------------------------


class TestDiffFiles:
    def test_normal_format(self, tmp_path: Path) -> None:
        """Default format is normal diff."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("world\n")
        result = diff_files(str(f1), str(f2))
        assert len(result) > 0

    def test_unified_format(self, tmp_path: Path) -> None:
        """Unified format via output_format='unified'."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("world\n")
        result = diff_files(str(f1), str(f2), output_format="unified")
        assert any(line.startswith("---") for line in result)

    def test_context_format(self, tmp_path: Path) -> None:
        """Context format via output_format='context'."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("world\n")
        result = diff_files(str(f1), str(f2), output_format="context")
        assert any(line.startswith("***") for line in result)

    def test_identical_files(self, tmp_path: Path) -> None:
        """Identical files produce no output."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("hello\n")
        result = diff_files(str(f1), str(f2))
        assert result == []

    def test_missing_file(self, tmp_path: Path) -> None:
        """Missing file produces an error message."""
        f1 = tmp_path / "a.txt"
        f1.write_text("hello\n")
        result = diff_files(str(f1), str(tmp_path / "nonexistent.txt"))
        assert len(result) > 0
        assert "No such file" in result[0]

    def test_brief_mode(self, tmp_path: Path) -> None:
        """Brief mode only reports whether files differ."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello\n")
        f2.write_text("world\n")
        result = diff_files(str(f1), str(f2), brief=True)
        assert len(result) == 1
        assert "differ" in result[0]

    def test_ignore_case(self, tmp_path: Path) -> None:
        """The -i flag makes comparison case-insensitive."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("Hello\n")
        f2.write_text("hello\n")
        result = diff_files(str(f1), str(f2), ignore_case=True)
        assert result == []

    def test_ignore_all_space(self, tmp_path: Path) -> None:
        """The -w flag ignores all whitespace differences."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("h e l l o\n")
        f2.write_text("hello\n")
        result = diff_files(str(f1), str(f2), ignore_all_space=True)
        assert result == []

    def test_treat_absent_as_empty(self, tmp_path: Path) -> None:
        """Absent file treated as empty shows all lines as added."""
        f1 = tmp_path / "a.txt"
        f1.write_text("hello\nworld\n")
        result = diff_files(
            str(f1), str(tmp_path / "nonexistent.txt"),
            treat_absent_as_empty=True,
        )
        assert len(result) > 0


# ---------------------------------------------------------------------------
# Recursive directory comparison
# ---------------------------------------------------------------------------


class TestDiffDirectories:
    def test_identical_dirs(self, tmp_path: Path) -> None:
        """Identical directories produce no diff output."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "file.txt").write_text("hello\n")
        (d2 / "file.txt").write_text("hello\n")

        result = diff_directories(str(d1), str(d2))
        assert result == []

    def test_different_files_in_dirs(self, tmp_path: Path) -> None:
        """Different files in directories produce diff output."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "file.txt").write_text("hello\n")
        (d2 / "file.txt").write_text("world\n")

        result = diff_directories(str(d1), str(d2))
        assert len(result) > 0

    def test_only_in_one_dir(self, tmp_path: Path) -> None:
        """Files present in only one directory are reported."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "only_here.txt").write_text("content\n")
        (d2 / "other.txt").write_text("content\n")

        result = diff_directories(str(d1), str(d2))
        assert any("Only in" in line for line in result)

    def test_subdirectory_recursion(self, tmp_path: Path) -> None:
        """Subdirectories are compared recursively."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "sub").mkdir()
        (d2 / "sub").mkdir()
        (d1 / "sub" / "file.txt").write_text("hello\n")
        (d2 / "sub" / "file.txt").write_text("world\n")

        result = diff_directories(str(d1), str(d2))
        assert len(result) > 0

    def test_exclude_pattern(self, tmp_path: Path) -> None:
        """Exclude patterns filter out matching files."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "file.txt").write_text("hello\n")
        (d2 / "file.txt").write_text("world\n")
        (d1 / "file.bak").write_text("backup\n")
        (d2 / "file.bak").write_text("different backup\n")

        result = diff_directories(
            str(d1), str(d2),
            exclude_patterns=["*.bak"],
        )
        # Should only show diff for file.txt, not file.bak.
        bak_mentioned = any("bak" in line for line in result)
        assert not bak_mentioned

    def test_brief_mode_in_dirs(self, tmp_path: Path) -> None:
        """Brief mode in directories shows 'differ' messages."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "file.txt").write_text("hello\n")
        (d2 / "file.txt").write_text("world\n")

        result = diff_directories(str(d1), str(d2), brief=True)
        assert any("differ" in line for line in result)

    def test_new_file_flag(self, tmp_path: Path) -> None:
        """The -N flag treats absent files as empty."""
        d1 = tmp_path / "dir1"
        d2 = tmp_path / "dir2"
        d1.mkdir()
        d2.mkdir()
        (d1 / "only_here.txt").write_text("content\n")

        result = diff_directories(str(d1), str(d2), new_file=True)
        # Should show diff output, not "Only in".
        assert not any("Only in" in line for line in result)
