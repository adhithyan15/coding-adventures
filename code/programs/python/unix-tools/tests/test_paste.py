"""Tests for the paste tool.

=== What These Tests Verify ===

These tests exercise the paste implementation, including:

1. Spec loading and CLI Builder integration
2. Parallel mode (default)
3. Serial mode (-s)
4. Custom delimiters (-d)
5. Delimiter cycling
6. Files of different lengths
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "paste.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from paste_tool import paste_parallel, paste_serial


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# CLI Builder integration tests
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["paste", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["paste", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_serial_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["paste", "-s", "file1"])
        assert isinstance(result, ParseResult)
        assert result.flags["serial"] is True

    def test_delimiter_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["paste", "-d", ",", "file1"])
        assert isinstance(result, ParseResult)
        assert result.flags["delimiters"] == ","


# ---------------------------------------------------------------------------
# Parallel mode tests
# ---------------------------------------------------------------------------


class TestPasteParallel:
    def test_two_files_same_length(self) -> None:
        file1 = ["a1", "a2", "a3"]
        file2 = ["b1", "b2", "b3"]
        result = paste_parallel([file1, file2])
        assert result == ["a1\tb1", "a2\tb2", "a3\tb3"]

    def test_three_files(self) -> None:
        f1 = ["a"]
        f2 = ["b"]
        f3 = ["c"]
        result = paste_parallel([f1, f2, f3])
        assert result == ["a\tb\tc"]

    def test_different_lengths(self) -> None:
        """Shorter files contribute empty strings."""
        file1 = ["a1", "a2", "a3"]
        file2 = ["b1"]
        result = paste_parallel([file1, file2])
        assert result == ["a1\tb1", "a2\t", "a3\t"]

    def test_custom_delimiter(self) -> None:
        file1 = ["a1", "a2"]
        file2 = ["b1", "b2"]
        result = paste_parallel([file1, file2], delimiters=",")
        assert result == ["a1,b1", "a2,b2"]

    def test_delimiter_cycling(self) -> None:
        """With multiple files, delimiters cycle."""
        f1 = ["a"]
        f2 = ["b"]
        f3 = ["c"]
        result = paste_parallel([f1, f2, f3], delimiters=",:")
        assert result == ["a,b:c"]

    def test_single_file(self) -> None:
        file1 = ["a1", "a2"]
        result = paste_parallel([file1])
        assert result == ["a1", "a2"]

    def test_empty_files(self) -> None:
        result = paste_parallel([[], []])
        assert result == []

    def test_empty_input(self) -> None:
        result = paste_parallel([])
        assert result == []


# ---------------------------------------------------------------------------
# Serial mode tests
# ---------------------------------------------------------------------------


class TestPasteSerial:
    def test_single_file(self) -> None:
        file1 = ["a1", "a2", "a3"]
        result = paste_serial([file1])
        assert result == ["a1\ta2\ta3"]

    def test_two_files(self) -> None:
        file1 = ["a1", "a2"]
        file2 = ["b1", "b2", "b3"]
        result = paste_serial([file1, file2])
        assert result == ["a1\ta2", "b1\tb2\tb3"]

    def test_custom_delimiter(self) -> None:
        file1 = ["x", "y", "z"]
        result = paste_serial([file1], delimiters=",")
        assert result == ["x,y,z"]

    def test_empty_file(self) -> None:
        result = paste_serial([[]])
        assert result == [""]

    def test_single_line_file(self) -> None:
        result = paste_serial([["only"]])
        assert result == ["only"]

    def test_delimiter_cycling_serial(self) -> None:
        """Delimiters cycle within a single file in serial mode."""
        file1 = ["a", "b", "c", "d"]
        result = paste_serial([file1], delimiters=",:")
        assert result == ["a,b:c,d"]
