"""Tests for the dirname tool.

=== What These Tests Verify ===

These tests exercise the dirname implementation, including:

1. Spec loading and basic parsing
2. Directory extraction from various path forms
3. The -z flag (NUL terminator)
4. CLI Builder integration (--help, --version)
5. Business logic (get_dirname) edge cases
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "dirname.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the dirname spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the dirname.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["dirname", "/usr/bin/python"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Test: Argument parsing
# ---------------------------------------------------------------------------


class TestArgumentParsing:
    """Verify that positional arguments are correctly parsed."""

    def test_single_name(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["dirname", "/usr/bin/python"])
        assert isinstance(result, ParseResult)
        names = result.arguments.get("names", [])
        if isinstance(names, str):
            assert names == "/usr/bin/python"
        else:
            assert "/usr/bin/python" in names


# ---------------------------------------------------------------------------
# Test: Zero flag
# ---------------------------------------------------------------------------


class TestZeroFlag:
    """The ``-z`` flag changes the terminator to NUL."""

    def test_z_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["dirname", "-z", "/path/file"])
        assert isinstance(result, ParseResult)
        assert result.flags["zero"] is True


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["dirname", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["dirname", "--help"])
        assert isinstance(result, HelpResult)
        assert "dirname" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["dirname", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["dirname", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — get_dirname
# ---------------------------------------------------------------------------


class TestGetDirname:
    """Test the get_dirname function directly."""

    def test_absolute_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("/usr/bin/python") == "/usr/bin"

    def test_bare_filename(self) -> None:
        """A bare filename has no directory, so dirname returns '.'."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("myfile.txt") == "."

    def test_trailing_slash(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("/usr/bin/") == "/usr"

    def test_root_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("/") == "/"

    def test_single_component(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("/usr") == "/"

    def test_relative_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("a/b/c") == "a/b"

    def test_dot_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname(".") == "."

    def test_double_dot_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from dirname_tool import get_dirname

        assert get_dirname("..") == "."
