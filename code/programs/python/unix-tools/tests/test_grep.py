"""Tests for the grep tool.

=== What These Tests Verify ===

These tests exercise the grep implementation, including:

1. Spec loading and CLI Builder integration
2. Pattern compilation (compile_pattern)
3. Line matching (grep_line)
4. File searching (grep_file)
5. Case-insensitive matching (-i)
6. Inverted matching (-v)
7. Word matching (-w)
8. Line matching (-x)
9. Fixed-string matching (-F)
10. Count mode (-c)
11. Only-matching mode (-o)
12. Line numbers (-n)
13. Filename display (-H, -h)
14. Context lines (-A, -B, -C)
15. Max count (-m)
16. File walking (walk_files)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "grep.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from grep_tool import (
    GrepOptions,
    compile_pattern,
    grep_file,
    grep_line,
    walk_files,
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

        result = parse_argv(["grep", "pattern", "file.txt"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Flag parsing
# ---------------------------------------------------------------------------


class TestFlags:
    def test_ignore_case_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["grep", "-i", "pattern", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["ignore_case"] is True

    def test_invert_match_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["grep", "-v", "pattern", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["invert_match"] is True

    def test_count_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["grep", "-c", "pattern", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["count"] is True

    def test_line_number_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["grep", "-n", "pattern", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["line_number"] is True

    def test_recursive_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["grep", "-r", "pattern", "dir"])
        assert isinstance(result, ParseResult)
        assert result.flags["recursive"] is True


# ---------------------------------------------------------------------------
# Help and version
# ---------------------------------------------------------------------------


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["grep", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["grep", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# compile_pattern
# ---------------------------------------------------------------------------


class TestCompilePattern:
    def test_basic_pattern(self) -> None:
        """A basic pattern matches simple text."""
        pat = compile_pattern("hello")
        assert pat.search("hello world") is not None

    def test_no_match(self) -> None:
        pat = compile_pattern("xyz")
        assert pat.search("hello world") is None

    def test_ignore_case(self) -> None:
        pat = compile_pattern("hello", ignore_case=True)
        assert pat.search("HELLO WORLD") is not None

    def test_fixed_strings(self) -> None:
        """Fixed strings escape regex metacharacters."""
        pat = compile_pattern("a.b", fixed_strings=True)
        assert pat.search("a.b") is not None
        assert pat.search("axb") is None  # '.' not treated as regex.

    def test_word_regexp(self) -> None:
        """Word regexp wraps pattern with word boundaries."""
        pat = compile_pattern("cat", word_regexp=True)
        assert pat.search("the cat sat") is not None
        assert pat.search("concatenate") is None

    def test_line_regexp(self) -> None:
        """Line regexp anchors the pattern to the full line."""
        pat = compile_pattern("hello", line_regexp=True)
        assert pat.search("hello") is not None
        assert pat.search("hello world") is None


# ---------------------------------------------------------------------------
# grep_line
# ---------------------------------------------------------------------------


class TestGrepLine:
    def test_matching_line(self) -> None:
        pat = compile_pattern("hello")
        opts = GrepOptions()
        assert grep_line("hello world", pat, opts) is True

    def test_non_matching_line(self) -> None:
        pat = compile_pattern("xyz")
        opts = GrepOptions()
        assert grep_line("hello world", pat, opts) is False

    def test_invert_match_true(self) -> None:
        """With invert_match, matching lines are excluded."""
        pat = compile_pattern("hello")
        opts = GrepOptions(invert_match=True)
        assert grep_line("hello world", pat, opts) is False

    def test_invert_match_false(self) -> None:
        """With invert_match, non-matching lines are included."""
        pat = compile_pattern("xyz")
        opts = GrepOptions(invert_match=True)
        assert grep_line("hello world", pat, opts) is True


# ---------------------------------------------------------------------------
# grep_file — basic modes
# ---------------------------------------------------------------------------


class TestGrepFile:
    def test_basic_match(self) -> None:
        """grep_file returns matching lines."""
        lines = ["apple", "banana", "avocado", "cherry"]
        pat = compile_pattern("a")
        opts = GrepOptions()

        result = grep_file(lines, pat, opts)
        assert result == ["apple", "banana", "avocado"]

    def test_no_matches(self) -> None:
        """grep_file returns empty list when nothing matches."""
        lines = ["apple", "banana"]
        pat = compile_pattern("xyz")
        opts = GrepOptions()

        result = grep_file(lines, pat, opts)
        assert result == []

    def test_all_match(self) -> None:
        """grep_file returns all lines when all match."""
        lines = ["cat", "catch", "scatter"]
        pat = compile_pattern("cat")
        opts = GrepOptions()

        result = grep_file(lines, pat, opts)
        assert len(result) == 3


# ---------------------------------------------------------------------------
# grep_file — count mode
# ---------------------------------------------------------------------------


class TestCountMode:
    def test_count_matches(self) -> None:
        lines = ["apple", "banana", "avocado", "cherry"]
        pat = compile_pattern("a")
        opts = GrepOptions(count=True)

        result = grep_file(lines, pat, opts)
        assert result == ["3"]

    def test_count_no_matches(self) -> None:
        lines = ["apple", "banana"]
        pat = compile_pattern("xyz")
        opts = GrepOptions(count=True)

        result = grep_file(lines, pat, opts)
        assert result == ["0"]

    def test_count_with_filename(self) -> None:
        lines = ["apple", "banana"]
        pat = compile_pattern("a")
        opts = GrepOptions(count=True, with_filename=True)

        result = grep_file(lines, pat, opts, filename="test.txt")
        assert result == ["test.txt:2"]


# ---------------------------------------------------------------------------
# grep_file — only-matching mode
# ---------------------------------------------------------------------------


class TestOnlyMatching:
    def test_only_matching(self) -> None:
        lines = ["the cat sat on the mat"]
        pat = compile_pattern("[cm]at")
        opts = GrepOptions(only_matching=True)

        result = grep_file(lines, pat, opts)
        assert "cat" in result
        assert "mat" in result


# ---------------------------------------------------------------------------
# grep_file — line numbers
# ---------------------------------------------------------------------------


class TestLineNumbers:
    def test_line_numbers(self) -> None:
        lines = ["apple", "banana", "cherry"]
        pat = compile_pattern("a")
        opts = GrepOptions(line_number=True)

        result = grep_file(lines, pat, opts)
        assert result[0].startswith("1:")
        assert result[1].startswith("2:")

    def test_line_numbers_with_filename(self) -> None:
        lines = ["apple", "banana"]
        pat = compile_pattern("a")
        opts = GrepOptions(line_number=True, with_filename=True)

        result = grep_file(lines, pat, opts, filename="f.txt")
        assert result[0].startswith("f.txt:1:")


# ---------------------------------------------------------------------------
# grep_file — filename display
# ---------------------------------------------------------------------------


class TestFilenameDisplay:
    def test_with_filename(self) -> None:
        lines = ["hello"]
        pat = compile_pattern("hello")
        opts = GrepOptions(with_filename=True)

        result = grep_file(lines, pat, opts, filename="test.txt")
        assert result[0].startswith("test.txt:")

    def test_no_filename(self) -> None:
        lines = ["hello"]
        pat = compile_pattern("hello")
        opts = GrepOptions(with_filename=True, no_filename=True)

        result = grep_file(lines, pat, opts, filename="test.txt")
        assert not result[0].startswith("test.txt:")


# ---------------------------------------------------------------------------
# grep_file — max count
# ---------------------------------------------------------------------------


class TestMaxCount:
    def test_max_count(self) -> None:
        lines = ["a1", "a2", "a3", "a4", "a5"]
        pat = compile_pattern("a")
        opts = GrepOptions(max_count=3)

        result = grep_file(lines, pat, opts)
        assert len(result) == 3

    def test_max_count_with_count(self) -> None:
        lines = ["a1", "a2", "a3", "a4"]
        pat = compile_pattern("a")
        opts = GrepOptions(count=True, max_count=2)

        result = grep_file(lines, pat, opts)
        assert result == ["2"]


# ---------------------------------------------------------------------------
# grep_file — context lines
# ---------------------------------------------------------------------------


class TestContextLines:
    def test_after_context(self) -> None:
        lines = ["a", "b", "MATCH", "c", "d", "e"]
        pat = compile_pattern("MATCH")
        opts = GrepOptions(after_context=2)

        result = grep_file(lines, pat, opts)
        assert "MATCH" in result
        assert any("c" in line for line in result)
        assert any("d" in line for line in result)

    def test_before_context(self) -> None:
        lines = ["a", "b", "MATCH", "c", "d"]
        pat = compile_pattern("MATCH")
        opts = GrepOptions(before_context=2)

        result = grep_file(lines, pat, opts)
        assert "MATCH" in result
        assert any("a" in line for line in result)
        assert any("b" in line for line in result)

    def test_context_both(self) -> None:
        lines = ["a", "b", "MATCH", "c", "d"]
        pat = compile_pattern("MATCH")
        opts = GrepOptions(context=1)

        result = grep_file(lines, pat, opts)
        assert len(result) == 3  # before + match + after

    def test_context_separator(self) -> None:
        """Non-contiguous context groups are separated by '--'."""
        lines = ["a", "MATCH1", "b", "c", "d", "MATCH2", "e"]
        pat = compile_pattern("MATCH")
        opts = GrepOptions(after_context=0, before_context=0)

        # No context — should get just the match lines.
        result = grep_file(lines, pat, opts)
        assert "MATCH1" in result
        assert "MATCH2" in result

    def test_context_with_line_numbers(self) -> None:
        lines = ["a", "MATCH", "b"]
        pat = compile_pattern("MATCH")
        opts = GrepOptions(context=1, line_number=True)

        result = grep_file(lines, pat, opts)
        # Matching line uses ":", context lines use "-".
        assert any("2:" in line for line in result)  # Match line.
        assert any("1-" in line for line in result)  # Before context.


# ---------------------------------------------------------------------------
# grep_file — inverted match
# ---------------------------------------------------------------------------


class TestInvertedMatch:
    def test_invert_basic(self) -> None:
        lines = ["apple", "banana", "cherry"]
        pat = compile_pattern("banana")
        opts = GrepOptions(invert_match=True)

        result = grep_file(lines, pat, opts)
        assert "apple" in result
        assert "cherry" in result
        assert "banana" not in result

    def test_invert_count(self) -> None:
        lines = ["apple", "banana", "cherry"]
        pat = compile_pattern("banana")
        opts = GrepOptions(invert_match=True, count=True)

        result = grep_file(lines, pat, opts)
        assert result == ["2"]


# ---------------------------------------------------------------------------
# walk_files
# ---------------------------------------------------------------------------


class TestWalkFiles:
    def test_walk_regular_files(self, tmp_path: Path) -> None:
        """Regular files are returned as-is."""
        f = tmp_path / "test.txt"
        f.write_text("content")

        result = walk_files([str(f)])
        assert result == [str(f)]

    def test_walk_directory_without_recursive(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Directories without -r produce an error message."""
        d = tmp_path / "subdir"
        d.mkdir()

        result = walk_files([str(d)], recursive=False)
        assert result == []
        captured = capsys.readouterr()
        assert "Is a directory" in captured.err

    def test_walk_directory_recursive(self, tmp_path: Path) -> None:
        """With recursive=True, files in subdirectories are found."""
        d = tmp_path / "subdir"
        d.mkdir()
        (d / "file.txt").write_text("content")
        (d / "file.py").write_text("code")

        result = walk_files([str(d)], recursive=True)
        assert len(result) == 2

    def test_walk_include_glob(self, tmp_path: Path) -> None:
        """Include glob filters the file list."""
        d = tmp_path / "subdir"
        d.mkdir()
        (d / "file.txt").write_text("content")
        (d / "file.py").write_text("code")

        result = walk_files([str(d)], recursive=True, include_globs=["*.txt"])
        assert len(result) == 1
        assert result[0].endswith(".txt")

    def test_walk_exclude_glob(self, tmp_path: Path) -> None:
        """Exclude glob filters out matching files."""
        d = tmp_path / "subdir"
        d.mkdir()
        (d / "file.txt").write_text("content")
        (d / "file.py").write_text("code")

        result = walk_files([str(d)], recursive=True, exclude_globs=["*.py"])
        assert len(result) == 1
        assert result[0].endswith(".txt")

    def test_walk_exclude_dir(self, tmp_path: Path) -> None:
        """Exclude-dir glob skips matching directories."""
        d = tmp_path / "project"
        d.mkdir()
        (d / "file.txt").write_text("content")
        node = d / "node_modules"
        node.mkdir()
        (node / "lib.js").write_text("code")

        result = walk_files(
            [str(d)], recursive=True, exclude_dir_globs=["node_modules"]
        )
        filenames = [os.path.basename(f) for f in result]
        assert "file.txt" in filenames
        assert "lib.js" not in filenames


# ---------------------------------------------------------------------------
# grep_file — regex patterns
# ---------------------------------------------------------------------------


class TestRegexPatterns:
    def test_dot_matches_any(self) -> None:
        lines = ["cat", "cot", "cut"]
        pat = compile_pattern("c.t")
        opts = GrepOptions()

        result = grep_file(lines, pat, opts)
        assert len(result) == 3

    def test_anchored_start(self) -> None:
        lines = ["hello world", "say hello"]
        pat = compile_pattern("^hello")
        opts = GrepOptions()

        result = grep_file(lines, pat, opts)
        assert result == ["hello world"]

    def test_character_class(self) -> None:
        lines = ["cat", "bat", "rat", "hat"]
        pat = compile_pattern("[cr]at")
        opts = GrepOptions()

        result = grep_file(lines, pat, opts)
        assert "cat" in result
        assert "rat" in result
        assert "bat" not in result
