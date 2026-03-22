"""Tests for the realpath tool.

=== What These Tests Verify ===

These tests exercise the realpath implementation, including:

1. Spec loading and CLI Builder integration
2. Basic path resolution
3. Symlink resolution
4. The -e flag (all must exist)
5. The -m flag (no component need exist)
6. The -s flag (no symlink resolution)
7. Relative path output (--relative-to, --relative-base)
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "realpath.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from realpath_tool import make_relative, resolve_path


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


class TestSpecLoading:
    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["realpath", "/tmp"])
        assert isinstance(result, ParseResult)


class TestFlags:
    def test_canonicalize_existing_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["realpath", "-e", "/tmp"])
        assert isinstance(result, ParseResult)
        assert result.flags["canonicalize_existing"] is True

    def test_canonicalize_missing_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["realpath", "-m", "/nonexistent"])
        assert isinstance(result, ParseResult)
        assert result.flags["canonicalize_missing"] is True

    def test_no_symlinks_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["realpath", "-s", "/tmp"])
        assert isinstance(result, ParseResult)
        assert result.flags["no_symlinks"] is True


class TestHelpVersion:
    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["realpath", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["realpath", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestResolvePath:
    def test_resolve_existing_path(self, tmp_path: Path) -> None:
        filepath = tmp_path / "file.txt"
        filepath.write_text("data")
        result = resolve_path(
            str(filepath),
            canonicalize_existing=False,
            canonicalize_missing=False,
            no_symlinks=False,
        )
        assert result is not None
        assert os.path.isabs(result)

    def test_resolve_symlink(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("data")
        link = tmp_path / "link.txt"
        os.symlink(str(target), str(link))
        result = resolve_path(
            str(link),
            canonicalize_existing=False,
            canonicalize_missing=False,
            no_symlinks=False,
        )
        assert result == str(target.resolve())

    def test_no_symlinks_mode(self, tmp_path: Path) -> None:
        target = tmp_path / "target.txt"
        target.write_text("data")
        link = tmp_path / "link.txt"
        os.symlink(str(target), str(link))
        result = resolve_path(
            str(link),
            canonicalize_existing=False,
            canonicalize_missing=False,
            no_symlinks=True,
        )
        # With -s, the symlink should not be resolved.
        assert result == os.path.abspath(str(link))

    def test_canonicalize_existing_fails(self) -> None:
        result = resolve_path(
            "/this/path/does/not/exist",
            canonicalize_existing=True,
            canonicalize_missing=False,
            no_symlinks=False,
        )
        assert result is None

    def test_canonicalize_missing_succeeds(self) -> None:
        result = resolve_path(
            "/this/path/does/not/exist",
            canonicalize_existing=False,
            canonicalize_missing=True,
            no_symlinks=False,
        )
        assert result is not None
        assert os.path.isabs(result)


class TestMakeRelative:
    def test_relative_to(self, tmp_path: Path) -> None:
        result = make_relative(
            str(tmp_path / "a" / "b" / "c"),
            relative_to=str(tmp_path / "a"),
            relative_base=None,
        )
        assert result == os.path.join("b", "c")

    def test_relative_base_under(self, tmp_path: Path) -> None:
        base = str(tmp_path)
        resolved = str(tmp_path / "sub" / "file.txt")
        result = make_relative(resolved, relative_to=None, relative_base=base)
        assert not os.path.isabs(result)

    def test_relative_base_outside(self, tmp_path: Path) -> None:
        base = str(tmp_path / "subdir")
        resolved = "/completely/different/path"
        result = make_relative(resolved, relative_to=None, relative_base=base)
        # Should remain absolute since it's outside the base.
        assert os.path.isabs(result)
