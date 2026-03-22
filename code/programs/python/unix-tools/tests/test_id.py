"""Tests for the id tool.

=== What These Tests Verify ===

These tests exercise the id implementation, including:

1. Spec loading and CLI Builder integration
2. get_user_info returns expected structure
3. format_id_default produces correct format
4. Cross-platform compatibility
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any
from unittest.mock import patch

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "id.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from id_tool import format_id_default, get_user_info


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

        result = parse_argv(["id"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["id", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["id", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


class TestFlags:
    def test_user_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["id", "-u"])
        assert isinstance(result, ParseResult)
        assert result.flags["user"] is True

    def test_group_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["id", "-g"])
        assert isinstance(result, ParseResult)
        assert result.flags["group"] is True

    def test_groups_flag(self) -> None:
        from cli_builder import ParseResult

        result = parse_argv(["id", "-G"])
        assert isinstance(result, ParseResult)
        assert result.flags["groups"] is True


# ---------------------------------------------------------------------------
# get_user_info tests
# ---------------------------------------------------------------------------


class TestGetUserInfo:
    def test_returns_dict(self) -> None:
        info = get_user_info()
        assert isinstance(info, dict)

    def test_has_required_fields(self) -> None:
        info = get_user_info()
        for field in ["uid", "gid", "username", "groupname", "groups", "euid", "egid"]:
            assert field in info, f"Missing field: {field}"

    def test_uid_is_int(self) -> None:
        info = get_user_info()
        assert isinstance(info["uid"], int)

    def test_gid_is_int(self) -> None:
        info = get_user_info()
        assert isinstance(info["gid"], int)

    def test_username_is_string(self) -> None:
        info = get_user_info()
        assert isinstance(info["username"], str)
        assert len(info["username"]) > 0

    def test_groups_is_list(self) -> None:
        info = get_user_info()
        assert isinstance(info["groups"], list)
        assert len(info["groups"]) > 0

    def test_groups_are_tuples(self) -> None:
        info = get_user_info()
        for entry in info["groups"]:
            assert isinstance(entry, tuple)
            assert len(entry) == 2
            assert isinstance(entry[0], int)
            assert isinstance(entry[1], str)

    def test_euid_matches_os(self) -> None:
        """On Unix, euid should match os.geteuid()."""
        if not hasattr(os, "geteuid"):
            pytest.skip("Not on Unix")
        info = get_user_info()
        assert info["euid"] == os.geteuid()

    def test_nonexistent_user_raises(self) -> None:
        if not hasattr(os, "getuid"):
            pytest.skip("Not on Unix")
        with pytest.raises(KeyError):
            get_user_info("nonexistent_user_12345")


# ---------------------------------------------------------------------------
# format_id_default tests
# ---------------------------------------------------------------------------


class TestFormatIdDefault:
    def test_format_contains_uid(self) -> None:
        info = {
            "uid": 1000,
            "gid": 1000,
            "username": "alice",
            "groupname": "alice",
            "groups": [(1000, "alice")],
            "euid": 1000,
            "egid": 1000,
        }
        result = format_id_default(info)
        assert "uid=1000(alice)" in result

    def test_format_contains_gid(self) -> None:
        info = {
            "uid": 1000,
            "gid": 1000,
            "username": "alice",
            "groupname": "alice",
            "groups": [(1000, "alice")],
            "euid": 1000,
            "egid": 1000,
        }
        result = format_id_default(info)
        assert "gid=1000(alice)" in result

    def test_format_contains_groups(self) -> None:
        info = {
            "uid": 1000,
            "gid": 1000,
            "username": "alice",
            "groupname": "alice",
            "groups": [(1000, "alice"), (27, "sudo")],
            "euid": 1000,
            "egid": 1000,
        }
        result = format_id_default(info)
        assert "groups=1000(alice),27(sudo)" in result

    def test_format_full(self) -> None:
        info = {
            "uid": 1000,
            "gid": 1000,
            "username": "bob",
            "groupname": "bob",
            "groups": [(1000, "bob")],
            "euid": 1000,
            "egid": 1000,
        }
        result = format_id_default(info)
        assert result == "uid=1000(bob) gid=1000(bob) groups=1000(bob)"

    def test_current_user_format(self) -> None:
        """The actual current user's info should produce valid output."""
        info = get_user_info()
        result = format_id_default(info)
        assert result.startswith("uid=")
        assert "gid=" in result
        assert "groups=" in result
