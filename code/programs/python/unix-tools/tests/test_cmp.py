"""Tests for the cmp tool.

=== What These Tests Verify ===

These tests exercise the cmp implementation, including:

1. Byte-by-byte comparison of identical files
2. Detection of first difference
3. Verbose mode (list all differences)
4. Silent mode (exit code only)
5. Print bytes mode
6. Skip and max_bytes options
7. EOF handling (files of different lengths)
8. Spec loading and CLI Builder integration
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

SPEC_FILE = str(Path(__file__).parent.parent / "cmp.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from cmp_tool import cmp_files


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

        result = parse_argv(["cmp", "file1", "file2"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["cmp", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["cmp", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# Identical files
# ---------------------------------------------------------------------------


class TestIdenticalFiles:
    def test_identical_text(self, tmp_path: Path) -> None:
        """Identical text files return exit code 0."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello world\n")
        f2.write_text("hello world\n")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 0
        assert output == []

    def test_identical_binary(self, tmp_path: Path) -> None:
        """Identical binary files return exit code 0."""
        f1 = tmp_path / "a.bin"
        f2 = tmp_path / "b.bin"
        data = bytes(range(256))
        f1.write_bytes(data)
        f2.write_bytes(data)

        code, output = cmp_files(str(f1), str(f2))
        assert code == 0
        assert output == []

    def test_empty_files(self, tmp_path: Path) -> None:
        """Two empty files are identical."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_bytes(b"")
        f2.write_bytes(b"")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 0


# ---------------------------------------------------------------------------
# Different files -- default mode
# ---------------------------------------------------------------------------


class TestDifferentFiles:
    def test_first_byte_differs(self, tmp_path: Path) -> None:
        """Report the first byte that differs."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello")
        f2.write_text("Hello")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 1
        assert len(output) == 1
        assert "byte 1" in output[0]
        assert "line 1" in output[0]

    def test_difference_on_second_line(self, tmp_path: Path) -> None:
        """Line numbers track newlines in the first file."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("same\ndifferent\n")
        f2.write_text("same\nDifferent\n")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 1
        assert "line 2" in output[0]

    def test_difference_at_offset(self, tmp_path: Path) -> None:
        """The byte offset is correctly reported."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_bytes(b"abc\x00xyz")
        f2.write_bytes(b"abc\x00XYZ")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 1
        assert "byte 5" in output[0]


# ---------------------------------------------------------------------------
# Verbose mode (-l)
# ---------------------------------------------------------------------------


class TestVerboseMode:
    def test_lists_all_differences(self, tmp_path: Path) -> None:
        """Verbose mode lists all byte differences, not just the first."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("abcd")
        f2.write_text("ABCD")

        code, output = cmp_files(str(f1), str(f2), verbose=True)
        assert code == 1
        # Should list 4 differences (a vs A, b vs B, c vs C, d vs D).
        assert len(output) == 4

    def test_verbose_octal_values(self, tmp_path: Path) -> None:
        """Verbose mode shows octal byte values."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_bytes(b"a")  # 0o141
        f2.write_bytes(b"b")  # 0o142

        code, output = cmp_files(str(f1), str(f2), verbose=True)
        assert code == 1
        assert "141" in output[0]
        assert "142" in output[0]


# ---------------------------------------------------------------------------
# Silent mode (-s)
# ---------------------------------------------------------------------------


class TestSilentMode:
    def test_silent_identical(self, tmp_path: Path) -> None:
        """Silent mode produces no output for identical files."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("same")
        f2.write_text("same")

        code, output = cmp_files(str(f1), str(f2), silent=True)
        assert code == 0
        assert output == []

    def test_silent_different(self, tmp_path: Path) -> None:
        """Silent mode produces no output for different files."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello")
        f2.write_text("world")

        code, output = cmp_files(str(f1), str(f2), silent=True)
        assert code == 1
        assert output == []

    def test_silent_missing_file(self, tmp_path: Path) -> None:
        """Silent mode produces no output for missing files."""
        f1 = tmp_path / "a.txt"
        f1.write_text("hello")

        code, output = cmp_files(str(f1), str(tmp_path / "nonexistent"), silent=True)
        assert code == 2
        assert output == []


# ---------------------------------------------------------------------------
# Print bytes mode (-b)
# ---------------------------------------------------------------------------


class TestPrintBytesMode:
    def test_shows_characters(self, tmp_path: Path) -> None:
        """Print bytes mode shows character representations."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("a")
        f2.write_text("b")

        code, output = cmp_files(str(f1), str(f2), print_bytes=True)
        assert code == 1
        assert "a" in output[0]
        assert "b" in output[0]


# ---------------------------------------------------------------------------
# Skip and max_bytes
# ---------------------------------------------------------------------------


class TestSkipAndLimit:
    def test_skip_bytes(self, tmp_path: Path) -> None:
        """Skipping bytes ignores differences before the skip offset."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("XXXhello")
        f2.write_text("YYYhello")

        code, output = cmp_files(str(f1), str(f2), skip=3)
        assert code == 0  # After skipping, files are identical.

    def test_skip_past_difference(self, tmp_path: Path) -> None:
        """Skipping past differences makes files appear identical."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("AB")
        f2.write_text("CD")

        code, _ = cmp_files(str(f1), str(f2), skip=2)
        assert code == 0

    def test_max_bytes_limit(self, tmp_path: Path) -> None:
        """Max bytes limits comparison to first N bytes."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("helloXXX")
        f2.write_text("helloYYY")

        code, output = cmp_files(str(f1), str(f2), max_bytes=5)
        assert code == 0  # First 5 bytes are identical.

    def test_max_bytes_detects_difference(self, tmp_path: Path) -> None:
        """Max bytes still detects differences within the limit."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("AXXXX")
        f2.write_text("BXXXX")

        code, output = cmp_files(str(f1), str(f2), max_bytes=5)
        assert code == 1


# ---------------------------------------------------------------------------
# EOF handling
# ---------------------------------------------------------------------------


class TestEOFHandling:
    def test_shorter_file1(self, tmp_path: Path) -> None:
        """When file1 is shorter, report EOF on file1."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello")
        f2.write_text("hello world")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 1
        assert any("EOF" in line for line in output)

    def test_shorter_file2(self, tmp_path: Path) -> None:
        """When file2 is shorter, report EOF on file2."""
        f1 = tmp_path / "a.txt"
        f2 = tmp_path / "b.txt"
        f1.write_text("hello world")
        f2.write_text("hello")

        code, output = cmp_files(str(f1), str(f2))
        assert code == 1
        assert any("EOF" in line for line in output)


# ---------------------------------------------------------------------------
# Error handling
# ---------------------------------------------------------------------------


class TestErrorHandling:
    def test_missing_file(self, tmp_path: Path) -> None:
        """Missing file returns exit code 2."""
        f1 = tmp_path / "a.txt"
        f1.write_text("hello")

        code, output = cmp_files(str(f1), str(tmp_path / "nonexistent"))
        assert code == 2
        assert any("No such file" in line for line in output)

    def test_both_files_missing(self, tmp_path: Path) -> None:
        """Both files missing returns exit code 2."""
        code, output = cmp_files(
            str(tmp_path / "nope1"),
            str(tmp_path / "nope2"),
        )
        assert code == 2
