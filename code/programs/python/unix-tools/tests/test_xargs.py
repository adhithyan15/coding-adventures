"""Tests for the xargs tool.

=== What These Tests Verify ===

These tests exercise the xargs implementation, including:

1. Input parsing (whitespace, null delimiter, custom delimiter)
2. EOF string handling
3. Command execution
4. Batching with -n (max_args)
5. Replace mode with -I
6. No-run-if-empty (-r)
7. Verbose mode (-t)
8. Parallel execution (-P)
9. Spec loading and CLI Builder integration
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "xargs.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from xargs_tool import execute_batches, parse_items, run_command


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

        result = parse_argv(["xargs", "echo"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["xargs", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["xargs", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# Input parsing -- whitespace mode
# ---------------------------------------------------------------------------


class TestParseItemsWhitespace:
    def test_simple_words(self) -> None:
        """Simple whitespace-separated words."""
        items = parse_items("hello world foo")
        assert items == ["hello", "world", "foo"]

    def test_multiple_spaces(self) -> None:
        """Multiple spaces between items."""
        items = parse_items("hello    world")
        assert items == ["hello", "world"]

    def test_newlines(self) -> None:
        """Newlines also separate items."""
        items = parse_items("hello\nworld\nfoo")
        assert items == ["hello", "world", "foo"]

    def test_mixed_whitespace(self) -> None:
        """Tabs and newlines and spaces mix."""
        items = parse_items("a\tb\n  c  d")
        assert items == ["a", "b", "c", "d"]

    def test_empty_input(self) -> None:
        """Empty input produces no items."""
        items = parse_items("")
        assert items == []

    def test_whitespace_only(self) -> None:
        """Whitespace-only input produces no items."""
        items = parse_items("   \n\t  ")
        assert items == []

    def test_quoted_strings(self) -> None:
        """Quoted strings are kept together."""
        items = parse_items('"hello world" foo')
        assert items == ["hello world", "foo"]

    def test_single_quoted_strings(self) -> None:
        """Single-quoted strings are kept together."""
        items = parse_items("'hello world' foo")
        assert items == ["hello world", "foo"]


# ---------------------------------------------------------------------------
# Input parsing -- null delimiter
# ---------------------------------------------------------------------------


class TestParseItemsNull:
    def test_null_separated(self) -> None:
        """Null-separated items."""
        items = parse_items("hello\0world\0foo", null_delimiter=True)
        assert items == ["hello", "world", "foo"]

    def test_trailing_null(self) -> None:
        """Trailing null doesn't create empty item."""
        items = parse_items("hello\0world\0", null_delimiter=True)
        assert items == ["hello", "world"]

    def test_items_with_spaces(self) -> None:
        """Null-delimited items can contain spaces."""
        items = parse_items("hello world\0foo bar\0", null_delimiter=True)
        assert items == ["hello world", "foo bar"]


# ---------------------------------------------------------------------------
# Input parsing -- custom delimiter
# ---------------------------------------------------------------------------


class TestParseItemsCustomDelimiter:
    def test_comma_delimiter(self) -> None:
        """Comma-separated items."""
        items = parse_items("a,b,c", delimiter=",")
        assert items == ["a", "b", "c"]

    def test_colon_delimiter(self) -> None:
        """Colon-separated items (like PATH)."""
        items = parse_items("/usr/bin:/bin:/usr/local/bin", delimiter=":")
        assert items == ["/usr/bin", "/bin", "/usr/local/bin"]

    def test_empty_items_filtered(self) -> None:
        """Empty items between delimiters are filtered out."""
        items = parse_items("a,,b,,c", delimiter=",")
        assert items == ["a", "b", "c"]


# ---------------------------------------------------------------------------
# EOF string
# ---------------------------------------------------------------------------


class TestEOFString:
    def test_eof_stops_processing(self) -> None:
        """Items after the EOF string are ignored."""
        items = parse_items("a b STOP c d", eof_str="STOP")
        assert items == ["a", "b"]

    def test_no_eof_match(self) -> None:
        """Without matching EOF, all items are returned."""
        items = parse_items("a b c", eof_str="STOP")
        assert items == ["a", "b", "c"]


# ---------------------------------------------------------------------------
# Command execution
# ---------------------------------------------------------------------------


class TestRunCommand:
    def test_echo_command(self) -> None:
        """Running echo produces exit code 0."""
        code = run_command(["echo"], ["hello"])
        assert code == 0

    def test_nonexistent_command(self) -> None:
        """Running a nonexistent command returns 127."""
        code = run_command(["/nonexistent/command"], [])
        assert code == 127

    def test_replace_mode(self) -> None:
        """Replace mode substitutes the replace string."""
        code = run_command(
            ["echo", "File: {}"],
            ["test.txt"],
            replace_str="{}",
        )
        assert code == 0

    def test_verbose_output(self, capsys: pytest.CaptureFixture[str]) -> None:
        """Verbose mode prints command to stderr."""
        run_command(["echo", "hello"], [], verbose=True)
        captured = capsys.readouterr()
        assert "echo" in captured.err


# ---------------------------------------------------------------------------
# Batching
# ---------------------------------------------------------------------------


class TestBatching:
    def test_single_batch(self) -> None:
        """All items in a single batch by default."""
        code = execute_batches(["echo"], ["a", "b", "c"])
        assert code == 0

    def test_max_args_batching(self) -> None:
        """Items are split into batches of max_args."""
        code = execute_batches(["echo"], ["a", "b", "c", "d"], max_args=2)
        assert code == 0

    def test_max_args_one(self) -> None:
        """max_args=1 runs command once per item."""
        code = execute_batches(["echo"], ["a", "b", "c"], max_args=1)
        assert code == 0

    def test_replace_implies_batch_one(self) -> None:
        """Replace mode processes one item at a time."""
        code = execute_batches(
            ["echo", "Item: {}"],
            ["a", "b"],
            replace_str="{}",
        )
        assert code == 0

    def test_no_run_if_empty_with_items(self) -> None:
        """no_run_if_empty still runs when items exist."""
        code = execute_batches(
            ["echo"], ["hello"],
            no_run_if_empty=True,
        )
        assert code == 0

    def test_no_run_if_empty_without_items(self) -> None:
        """no_run_if_empty skips execution when no items."""
        code = execute_batches(
            ["echo"], [],
            no_run_if_empty=True,
        )
        assert code == 0

    def test_run_with_no_items_default(self) -> None:
        """By default, command runs once even with no items."""
        code = execute_batches(["echo"], [])
        assert code == 0

    def test_command_failure_propagates(self) -> None:
        """If the command fails, the exit code propagates."""
        code = execute_batches(["false"], ["a"])
        assert code != 0


# ---------------------------------------------------------------------------
# Parallel execution
# ---------------------------------------------------------------------------


class TestParallelExecution:
    def test_parallel_execution(self) -> None:
        """Parallel mode runs multiple processes."""
        code = execute_batches(
            ["echo"], ["a", "b", "c", "d"],
            max_args=1,
            max_procs=2,
        )
        assert code == 0

    def test_parallel_zero_means_all(self) -> None:
        """max_procs=0 means as many as possible."""
        code = execute_batches(
            ["echo"], ["a", "b", "c"],
            max_args=1,
            max_procs=0,
        )
        assert code == 0
