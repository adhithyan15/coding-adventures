"""Tests for the df tool.

=== What These Tests Verify ===

These tests exercise the df implementation, including:

1. Spec loading and CLI Builder integration
2. get_filesystem_info returns valid data
3. format_size helper function
4. format_df_output table formatting
5. Human-readable and SI modes
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "df.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from df_tool import format_df_output, format_size, get_filesystem_info


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

        result = parse_argv(["df"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["df", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["df", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_human_readable_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["df", "-h"])
        assert isinstance(result, ParseResult)
        assert result.flags["human_readable"] is True

    def test_si_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["df", "-H"])
        assert isinstance(result, ParseResult)
        assert result.flags["si"] is True


# ---------------------------------------------------------------------------
# format_size tests
# ---------------------------------------------------------------------------


class TestFormatSize:
    def test_default_1k_blocks(self) -> None:
        """Default mode returns 1K blocks."""
        assert format_size(1024) == "1"
        assert format_size(2048) == "2"

    def test_zero(self) -> None:
        assert format_size(0) == "0"

    def test_human_readable_bytes(self) -> None:
        result = format_size(500, human=True)
        assert result == "500"

    def test_human_readable_kilobytes(self) -> None:
        result = format_size(1024, human=True)
        assert "K" in result

    def test_human_readable_megabytes(self) -> None:
        result = format_size(1024 * 1024 * 5, human=True)
        assert "M" in result

    def test_human_readable_gigabytes(self) -> None:
        result = format_size(1024 ** 3 * 2, human=True)
        assert "G" in result

    def test_si_mode(self) -> None:
        result = format_size(1000, si=True)
        assert "K" in result

    def test_human_zero(self) -> None:
        assert format_size(0, human=True) == "0"


# ---------------------------------------------------------------------------
# get_filesystem_info tests
# ---------------------------------------------------------------------------


class TestGetFilesystemInfo:
    def test_root_filesystem(self) -> None:
        entries = get_filesystem_info(["/"])
        assert len(entries) >= 1
        entry = entries[0]
        assert "total" in entry
        assert "used" in entry
        assert "available" in entry
        assert "use_percent" in entry

    def test_total_greater_than_zero(self) -> None:
        entries = get_filesystem_info(["/"])
        assert entries[0]["total"] > 0

    def test_used_is_nonnegative(self) -> None:
        entries = get_filesystem_info(["/"])
        assert entries[0]["used"] >= 0

    def test_available_is_nonnegative(self) -> None:
        entries = get_filesystem_info(["/"])
        assert entries[0]["available"] >= 0

    def test_default_paths(self) -> None:
        """With no paths, defaults to root."""
        entries = get_filesystem_info()
        assert len(entries) >= 1

    def test_current_directory(self) -> None:
        entries = get_filesystem_info(["."])
        assert len(entries) == 1

    def test_use_percent_has_percent_sign(self) -> None:
        entries = get_filesystem_info(["/"])
        assert "%" in str(entries[0]["use_percent"])


# ---------------------------------------------------------------------------
# format_df_output tests
# ---------------------------------------------------------------------------


class TestFormatDfOutput:
    def test_output_has_header(self) -> None:
        entries = [
            {
                "filesystem": "/dev/sda1",
                "total": 1024 * 1024 * 100,
                "used": 1024 * 1024 * 50,
                "available": 1024 * 1024 * 50,
                "use_percent": "50%",
                "mounted_on": "/",
            },
        ]
        output = format_df_output(entries)
        lines = output.split("\n")
        assert "Filesystem" in lines[0]

    def test_output_has_data_row(self) -> None:
        entries = [
            {
                "filesystem": "/dev/sda1",
                "total": 1024 * 1024 * 100,
                "used": 1024 * 1024 * 50,
                "available": 1024 * 1024 * 50,
                "use_percent": "50%",
                "mounted_on": "/",
            },
        ]
        output = format_df_output(entries)
        lines = output.split("\n")
        assert len(lines) >= 2

    def test_human_readable_output(self) -> None:
        entries = [
            {
                "filesystem": "/dev/sda1",
                "total": 1024 ** 3,
                "used": 512 * 1024 * 1024,
                "available": 512 * 1024 * 1024,
                "use_percent": "50%",
                "mounted_on": "/",
            },
        ]
        output = format_df_output(entries, human=True)
        assert "Size" in output  # Human-readable header
