"""Tests for the comm tool.

=== What These Tests Verify ===

These tests exercise the comm implementation, including:

1. Spec loading and CLI Builder integration
2. Three-column comparison of sorted inputs
3. Column suppression (-1, -2, -3)
4. Custom output delimiter
5. Edge cases (empty inputs, identical files, disjoint files)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "comm.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from comm_tool import compare_sorted


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

        result = parse_argv(["comm", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["comm", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Three-column comparison tests
# ---------------------------------------------------------------------------


class TestCompareSorted:
    def test_basic_comparison(self) -> None:
        """Lines unique to file1, file2, and common."""
        lines1 = ["a", "b", "d"]
        lines2 = ["b", "c", "d"]
        result = compare_sorted(lines1, lines2)
        assert "a" in result[0]  # column 1 (unique to file1)
        assert "\tb" in result[1]  # column 3 (common, 2 tabs)
        assert "\tc" in result[2]  # column 2 (unique to file2)
        assert "\t\td" in result[3]  # column 3 (common)

    def test_all_common(self) -> None:
        lines = ["a", "b", "c"]
        result = compare_sorted(lines, lines)
        for line in result:
            assert line.startswith("\t\t")

    def test_completely_disjoint(self) -> None:
        lines1 = ["a", "c"]
        lines2 = ["b", "d"]
        result = compare_sorted(lines1, lines2)
        assert result == ["a", "\tb", "c", "\td"]

    def test_empty_file1(self) -> None:
        result = compare_sorted([], ["a", "b"])
        assert result == ["\ta", "\tb"]

    def test_empty_file2(self) -> None:
        result = compare_sorted(["a", "b"], [])
        assert result == ["a", "b"]

    def test_both_empty(self) -> None:
        assert compare_sorted([], []) == []


# ---------------------------------------------------------------------------
# Column suppression tests
# ---------------------------------------------------------------------------


class TestSuppression:
    def test_suppress_col1(self) -> None:
        """Suppress lines unique to file1."""
        lines1 = ["a", "b"]
        lines2 = ["b", "c"]
        result = compare_sorted(lines1, lines2, suppress=(True, False, False))
        # "a" should be suppressed. "b" is common, "c" is in file2.
        assert not any(line == "a" for line in result)

    def test_suppress_col2(self) -> None:
        lines1 = ["a", "b"]
        lines2 = ["b", "c"]
        result = compare_sorted(lines1, lines2, suppress=(False, True, False))
        # "c" (unique to file2) should be suppressed.
        assert not any("c" in line for line in result)

    def test_suppress_col3(self) -> None:
        lines1 = ["a", "b"]
        lines2 = ["b", "c"]
        result = compare_sorted(lines1, lines2, suppress=(False, False, True))
        # "b" (common) should be suppressed.
        assert len(result) == 2
        assert result[0] == "a"
        assert "\tc" in result[1]

    def test_suppress_12_shows_only_common(self) -> None:
        """Suppress columns 1 and 2 — only common lines remain."""
        lines1 = ["a", "b", "d"]
        lines2 = ["b", "c", "d"]
        result = compare_sorted(lines1, lines2, suppress=(True, True, False))
        assert len(result) == 2
        assert "b" in result[0]
        assert "d" in result[1]

    def test_suppress_all(self) -> None:
        lines1 = ["a", "b"]
        lines2 = ["b", "c"]
        result = compare_sorted(
            lines1, lines2, suppress=(True, True, True),
        )
        assert result == []


# ---------------------------------------------------------------------------
# Custom delimiter tests
# ---------------------------------------------------------------------------


class TestCustomDelimiter:
    def test_custom_output_delimiter(self) -> None:
        lines1 = ["a", "b"]
        lines2 = ["b", "c"]
        result = compare_sorted(
            lines1, lines2, output_delimiter="  ",
        )
        # Column 2 lines should have the custom delimiter.
        assert any("  c" in line for line in result)
        # Column 3 (common) should have two custom delimiters.
        assert any("    b" in line for line in result)


# ---------------------------------------------------------------------------
# Edge cases
# ---------------------------------------------------------------------------


class TestEdgeCases:
    def test_single_element_each(self) -> None:
        result = compare_sorted(["a"], ["a"])
        assert len(result) == 1
        assert result[0] == "\t\ta"

    def test_single_element_different(self) -> None:
        result = compare_sorted(["a"], ["b"])
        assert result == ["a", "\tb"]

    def test_many_duplicates_in_common(self) -> None:
        """Both files have the same repeated entries."""
        lines = ["a", "a", "b"]
        result = compare_sorted(lines, lines)
        assert len(result) == 3
