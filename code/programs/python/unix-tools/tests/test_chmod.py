"""Tests for the chmod tool.

=== What These Tests Verify ===

These tests exercise the chmod implementation, including:

1. Octal mode parsing (755, 644, etc.)
2. Symbolic mode parsing (u+x, go-w, a=rwx, etc.)
3. Applying modes to files
4. Recursive mode changes (-R)
5. Verbose and changes-only reporting (-v, -c)
6. Error handling (missing files, permission errors)
7. Special modes (setuid, setgid, sticky bit)
8. Spec loading and CLI Builder integration
"""

from __future__ import annotations

import os
import stat
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "chmod.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from chmod_tool import apply_symbolic_mode, chmod_file, parse_octal_mode


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["chmod", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["chmod", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# Octal mode parsing
# ---------------------------------------------------------------------------


class TestOctalParsing:
    def test_common_modes(self) -> None:
        """Common octal modes parse correctly."""
        assert parse_octal_mode("755") == 0o755
        assert parse_octal_mode("644") == 0o644
        assert parse_octal_mode("777") == 0o777
        assert parse_octal_mode("000") == 0o000

    def test_four_digit_mode(self) -> None:
        """Four-digit modes (with setuid/setgid/sticky) parse correctly."""
        assert parse_octal_mode("1755") == 0o1755
        assert parse_octal_mode("4755") == 0o4755
        assert parse_octal_mode("2755") == 0o2755

    def test_single_digit(self) -> None:
        """Single-digit modes work."""
        assert parse_octal_mode("7") == 0o7

    def test_invalid_octal(self) -> None:
        """Non-octal strings return None."""
        assert parse_octal_mode("u+x") is None
        assert parse_octal_mode("abc") is None
        assert parse_octal_mode("999") is None

    def test_empty_string(self) -> None:
        """Empty string returns None."""
        assert parse_octal_mode("") is None


# ---------------------------------------------------------------------------
# Symbolic mode parsing
# ---------------------------------------------------------------------------


class TestSymbolicMode:
    def test_add_user_execute(self) -> None:
        """u+x adds execute permission for user."""
        result = apply_symbolic_mode("u+x", 0o644)
        assert result & stat.S_IXUSR

    def test_remove_group_write(self) -> None:
        """g-w removes write permission for group."""
        result = apply_symbolic_mode("g-w", 0o666)
        assert not (result & stat.S_IWGRP)

    def test_set_other_read_only(self) -> None:
        """o=r sets other to read-only."""
        result = apply_symbolic_mode("o=r", 0o777)
        # Other should have read only.
        assert result & stat.S_IROTH
        assert not (result & stat.S_IWOTH)
        assert not (result & stat.S_IXOTH)

    def test_all_read_write(self) -> None:
        """a+rw adds read and write for all."""
        result = apply_symbolic_mode("a+rw", 0o000)
        assert result & stat.S_IRUSR
        assert result & stat.S_IWUSR
        assert result & stat.S_IRGRP
        assert result & stat.S_IWGRP
        assert result & stat.S_IROTH
        assert result & stat.S_IWOTH

    def test_comma_separated(self) -> None:
        """Comma-separated clauses all apply."""
        result = apply_symbolic_mode("u+x,go-w", 0o666)
        assert result & stat.S_IXUSR          # u+x applied
        assert not (result & stat.S_IWGRP)    # g-w applied
        assert not (result & stat.S_IWOTH)    # o-w applied

    def test_no_who_defaults_to_all(self) -> None:
        """If no who is specified, it defaults to 'a' (all)."""
        result = apply_symbolic_mode("+x", 0o000)
        assert result & stat.S_IXUSR
        assert result & stat.S_IXGRP
        assert result & stat.S_IXOTH

    def test_capital_x_on_directory(self) -> None:
        """X adds execute only for directories."""
        result = apply_symbolic_mode("a+X", 0o644, is_directory=True)
        assert result & stat.S_IXUSR

    def test_capital_x_on_non_executable_file(self) -> None:
        """X does not add execute for files without existing execute."""
        result = apply_symbolic_mode("a+X", 0o644, is_directory=False)
        assert not (result & stat.S_IXUSR)

    def test_capital_x_on_executable_file(self) -> None:
        """X adds execute for files that already have some execute bit."""
        result = apply_symbolic_mode("a+X", 0o744, is_directory=False)
        assert result & stat.S_IXUSR
        assert result & stat.S_IXGRP

    def test_equals_clears_first(self) -> None:
        """= clears the who bits before setting."""
        result = apply_symbolic_mode("u=r", 0o777)
        # User should have only read.
        assert result & stat.S_IRUSR
        assert not (result & stat.S_IWUSR)
        assert not (result & stat.S_IXUSR)
        # Group and other should be unchanged.
        assert result & stat.S_IRWXG
        assert result & stat.S_IRWXO

    def test_setuid(self) -> None:
        """u+s sets the setuid bit."""
        result = apply_symbolic_mode("u+s", 0o755)
        assert result & stat.S_ISUID

    def test_sticky_bit(self) -> None:
        """a+t or +t sets the sticky bit."""
        result = apply_symbolic_mode("+t", 0o755)
        assert result & stat.S_ISVTX


# ---------------------------------------------------------------------------
# chmod_file -- applying modes to files
# ---------------------------------------------------------------------------


class TestChmodFile:
    def test_octal_mode(self, tmp_path: Path) -> None:
        """Applying an octal mode changes the file permissions."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        f.chmod(0o644)

        assert chmod_file(str(f), "755") is True
        actual = stat.S_IMODE(os.stat(str(f)).st_mode)
        assert actual == 0o755

    def test_symbolic_mode(self, tmp_path: Path) -> None:
        """Applying a symbolic mode modifies permissions."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        f.chmod(0o644)

        assert chmod_file(str(f), "u+x") is True
        actual = stat.S_IMODE(os.stat(str(f)).st_mode)
        assert actual & stat.S_IXUSR

    def test_missing_file(self, tmp_path: Path) -> None:
        """Changing mode of a nonexistent file returns False."""
        result = chmod_file(str(tmp_path / "nonexistent"), "755")
        assert result is False

    def test_verbose_output(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Verbose mode prints the mode change."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        f.chmod(0o644)

        chmod_file(str(f), "755", verbose=True)
        captured = capsys.readouterr()
        assert "mode of" in captured.out

    def test_changes_only_reports_actual_change(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Changes mode only reports when the mode actually changes."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        f.chmod(0o644)

        chmod_file(str(f), "755", changes=True)
        captured = capsys.readouterr()
        assert "mode of" in captured.out

    def test_changes_silent_when_no_change(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Changes mode is silent when the mode doesn't change."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        f.chmod(0o755)

        chmod_file(str(f), "755", changes=True)
        captured = capsys.readouterr()
        assert captured.out == ""

    def test_silent_suppresses_errors(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Silent mode suppresses error messages."""
        result = chmod_file(
            str(tmp_path / "nonexistent"), "755",
            silent=True,
        )
        assert result is False
        captured = capsys.readouterr()
        assert captured.err == ""


# ---------------------------------------------------------------------------
# Recursive chmod
# ---------------------------------------------------------------------------


class TestRecursiveChmod:
    def test_recursive_directory(self, tmp_path: Path) -> None:
        """Recursive mode changes permissions of directory contents."""
        d = tmp_path / "dir"
        d.mkdir()
        f1 = d / "file1.txt"
        f2 = d / "file2.txt"
        f1.write_text("content")
        f2.write_text("content")
        f1.chmod(0o644)
        f2.chmod(0o644)

        assert chmod_file(str(d), "755", recursive=True) is True

        assert stat.S_IMODE(os.stat(str(f1)).st_mode) == 0o755
        assert stat.S_IMODE(os.stat(str(f2)).st_mode) == 0o755

    def test_recursive_subdirectories(self, tmp_path: Path) -> None:
        """Recursive mode descends into subdirectories."""
        d = tmp_path / "dir"
        d.mkdir()
        sub = d / "sub"
        sub.mkdir()
        f = sub / "file.txt"
        f.write_text("content")
        f.chmod(0o644)

        assert chmod_file(str(d), "755", recursive=True) is True
        assert stat.S_IMODE(os.stat(str(f)).st_mode) == 0o755

    def test_non_recursive_skips_children(self, tmp_path: Path) -> None:
        """Without -R, directory children are not changed."""
        d = tmp_path / "dir"
        d.mkdir()
        f = d / "file.txt"
        f.write_text("content")
        f.chmod(0o644)

        chmod_file(str(d), "755", recursive=False)
        # The file inside should still have its original mode.
        assert stat.S_IMODE(os.stat(str(f)).st_mode) == 0o644
