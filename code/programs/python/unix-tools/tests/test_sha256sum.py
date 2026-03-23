"""Tests for the sha256sum tool.

=== What These Tests Verify ===

These tests exercise the sha256sum implementation, including:

1. Spec loading and CLI Builder integration
2. compute_sha256 on files with known content
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

SPEC_FILE = str(Path(__file__).parent.parent / "sha256sum.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from sha256sum_tool import check_checksums, compute_sha256, format_checksum_line


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

        result = parse_argv(["sha256sum"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["sha256sum", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["sha256sum", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_check_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sha256sum", "-c", "file"])
        assert isinstance(result, ParseResult)
        assert result.flags["check"] is True

    def test_binary_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["sha256sum", "-b", "file"])
        assert isinstance(result, ParseResult)
        assert result.flags["binary"] is True


# ---------------------------------------------------------------------------
# compute_sha256 tests
# ---------------------------------------------------------------------------


class TestComputeSha256:
    def test_known_content(self) -> None:
        """Verify SHA-256 of known content."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"hello\n")
            fname = f.name

        try:
            result = compute_sha256(fname)
            expected = hashlib.sha256(b"hello\n").hexdigest()
            assert result == expected
        finally:
            os.unlink(fname)

    def test_empty_file(self) -> None:
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            fname = f.name

        try:
            result = compute_sha256(fname)
            expected = hashlib.sha256(b"").hexdigest()
            assert result == expected
        finally:
            os.unlink(fname)

    def test_returns_hex_string(self) -> None:
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"test")
            fname = f.name

        try:
            result = compute_sha256(fname)
            assert len(result) == 64
            assert all(c in "0123456789abcdef" for c in result)
        finally:
            os.unlink(fname)

    def test_nonexistent_file_raises(self) -> None:
        with pytest.raises(FileNotFoundError):
            compute_sha256("/nonexistent/file/path")

    def test_binary_content(self) -> None:
        data = bytes(range(256))
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(data)
            fname = f.name

        try:
            result = compute_sha256(fname)
            expected = hashlib.sha256(data).hexdigest()
            assert result == expected
        finally:
            os.unlink(fname)

    def test_large_file(self) -> None:
        data = b"y" * 100_000
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(data)
            fname = f.name

        try:
            result = compute_sha256(fname)
            expected = hashlib.sha256(data).hexdigest()
            assert result == expected
        finally:
            os.unlink(fname)

    def test_differs_from_md5(self) -> None:
        """SHA-256 should produce a different hash than MD5."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"test data")
            fname = f.name

        try:
            sha_hash = compute_sha256(fname)
            md5_hash = hashlib.md5(b"test data").hexdigest()  # noqa: S324
            assert sha_hash != md5_hash
            assert len(sha_hash) == 64  # SHA-256 is longer
            assert len(md5_hash) == 32  # MD5 is shorter
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
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"hello\n")
            data_file = f.name

        expected_hash = hashlib.sha256(b"hello\n").hexdigest()

        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".sha256",
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
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"hello\n")
            data_file = f.name

        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".sha256",
        ) as cf:
            cf.write(f"{'0' * 64}  {data_file}\n")
            check_file = cf.name

        try:
            failures, checked = check_checksums(check_file)
            assert checked == 1
            assert failures == 1
        finally:
            os.unlink(data_file)
            os.unlink(check_file)

    def test_missing_file(self) -> None:
        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".sha256",
        ) as cf:
            cf.write(f"{'0' * 64}  /nonexistent/file\n")
            check_file = cf.name

        try:
            failures, checked = check_checksums(check_file)
            assert failures == 1
        finally:
            os.unlink(check_file)

    def test_quiet_mode(self, capsys: pytest.CaptureFixture[str]) -> None:
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"test")
            data_file = f.name

        expected_hash = hashlib.sha256(b"test").hexdigest()
        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".sha256",
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

    def test_status_mode(self, capsys: pytest.CaptureFixture[str]) -> None:
        """Status mode suppresses all output."""
        with tempfile.NamedTemporaryFile(delete=False, mode="wb") as f:
            f.write(b"test")
            data_file = f.name

        expected_hash = hashlib.sha256(b"test").hexdigest()
        with tempfile.NamedTemporaryFile(
            delete=False, mode="w", suffix=".sha256",
        ) as cf:
            cf.write(f"{expected_hash}  {data_file}\n")
            check_file = cf.name

        try:
            check_checksums(check_file, status=True)
            captured = capsys.readouterr()
            assert captured.out == ""
        finally:
            os.unlink(data_file)
            os.unlink(check_file)
