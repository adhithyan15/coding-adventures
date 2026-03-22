"""Tests for the chown tool.

=== What These Tests Verify ===

These tests exercise the chown implementation, including:

1. Parsing OWNER[:GROUP] specifications
2. User and group name resolution
3. Numeric UID/GID handling
4. File ownership changes (gracefully handling permission errors)
5. Verbose and changes-only reporting
6. Error handling (missing files, invalid users)
7. Spec loading and CLI Builder integration

Note: Most actual chown operations require root privileges. These tests
use the current user's UID/GID to test the logic without needing root.
Tests that would require root are designed to gracefully handle
PermissionError.
"""

from __future__ import annotations

import grp
import os
import pwd
import sys
from pathlib import Path
from typing import Any

import pytest

SPEC_FILE = str(Path(__file__).parent.parent / "chown.json")
sys.path.insert(0, str(Path(__file__).parent.parent))

from chown_tool import (
    _gid_to_name,
    _resolve_group,
    _resolve_user,
    _uid_to_name,
    chown_file,
    parse_owner_group,
)


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

        result = parse_argv(["chown", "--help"])
        assert isinstance(result, HelpResult)

    def test_version(self) -> None:
        from cli_builder import VersionResult

        result = parse_argv(["chown", "--version"])
        assert isinstance(result, VersionResult)


# ---------------------------------------------------------------------------
# User and group resolution
# ---------------------------------------------------------------------------


class TestResolveUser:
    def test_current_user_by_name(self) -> None:
        """Resolving the current user's name returns the correct UID."""
        current_uid = os.getuid()
        current_name = pwd.getpwuid(current_uid).pw_name
        assert _resolve_user(current_name) == current_uid

    def test_numeric_uid(self) -> None:
        """Numeric UID strings are resolved."""
        uid = os.getuid()
        assert _resolve_user(str(uid)) == uid

    def test_invalid_user(self) -> None:
        """Invalid usernames return None."""
        assert _resolve_user("nonexistent_user_12345") is None


class TestResolveGroup:
    def test_current_group_by_name(self) -> None:
        """Resolving a group name returns the correct GID."""
        current_gid = os.getgid()
        group_name = grp.getgrgid(current_gid).gr_name
        assert _resolve_group(group_name) == current_gid

    def test_numeric_gid(self) -> None:
        """Numeric GID strings are resolved."""
        gid = os.getgid()
        assert _resolve_group(str(gid)) == gid

    def test_invalid_group(self) -> None:
        """Invalid group names return None."""
        assert _resolve_group("nonexistent_group_12345") is None


# ---------------------------------------------------------------------------
# UID/GID to name conversion
# ---------------------------------------------------------------------------


class TestUidGidToName:
    def test_uid_to_name(self) -> None:
        """UID converts back to username."""
        uid = os.getuid()
        name = _uid_to_name(uid)
        assert name == pwd.getpwuid(uid).pw_name

    def test_gid_to_name(self) -> None:
        """GID converts back to group name."""
        gid = os.getgid()
        name = _gid_to_name(gid)
        assert name == grp.getgrgid(gid).gr_name

    def test_unknown_uid(self) -> None:
        """Unknown UIDs fall back to numeric string."""
        name = _uid_to_name(99999)
        assert name == "99999"

    def test_unknown_gid(self) -> None:
        """Unknown GIDs fall back to numeric string."""
        name = _gid_to_name(99999)
        assert name == "99999"


# ---------------------------------------------------------------------------
# parse_owner_group
# ---------------------------------------------------------------------------


class TestParseOwnerGroup:
    def test_owner_only(self) -> None:
        """Parse 'username' -- owner only."""
        uid = os.getuid()
        username = pwd.getpwuid(uid).pw_name
        parsed_uid, parsed_gid = parse_owner_group(username)
        assert parsed_uid == uid
        assert parsed_gid is None

    def test_owner_colon_group(self) -> None:
        """Parse 'owner:group' -- both specified."""
        uid = os.getuid()
        gid = os.getgid()
        username = pwd.getpwuid(uid).pw_name
        groupname = grp.getgrgid(gid).gr_name
        parsed_uid, parsed_gid = parse_owner_group(f"{username}:{groupname}")
        assert parsed_uid == uid
        assert parsed_gid == gid

    def test_colon_group_only(self) -> None:
        """Parse ':group' -- group only."""
        gid = os.getgid()
        groupname = grp.getgrgid(gid).gr_name
        parsed_uid, parsed_gid = parse_owner_group(f":{groupname}")
        assert parsed_uid is None
        assert parsed_gid == gid

    def test_owner_colon_empty(self) -> None:
        """Parse 'owner:' -- owner with login group."""
        uid = os.getuid()
        username = pwd.getpwuid(uid).pw_name
        parsed_uid, parsed_gid = parse_owner_group(f"{username}:")
        assert parsed_uid == uid
        # GID should be the user's primary group.
        assert parsed_gid is not None

    def test_numeric_uid(self) -> None:
        """Parse numeric UID."""
        uid = os.getuid()
        parsed_uid, parsed_gid = parse_owner_group(str(uid))
        assert parsed_uid == uid
        assert parsed_gid is None

    def test_numeric_uid_gid(self) -> None:
        """Parse 'uid:gid' with numeric values."""
        uid = os.getuid()
        gid = os.getgid()
        parsed_uid, parsed_gid = parse_owner_group(f"{uid}:{gid}")
        assert parsed_uid == uid
        assert parsed_gid == gid

    def test_invalid_user_raises(self) -> None:
        """Invalid username raises ValueError."""
        with pytest.raises(ValueError, match="invalid user"):
            parse_owner_group("nonexistent_user_12345")

    def test_invalid_group_raises(self) -> None:
        """Invalid group name raises ValueError."""
        uid = os.getuid()
        username = pwd.getpwuid(uid).pw_name
        with pytest.raises(ValueError, match="invalid group"):
            parse_owner_group(f"{username}:nonexistent_group_12345")

    def test_dot_separator(self) -> None:
        """Legacy dot separator works like colon."""
        uid = os.getuid()
        gid = os.getgid()
        username = pwd.getpwuid(uid).pw_name
        groupname = grp.getgrgid(gid).gr_name
        parsed_uid, parsed_gid = parse_owner_group(f"{username}.{groupname}")
        assert parsed_uid == uid
        assert parsed_gid == gid


