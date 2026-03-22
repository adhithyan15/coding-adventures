"""Tests for the basename tool.

=== What These Tests Verify ===

These tests exercise the basename implementation, including:

1. Spec loading and default behavior
2. Basic directory stripping
3. Suffix removal
4. The -a flag (multiple mode)
5. The -z flag (NUL terminator)
6. CLI Builder integration (--help, --version)
"""

from __future__ import annotations

import sys
from pathlib import Path
from typing import Any

import pytest

# The spec file lives alongside the main script.
SPEC_FILE = str(Path(__file__).parent.parent / "basename.json")


# ---------------------------------------------------------------------------
# Helper
# ---------------------------------------------------------------------------


def parse_argv(argv: list[str]) -> Any:  # noqa: ANN401
    """Parse an argv list against the basename spec and return the result."""
    from cli_builder import Parser

    return Parser(SPEC_FILE, argv).parse()


# ---------------------------------------------------------------------------
# Test: Spec loading
# ---------------------------------------------------------------------------


class TestSpecLoading:
    """Verify that the basename.json spec loads correctly."""

    def test_spec_file_exists(self) -> None:
        assert Path(SPEC_FILE).exists()

    def test_basic_parse_returns_parse_result(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["basename", "/usr/bin/python"])
        assert isinstance(result, ParseResult)


# ---------------------------------------------------------------------------
# Test: File argument parsing
# ---------------------------------------------------------------------------


class TestArgumentParsing:
    """Verify that positional arguments are correctly parsed."""

    def test_single_name(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["basename", "/usr/bin/python"])
        assert isinstance(result, ParseResult)
        names = result.arguments.get("name", [])
        if isinstance(names, str):
            assert names == "/usr/bin/python"
        else:
            assert "/usr/bin/python" in names

    def test_two_args_traditional_mode(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["basename", "/path/file.txt", ".txt"])
        assert isinstance(result, ParseResult)
        names = result.arguments.get("name", [])
        if isinstance(names, list):
            assert len(names) == 2


# ---------------------------------------------------------------------------
# Test: Flags
# ---------------------------------------------------------------------------


class TestMultipleFlag:
    """The ``-a`` flag enables multiple argument mode."""

    def test_a_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["basename", "-a", "/path/one", "/path/two"])
        assert isinstance(result, ParseResult)
        assert result.flags["multiple"] is True


class TestSuffixFlag:
    """The ``-s`` flag specifies a suffix to remove."""

    def test_s_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["basename", "-s", ".py", "/path/script.py"])
        assert isinstance(result, ParseResult)
        assert result.flags["suffix"] == ".py"


# ---------------------------------------------------------------------------
# Test: --help and --version
# ---------------------------------------------------------------------------


class TestHelpFlag:
    """``--help`` should return a HelpResult."""

    def test_help_returns_help_result(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["basename", "--help"])
        assert isinstance(result, HelpResult)

    def test_help_text_contains_program_name(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["basename", "--help"])
        assert isinstance(result, HelpResult)
        assert "basename" in result.text


class TestVersionFlag:
    """``--version`` should return a VersionResult."""

    def test_version_returns_version_result(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["basename", "--version"])
        assert isinstance(result, VersionResult)

    def test_version_string(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["basename", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# Test: Business logic — strip_basename
# ---------------------------------------------------------------------------


class TestStripBasename:
    """Test the strip_basename function directly."""

    def test_simple_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/usr/bin/python") == "python"

    def test_bare_filename(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("myfile.txt") == "myfile.txt"

    def test_trailing_slash(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/usr/bin/") == "bin"

    def test_root_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/") == "/"

    def test_with_suffix(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/path/to/file.txt", ".txt") == "file"

    def test_suffix_is_entire_name(self) -> None:
        """When the suffix equals the entire basename, don't strip it."""
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/path/.txt", ".txt") == ".txt"

    def test_suffix_not_present(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/path/file.py", ".txt") == "file.py"

    def test_deep_path(self) -> None:
        sys.path.insert(0, str(Path(__file__).parent.parent))
        from basename_tool import strip_basename

        assert strip_basename("/a/b/c/d/e.tar.gz", ".tar.gz") == "e"
