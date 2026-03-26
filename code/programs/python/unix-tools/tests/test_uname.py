"""Tests for the uname tool.

=== What These Tests Verify ===

These tests exercise the uname implementation, including:

1. Spec loading and CLI Builder integration
2. get_system_info returns all expected fields
3. format_uname with various flag combinations
4. Default behavior (kernel name only)
5. The -a flag (all fields)
"""

from __future__ import annotations

import platform
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "uname.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from uname_tool import format_uname, get_system_info


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

        result = parse_argv(["uname"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["uname", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["uname", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_all_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uname", "-a"])
        assert isinstance(result, ParseResult)
        assert result.flags["all"] is True

    def test_kernel_name_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uname", "-s"])
        assert isinstance(result, ParseResult)
        assert result.flags["kernel_name"] is True

    def test_machine_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["uname", "-m"])
        assert isinstance(result, ParseResult)
        assert result.flags["machine"] is True

    def test_unknown_flag_raises(self) -> None:
        from cli_builder import ParseErrors

        with pytest.raises(ParseErrors):
            parse_argv(["uname", "--nonexistent"])


# ---------------------------------------------------------------------------
# get_system_info tests
# ---------------------------------------------------------------------------


class TestGetSystemInfo:
    def test_returns_dict(self) -> None:
        info = get_system_info()
        assert isinstance(info, dict)

    def test_has_all_fields(self) -> None:
        info = get_system_info()
        expected_fields = [
            "kernel_name", "nodename", "kernel_release",
            "kernel_version", "machine", "processor",
            "hardware_platform", "operating_system",
        ]
        for field in expected_fields:
            assert field in info, f"Missing field: {field}"

    def test_kernel_name_matches_platform(self) -> None:
        info = get_system_info()
        assert info["kernel_name"] == platform.system()

    def test_machine_matches_platform(self) -> None:
        info = get_system_info()
        assert info["machine"] == platform.machine()

    def test_all_values_are_strings(self) -> None:
        info = get_system_info()
        for key, value in info.items():
            assert isinstance(value, str), f"{key} is {type(value)}, not str"

    def test_no_empty_values(self) -> None:
        info = get_system_info()
        for key, value in info.items():
            assert len(value) > 0, f"{key} is empty"


# ---------------------------------------------------------------------------
# format_uname tests
# ---------------------------------------------------------------------------


class TestFormatUname:
    def setup_method(self) -> None:
        """Create a predictable info dict for testing."""
        self.info = {
            "kernel_name": "Linux",
            "nodename": "myhost",
            "kernel_release": "5.15.0",
            "kernel_version": "#1 SMP",
            "machine": "x86_64",
            "processor": "x86_64",
            "hardware_platform": "x86_64",
            "operating_system": "GNU/Linux",
        }

    def test_default_shows_kernel_name(self) -> None:
        result = format_uname(self.info)
        assert result == "Linux"

    def test_kernel_name_flag(self) -> None:
        result = format_uname(self.info, show_kernel_name=True)
        assert result == "Linux"

    def test_nodename_flag(self) -> None:
        result = format_uname(self.info, show_nodename=True)
        assert result == "myhost"

    def test_machine_flag(self) -> None:
        result = format_uname(self.info, show_machine=True)
        assert result == "x86_64"

    def test_multiple_flags(self) -> None:
        result = format_uname(
            self.info, show_kernel_name=True, show_machine=True,
        )
        assert result == "Linux x86_64"

    def test_all_flag(self) -> None:
        result = format_uname(self.info, show_all=True)
        # All 8 fields should appear, separated by spaces. Note that
        # kernel_version "#1 SMP" itself contains a space, so we check
        # for key substrings rather than splitting.
        assert result.startswith("Linux myhost 5.15.0")
        assert result.endswith("GNU/Linux")
        assert "x86_64" in result
        assert "#1 SMP" in result

    def test_operating_system_flag(self) -> None:
        result = format_uname(self.info, show_operating_system=True)
        assert result == "GNU/Linux"

    def test_kernel_release_flag(self) -> None:
        result = format_uname(self.info, show_kernel_release=True)
        assert result == "5.15.0"
