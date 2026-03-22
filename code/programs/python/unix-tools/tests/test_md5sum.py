"""Tests for the md5sum tool.

=== What These Tests Verify ===

These tests exercise the md5sum implementation, including:

1. Spec loading and CLI Builder integration
2. compute_md5 on files with known content
3. format_checksum_line output format
4. check_checksums verification mode
5. Edge cases (empty files, binary mode indicator)
"""

from __future__ import annotations

import hashlib
import os
import sys
import tempfile
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "md5sum.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from md5sum_tool import check_checksums, compute_md5, format_checksum_line


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

        result = parse_argv(["md5sum"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["md5sum", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["md5sum", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_check_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["md5sum", "-c", "file"])
        assert isinstance(result, ParseResult)
        assert result.flags["check"] is True

    def test_binary_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["md5sum", "-b", "file"])
        assert isinstance(result, ParseResult)
        assert result.flags["binary"] is True


# ---------------------------------------------------------------------------
# compute_md5 tests
# ---------------------------------------------------------------------------


class TestComputeMd5:
    def test_known_content(self) -> None:
        """Verify MD5 of known content."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"hello\n")
            f.flush()
            fname = f.name

        try:
            result = compute_md5(fname)
            expected = hashlib.md5(b"hello\n").hexdigest()  # noqa: S324
            assert result == expected
        finally:
            os.unlink(fname)

    def test_empty_file(self) -> None:
        """MD5 of empty file is the MD5 of empty bytes."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            fname = f.name

        try:
            result = compute_md5(fname)
            expected = hashlib.md5(b"").hexdigest()  # noqa: S324
            assert result == expected
        finally:
            os.unlink(fname)

    def test_returns_hex_string(self) -> None:
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"test")
            fname = f.name

        try:
            result = compute_md5(fname)
            assert len(result) == 32
            assert all(c in "0123456789abcdef" for c in result)
        finally:
            os.unlink(fname)

    def test_nonexistent_file_raises(self) -> None:
        with pytest.raises(FileNotFoundError):
            compute_md5("/nonexistent/file/path")

    def test_binary_content(self) -> None:
        """Binary files should hash correctly."""
        data = bytes(range(256))
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(data)
            fname = f.name

        try:
            result = compute_md5(fname)
            expected = hashlib.md5(data).hexdigest()  # noqa: S324
            assert result == expected
        finally:
            os.unlink(fname)

    def test_large_file(self) -> None:
        """Verify chunked reading works for larger files."""
        data = b"x" * 100_000
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(data)
            fname = f.name

        try:
            result = compute_md5(fname)
            expected = hashlib.md5(data).hexdigest()  # noqa: S324
            assert result == expected
        finally:
            os.unlink(fname)


# ---------------------------------------------------------------------------
# format_checksum_line tests
# ---------------------------------------------------------------------------


class TestFormatChecksumLine:
    def test_text_mode(self) -> None:
        result = format_checksum_line("abc123", "file.txt")
        assert result == "abc123  file.txt"

    def test_binary_mode(self) -> None:
        result = format_checksum_line("abc123", "file.txt", binary=True)
        assert result == "abc123 *file.txt"

    def test_stdin_filename(self) -> None:
        result = format_checksum_line("abc123", "-")
        assert result == "abc123  -"


# ---------------------------------------------------------------------------
# check_checksums tests
# ---------------------------------------------------------------------------


class TestCheckChecksums:
    def test_valid_checksum(self) -> None:
        """Verify a file against its correct checksum."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"hello\n")
            data_file = f.name

        expected_hash = hashlib.md5(b"hello\n").hexdigest()  # noqa: S324

        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".md5",
        ) as cf:
            cf.write(f"{expected_hash}  {data_file}\n")
            check_file = cf.name

        try:
            failures, checked = check_checksums(check_file)
            assert checked == 1
            assert failures == 0
        finally:
            os.unlink(data_file)
            os.unlink(check_file)

    def test_invalid_checksum(self) -> None:
        """A wrong checksum should count as a failure."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"hello\n")
            data_file = f.name

        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".md5",
        ) as cf:
            cf.write(f"{'0' * 32}  {data_file}\n")
            check_file = cf.name

        try:
            failures, checked = check_checksums(check_file)
            assert checked == 1
            assert failures == 1
        finally:
            os.unlink(data_file)
            os.unlink(check_file)

    def test_missing_file(self) -> None:
        """A checksum for a missing file should count as failure."""
        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".md5",
        ) as cf:
            cf.write(f"{'0' * 32}  /nonexistent/file\n")
            check_file = cf.name

        try:
            failures, checked = check_checksums(check_file)
            assert failures == 1
        finally:
            os.unlink(check_file)

    def test_quiet_mode(self, capsys: pytest.CaptureFixture[str]) -> None:
        """Quiet mode should suppress OK messages."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"test")
            data_file = f.name

        expected_hash = hashlib.md5(b"test").hexdigest()  # noqa: S324
        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".md5",
        ) as cf:
            cf.write(f"{expected_hash}  {data_file}\n")
            check_file = cf.name

        try:
            check_checksums(check_file, quiet=True)
            captured = capsys.readouterr()
            assert "OK" not in captured.out
        finally:
            os.unlink(data_file)
            os.unlink(check_file)
