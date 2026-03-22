"""id — print real and effective user and group IDs.

=== What This Program Does ===

This is a reimplementation of the GNU ``id`` utility. It prints
information about the specified user (or the current user if none is
given): user ID (UID), group ID (GID), and supplementary group
memberships.

=== Default Output ===

With no flags, ``id`` prints all information in a specific format::

    uid=1000(alice) gid=1000(alice) groups=1000(alice),27(sudo),100(users)

This tells you:
- The effective UID and username.
- The effective GID and group name.
- All supplementary group memberships.

=== Selective Output Flags ===

You can request just one piece of information:

- ``-u``: Print only the effective user ID.
- ``-g``: Print only the effective group ID.
- ``-G``: Print all group IDs (primary + supplementary).

Each of these can be modified by:

- ``-n``: Print the name instead of the numeric ID.
- ``-r``: Print the real ID instead of the effective ID.

=== Cross-Platform Considerations ===

On Unix systems, we use the ``pwd``, ``grp``, and ``os`` modules to
query the system's user database. On Windows, these modules are not
available, so we fall back to environment variables and limited info.

=== CLI Builder Integration ===

The entire CLI is defined in ``id.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Unix-only modules — may not be available on Windows.
# ---------------------------------------------------------------------------

try:
    import grp
    import pwd

    _HAS_UNIX_MODULES = True
except ImportError:
    _HAS_UNIX_MODULES = False

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "id.json")


# ---------------------------------------------------------------------------
# Business logic: get_user_info
# ---------------------------------------------------------------------------


def get_user_info(username: str | None = None) -> dict:
    """Gather user and group information.

    If ``username`` is None, we look up the current process's user.
    Otherwise, we look up the named user in the system's password
    database.

    Returns a dictionary with:
    - ``uid``: The user ID (int).
    - ``gid``: The primary group ID (int).
    - ``username``: The username (str).
    - ``groupname``: The primary group name (str).
    - ``groups``: A list of (gid, groupname) tuples for all groups.
    - ``euid``: The effective user ID (int).
    - ``egid``: The effective group ID (int).

    Raises:
        KeyError: If the specified username does not exist.
    """
    if not _HAS_UNIX_MODULES:
        # Windows fallback: limited information.
        import getpass

        uname = username or getpass.getuser()
        return {
            "uid": 0,
            "gid": 0,
            "username": uname,
            "groupname": "unknown",
            "groups": [(0, "unknown")],
            "euid": 0,
            "egid": 0,
        }

    if username is not None:
        # Look up a specific user.
        try:
            pw = pwd.getpwnam(username)
        except KeyError:
            msg = f"id: '{username}': no such user"
            raise KeyError(msg) from None
        uid = pw.pw_uid
        gid = pw.pw_gid
        uname = pw.pw_name

        # Get all groups for this user.
        all_groups = _get_groups_for_user(uname, gid)

        return {
            "uid": uid,
            "gid": gid,
            "username": uname,
            "groupname": _gid_to_name(gid),
            "groups": all_groups,
            "euid": uid,
            "egid": gid,
        }

    # Current user.
    uid = os.getuid()
    euid = os.geteuid()
    gid = os.getgid()
    egid = os.getegid()
    uname = pwd.getpwuid(euid).pw_name

    # Get all supplementary groups for the current process.
    try:
        group_ids = os.getgroups()
    except OSError:
        group_ids = [gid]

    # Build the groups list with names.
    groups_list = []
    seen = set()
    # Always include the primary group first.
    if gid not in seen:
        groups_list.append((gid, _gid_to_name(gid)))
        seen.add(gid)
    for g in group_ids:
        if g not in seen:
            groups_list.append((g, _gid_to_name(g)))
            seen.add(g)

    return {
        "uid": uid,
        "gid": gid,
        "username": uname,
        "groupname": _gid_to_name(egid),
        "groups": groups_list,
        "euid": euid,
        "egid": egid,
    }


def _gid_to_name(gid: int) -> str:
    """Convert a GID to a group name, falling back to str(gid)."""
    if not _HAS_UNIX_MODULES:
        return str(gid)
    try:
        return grp.getgrgid(gid).gr_name
    except KeyError:
        return str(gid)


def _get_groups_for_user(username: str, primary_gid: int) -> list[tuple[int, str]]:
    """Get all groups for a specific user (by scanning /etc/group).

    Returns a list of (gid, group_name) tuples. The primary group
    is always first.
    """
    if not _HAS_UNIX_MODULES:
        return [(primary_gid, str(primary_gid))]

    groups: list[tuple[int, str]] = []
    seen: set[int] = set()

    # Always include primary group first.
    groups.append((primary_gid, _gid_to_name(primary_gid)))
    seen.add(primary_gid)

    # Scan all groups to find memberships.
    for g in grp.getgrall():
        if username in g.gr_mem and g.gr_gid not in seen:
            groups.append((g.gr_gid, g.gr_name))
            seen.add(g.gr_gid)

    return groups


def format_id_default(info: dict) -> str:
    """Format the default ``id`` output (all info).

    Example: ``uid=1000(alice) gid=1000(alice) groups=1000(alice),27(sudo)``
    """
    uid_part = f"uid={info['euid']}({info['username']})"
    gid_part = f"gid={info['egid']}({info['groupname']})"

    groups_parts = [f"{gid}({name})" for gid, name in info["groups"]]
    groups_part = f"groups={','.join(groups_parts)}"

    return f"{uid_part} {gid_part} {groups_part}"


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then print user info."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"id: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    username = result.arguments.get("user_name")
    show_user = result.flags.get("user", False)
    show_group = result.flags.get("group", False)
    show_groups = result.flags.get("groups", False)
    show_name = result.flags.get("name", False)
    show_real = result.flags.get("real", False)

    try:
        info = get_user_info(username)
    except KeyError as exc:
        print(str(exc).strip("'\""), file=sys.stderr)
        raise SystemExit(1) from None

    # --- Selective output modes ---

    if show_user:
        uid_val = info["uid"] if show_real else info["euid"]
        if show_name:
            if _HAS_UNIX_MODULES:
                print(pwd.getpwuid(uid_val).pw_name)
            else:
                print(info["username"])
        else:
            print(uid_val)
        return

    if show_group:
        gid_val = info["gid"] if show_real else info["egid"]
        if show_name:
            print(_gid_to_name(gid_val))
        else:
            print(gid_val)
        return

    if show_groups:
        if show_name:
            print(" ".join(name for _, name in info["groups"]))
        else:
            print(" ".join(str(gid) for gid, _ in info["groups"]))
        return

    # Default: full output.
    print(format_id_default(info))


if __name__ == "__main__":
    main()
