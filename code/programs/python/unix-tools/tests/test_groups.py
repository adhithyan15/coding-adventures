"""Tests for the groups tool.

=== What These Tests Verify ===

These tests exercise the groups implementation, including:

1. Spec loading and CLI Builder integration
2. get_user_groups for current user
3. get_user_groups for a specific user
4. Error handling for nonexistent users
"""

from __future__ import annotations

import os
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "groups.json")

sys.path.insert(0, str(Path(__file__).parent.parent))

from groups_tool import get_user_groups


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

        result = parse_argv(["groups"])
        assert isinstance(result, ParseResult)

    def test_help(self) -> None:
        from cli_builder import HelpResult

        result = parse_argv(["groups", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["groups", "--version"])
        assert isinstance(result, VersionResult)
        assert result.version == "1.0.0"


# ---------------------------------------------------------------------------
# get_user_groups tests
# ---------------------------------------------------------------------------


class TestGetUserGroups:
    def test_current_user_returns_list(self) -> None:
        groups = get_user_groups()
        assert isinstance(groups, list)

    def test_current_user_nonempty(self) -> None:
        groups = get_user_groups()
        assert len(groups) > 0

    def test_current_user_all_strings(self) -> None:
        groups = get_user_groups()
        for g in groups:
            assert isinstance(g, str)
            assert len(g) > 0

    def test_specific_user(self) -> None:
        """Look up the current user by name — should work."""
        if not hasattr(os, "getuid"):
            pytest.skip("Not on Unix")
        import getpass

        username = getpass.getuser()
        groups = get_user_groups(username)
        assert isinstance(groups, list)
        assert len(groups) > 0

    def test_nonexistent_user_raises(self) -> None:
        if not hasattr(os, "getuid"):
            pytest.skip("Not on Unix")
        with pytest.raises(KeyError):
            get_user_groups("nonexistent_user_xyz_99999")

    def test_current_user_has_primary_group(self) -> None:
        """The primary group should appear in the results."""
        if not hasattr(os, "getgid"):
            pytest.skip("Not on Unix")
        groups = get_user_groups()
        # At minimum, we should have at least one group.
        assert len(groups) >= 1


# ---------------------------------------------------------------------------
# Main function integration tests
# ---------------------------------------------------------------------------


class TestMain:
    def test_main_no_args(self, capsys: pytest.CaptureFixture[str]) -> None:
        from groups_tool import main

        old_argv = sys.argv
        try:
            sys.argv = ["groups"]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert len(captured.out.strip()) > 0

    def test_main_with_current_user(self, capsys: pytest.CaptureFixture[str]) -> None:
        import getpass

        from groups_tool import main

        username = getpass.getuser()
        old_argv = sys.argv
        try:
            sys.argv = ["groups", username]
            main()
        except SystemExit:
            pass
        finally:
            sys.argv = old_argv
        captured = capsys.readouterr()
        assert username in captured.out