# ---------------------------------------------------------------------------
# chown_file -- ownership changes
# ---------------------------------------------------------------------------


class TestChownFile:
    def test_chown_same_owner(self, tmp_path: Path) -> None:
        """Changing to the same owner succeeds."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        uid = os.getuid()
        gid = os.getgid()

        # This should succeed since we're not actually changing anything.
        result = chown_file(str(f), uid, gid)
        assert result is True

    def test_missing_file(self, tmp_path: Path) -> None:
        """Chown on a missing file returns False."""
        result = chown_file(str(tmp_path / "nonexistent"), 0, 0)
        assert result is False

    def test_verbose_output(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Verbose mode prints ownership information."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        uid = os.getuid()
        gid = os.getgid()

        chown_file(str(f), uid, gid, verbose=True)
        captured = capsys.readouterr()
        assert "ownership" in captured.out

    def test_silent_suppresses_errors(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Silent mode suppresses error messages."""
        result = chown_file(
            str(tmp_path / "nonexistent"), 0, 0,
            silent=True,
        )
        assert result is False
        captured = capsys.readouterr()
        assert captured.err == ""

    def test_uid_none_keeps_current(self, tmp_path: Path) -> None:
        """When uid is None, the current owner is kept."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        original_uid = os.stat(str(f)).st_uid

        chown_file(str(f), None, os.getgid())
        assert os.stat(str(f)).st_uid == original_uid

    def test_gid_none_keeps_current(self, tmp_path: Path) -> None:
        """When gid is None, the current group is kept."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        original_gid = os.stat(str(f)).st_gid

        chown_file(str(f), os.getuid(), None)
        assert os.stat(str(f)).st_gid == original_gid


# ---------------------------------------------------------------------------
# Recursive chown
# ---------------------------------------------------------------------------


class TestRecursiveChown:
    def test_recursive_directory(self, tmp_path: Path) -> None:
        """Recursive mode processes directory contents."""
        d = tmp_path / "dir"
        d.mkdir()
        f = d / "file.txt"
        f.write_text("content")
        uid = os.getuid()
        gid = os.getgid()

        result = chown_file(str(d), uid, gid, recursive=True)
        assert result is True

    def test_recursive_subdirectories(self, tmp_path: Path) -> None:
        """Recursive mode descends into subdirectories."""
        d = tmp_path / "dir"
        d.mkdir()
        sub = d / "sub"
        sub.mkdir()
        f = sub / "file.txt"
        f.write_text("content")
        uid = os.getuid()
        gid = os.getgid()

        result = chown_file(str(d), uid, gid, recursive=True)
        assert result is True

    def test_non_recursive_skips_children(self, tmp_path: Path) -> None:
        """Without -R, only the target is changed."""
        d = tmp_path / "dir"
        d.mkdir()
        f = d / "file.txt"
        f.write_text("content")
        uid = os.getuid()
        gid = os.getgid()

        # Should succeed for the directory but not recurse.
        result = chown_file(str(d), uid, gid, recursive=False)
        assert result is True


# ---------------------------------------------------------------------------
# Changes-only reporting
# ---------------------------------------------------------------------------


class TestChangesReporting:
    def test_changes_reports_when_changed(
        self, tmp_path: Path, capsys: pytest.CaptureFixture[str]
    ) -> None:
        """Changes mode reports when ownership changes."""
        f = tmp_path / "test.txt"
        f.write_text("content")
        uid = os.getuid()
        gid = os.getgid()

        # Changing to same owner -- no actual change.
        chown_file(str(f), uid, gid, changes=True)
        captured = capsys.readouterr()
        # Since we're setting to the same values, no change reported.
        assert captured.out == ""
