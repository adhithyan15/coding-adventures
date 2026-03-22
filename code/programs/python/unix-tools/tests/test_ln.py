"""Tests for the ln tool.

=== What These Tests Verify ===

These tests exercise the ln implementation, including:

1. Spec loading and CLI Builder integration
2. Hard link creation
3. Symbolic link creation (-s)
4. Force mode (-f)
5. Verbose output (-v)
6. Business logic function (make_link)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "ln.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from ln_tool import make_link


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ln", "target", "linkname"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_symbolic_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ln", "-s", "target", "link"])
        assert isinstance(result, ParseResult)
        assert result.flags["symbolic"] is True

    def test_force_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ln", "-f", "target", "link"])
        assert isinstance(result, ParseResult)
        assert result.flags["force"] is True

    def test_verbose_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["ln", "-v", "target", "link"])
        assert isinstance(result, ParseResult)
        assert result.flags["verbose"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["ln", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["ln", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestMakeLink:
    def test_hard_link(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("hello")
        link_name = str(tmp_path / "hardlink.txt")
        result = make_link(
            str(target), link_name,
            symbolic=False, force=False, verbose=False,
            relative=False, no_dereference=True,
        )
        assert result is True
        assert os.path.exists(link_name)
        assert os.stat(str(target)).st_ino == os.stat(link_name).st_ino

    def test_symbolic_link(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("hello")
        link_name = str(tmp_path / "symlink.txt")
        result = make_link(
            str(target), link_name,
            symbolic=True, force=False, verbose=False,
            relative=False, no_dereference=True,
        )
        assert result is True
        assert os.path.islink(link_name)

    def test_force_overwrites(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("hello")
        link_name = str(tmp_path / "existing.txt")
        Path(link_name).write_text("old")
        result = make_link(
            str(target), link_name,
            symbolic=True, force=True, verbose=False,
            relative=False, no_dereference=True,
        )
        assert result is True
        assert os.path.islink(link_name)

    def test_fails_if_exists(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("hello")
        link_name = str(tmp_path / "existing.txt")
        Path(link_name).write_text("old")
        result = make_link(
            str(target), link_name,
            symbolic=True, force=False, verbose=False,
            relative=False, no_dereference=True,
        )
        assert result is False

    def test_verbose_output(self, tmp_path: Path, capsys: pytest.CaptureFixture[str]) -> None:
        target = tmp_path / "target.txt"
        target.write_text("hello")
        link_name = str(tmp_path / "verboselink.txt")
        make_link(
            str(target), link_name,
            symbolic=True, force=False, verbose=True,
            relative=False, no_dereference=True,
        )
        captured = capsys.readouterr()
        assert "->" in captured.out

    def test_relative_symlink(self, tmp_path: Path) -> None:
        target = tmp_path / "subdir" / "target.txt"
        target.parent.mkdir()
        target.write_text("hello")
        link_name = str(tmp_path / "rellink.txt")
        result = make_link(
            str(target), link_name,
            symbolic=True, force=False, verbose=False,
            relative=True, no_dereference=True,
        )
        assert result is True
        assert os.path.islink(link_name)
        # The symlink target should be relative.
        link_target = os.readlink(link_name)
        assert not os.path.isabs(link_target)
