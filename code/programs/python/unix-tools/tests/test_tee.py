"""Tests for the tee tool.

=== What These Tests Verify ===

These tests exercise the tee implementation, including:

1. Spec loading and default behavior
2. The -a flag (append mode)
3. The -i flag (ignore interrupts)
4. File writing via tee_copy
5. CLI Builder integration (--help, --version)
6. Business logic function (tee_copy)
"""

from __future__ import annotations

import io
import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "tee.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the tee spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the tee.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_no_flags_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tee"])
        assert isinstance(result, ParseResult)

    def test_with_file_argument(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tee", "output.txt"])
        assert isinstance(result, ParseResult)
        files = result.arguments.get("files", [])
        if isinstance(files, str):
            assert files == "output.txt"
        else:
            assert "output.txt" in files


# ---------------------------------------------------------------------------
# Test: -a flag (append)
# ---------------------------------------------------------------------------


class TestAppendFlag:
    """The ``-a`` flag enables append mode."""

    def test_a_flag_short(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tee", "-a"])
        assert isinstance(result, ParseResult)
        assert result.flags["append"] is True

    def test_a_flag_long(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tee", "--append"])
        assert isinstance(result, ParseResult)
        assert result.flags["append"] is True


# ---------------------------------------------------------------------------
# Test: -i flag (ignore interrupts)
# ---------------------------------------------------------------------------


class TestIgnoreInterruptsFlag:
    """The ``-i`` flag tells tee to ignore SIGINT."""

    def test_i_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["tee", "-i"])
        assert isinstance(result, ParseResult)
        assert result.flags["ignore_interrupts"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tee", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["tee", "--help"])
        assert isinstance(result, HelpResult)
        assert "tee" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tee", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["tee", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — tee_copy
# ---------------------------------------------------------------------------


class TestTeeCopy:
    """Test the tee_copy function directly."""

    def test_copy_to_stdout_only(self, capsys: pytest.CaptureFixture[str]) -> None:
        """With no output files, tee_copy should just write to stdout."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tee_tool import tee_copy

        input_data = b"Hello, World!\n"
        input_stream = io.BytesIO(input_data)
        tee_copy(input_stream, [])
        captured = capsys.readouterr()
        assert captured.out == "Hello, World!\n"

    def test_copy_to_file(self, tmp_path: Path) -> None:
        """tee_copy should write to both stdout and the output file."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tee_tool import tee_copy

        input_data = b"test content\n"
        input_stream = io.BytesIO(input_data)
        output_file = tmp_path / "output.txt"

        with open(output_file, "wb") as f:
            tee_copy(input_stream, [f])

        assert output_file.read_bytes() == b"test content\n"

    def test_copy_to_multiple_files(self, tmp_path: Path) -> None:
        """tee_copy should write to multiple output files."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tee_tool import tee_copy

        input_data = b"multi-file test\n"
        input_stream = io.BytesIO(input_data)
        file1 = tmp_path / "out1.txt"
        file2 = tmp_path / "out2.txt"

        with open(file1, "wb") as f1, open(file2, "wb") as f2:
            tee_copy(input_stream, [f1, f2])

        assert file1.read_bytes() == b"multi-file test\n"
        assert file2.read_bytes() == b"multi-file test\n"

    def test_empty_input(self, tmp_path: Path) -> None:
        """Empty input should produce empty output."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from tee_tool import tee_copy

        input_stream = io.BytesIO(b"")
        output_file = tmp_path / "empty.txt"

        with open(output_file, "wb") as f:
            tee_copy(input_stream, [f])

        assert output_file.read_bytes() == b""
