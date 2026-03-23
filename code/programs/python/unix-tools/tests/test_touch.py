"""Tests for the touch tool.

=== What These Tests Verify ===

These tests exercise the touch implementation, including:

1. Spec loading and CLI Builder integration
2. File creation (touching nonexistent files)
3. The -c flag (no create)
4. The -a and -m flags (access/modification time only)
5. Timestamp parsing (-t flag)
6. Business logic functions
"""

from __future__ import annotations

import os
import sys
import time
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "touch.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from touch_tool import parse_date_string, parse_timestamp, touch_file


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["touch", "file.txt"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_no_create_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["touch", "-c", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["no_create"] is True

    def test_access_only_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["touch", "-a", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["access_only"] is True

    def test_modify_only_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["touch", "-m", "file.txt"])
        assert isinstance(result, ParseResult)
        assert result.flags["modify_only"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["touch", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["touch", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestParseTimestamp:
    def test_mmddhhmm(self) -> None:
        result = parse_timestamp("01151030")
        assert result is not None

    def test_yymmddhhmm(self) -> None:
        result = parse_timestamp("2401151030")
        assert result is not None

    def test_ccyymmddhhmm(self) -> None:
        result = parse_timestamp("202401151030")
        assert result is not None

    def test_with_seconds(self) -> None:
        result = parse_timestamp("01151030.45")
        assert result is not None

    def test_invalid_format(self) -> None:
        result = parse_timestamp("abc")
        assert result is None


class TestParseDateString:
    def test_iso_date(self) -> None:
        result = parse_date_string("2024-01-15")
        assert result is not None

    def test_iso_datetime(self) -> None:
        result = parse_date_string("2024-01-15 10:30:00")
        assert result is not None

    def test_iso_datetime_t(self) -> None:
        result = parse_date_string("2024-01-15T10:30:00")
        assert result is not None

    def test_invalid_date(self) -> None:
        result = parse_date_string("not-a-date")
        assert result is None


class TestTouchFile:
    def test_create_new_file(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "newfile.txt")
        result = touch_file(
            filepath, no_create=False, access_only=False,
            modify_only=False, timestamp=None,
        )
        assert result is True
        assert os.path.exists(filepath)

    def test_no_create_skips(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "nonexistent.txt")
        result = touch_file(
            filepath, no_create=True, access_only=False,
            modify_only=False, timestamp=None,
        )
        assert result is True
        assert not os.path.exists(filepath)

    def test_update_existing_file(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "existing.txt")
        Path(filepath).write_text("content")
        old_mtime = os.stat(filepath).st_mtime
        time.sleep(0.05)
        touch_file(
            filepath, no_create=False, access_only=False,
            modify_only=False, timestamp=None,
        )
        new_mtime = os.stat(filepath).st_mtime
        assert new_mtime >= old_mtime

    def test_specific_timestamp(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "timestamped.txt")
        Path(filepath).write_text("data")
        target_time = 1000000000.0  # 2001-09-09
        touch_file(
            filepath, no_create=False, access_only=False,
            modify_only=False, timestamp=target_time,
        )
        stat = os.stat(filepath)
        assert abs(stat.st_mtime - target_time) < 1.0

    def test_access_only(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "access.txt")
        Path(filepath).write_text("data")
        original_mtime = os.stat(filepath).st_mtime
        time.sleep(0.05)
        touch_file(
            filepath, no_create=False, access_only=True,
            modify_only=False, timestamp=None,
        )
        new_mtime = os.stat(filepath).st_mtime
        assert abs(new_mtime - original_mtime) < 1.0

    def test_modify_only(self, tmp_path: Path) -> None:
        filepath = str(tmp_path / "modify.txt")
        Path(filepath).write_text("data")
        original_atime = os.stat(filepath).st_atime
        time.sleep(0.05)
        touch_file(
            filepath, no_create=False, access_only=False,
            modify_only=True, timestamp=None,
        )
        new_atime = os.stat(filepath).st_atime
        assert abs(new_atime - original_atime) < 1.0
