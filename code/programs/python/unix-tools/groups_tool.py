"""groups — print the groups a user is in.

=== What This Program Does ===

This is a reimplementation of the GNU ``groups`` utility. It prints
the group memberships for each specified user, or for the current
user if no argument is given.

=== How Groups Work on Unix ===

Every user on a Unix system belongs to one or more groups. Groups
are a fundamental part of the Unix permission model:

- Each file has an owning user and an owning group.
- Group membership determines what files you can access.
- The ``groups`` command shows you which groups you belong to.

=== Primary vs Supplementary Groups ===

Each user has exactly one **primary group** (stored in ``/etc/passwd``).
They may also belong to additional **supplementary groups** (stored
in ``/etc/group``). The ``groups`` command shows all of them.

=== Output Format ===

When given a username argument, the output is prefixed with the
username and a colon::

    $ groups alice
    alice : alice sudo docker

When no argument is given, just the group names are printed::

    $ groups
    alice sudo docker

=== Cross-Platform Considerations ===

On Unix, we use the ``grp``, ``pwd``, and ``os`` modules. On Windows,
these are unavailable, so we provide a limited fallback.

=== CLI Builder Integration ===

The entire CLI is defined in ``groups.json``. CLI Builder handles flag
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

SPEC_FILE = str(Path(__file__).parent / "groups.json")


# ---------------------------------------------------------------------------
# Business logic: get_user_groups
# ---------------------------------------------------------------------------


def get_user_groups(username: str | None = None) -> list[str]:
    """Get the list of group names for a user.

    If ``username`` is None, returns groups for the current user.
    Otherwise, looks up the specified user.

    Args:
        username: The username to look up, or None for the current user.

    Returns:
        A list of group name strings.

    Raises:
        KeyError: If the specified username does not exist.
    """
    if not _HAS_UNIX_MODULES:
        # Windows fallback.
        import getpass

        return [getpass.getuser()]

    if username is not None:
        # Look up a specific user.
        try:
            pw = pwd.getpwnam(username)
        except KeyError:
            msg = f"groups: '{username}': no such user"
            raise KeyError(msg) from None

        primary_gid = pw.pw_gid
        uname = pw.pw_name

        # Collect all groups.
        group_names: list[str] = []
        seen: set[int] = set()

        # Primary group first.
        try:
            primary_name = grp.getgrgid(primary_gid).gr_name
        except KeyError:
            primary_name = str(primary_gid)
        group_names.append(primary_name)
        seen.add(primary_gid)

        # Supplementary groups.
        for g in grp.getgrall():
            if uname in g.gr_mem and g.gr_gid not in seen:
                group_names.append(g.gr_name)
                seen.add(g.gr_gid)

        return group_names

    # Current user — use os.getgroups() for efficiency.
    gids = os.getgroups()
    group_names = []
    seen_gids: set[int] = set()

    # Include primary group first.
    primary_gid = os.getgid()
    try:
        primary_name = grp.getgrgid(primary_gid).gr_name
    except KeyError:
        primary_name = str(primary_gid)

    if primary_gid not in seen_gids:
        group_names.append(primary_name)
        seen_gids.add(primary_gid)

    for gid in gids:
        if gid not in seen_gids:
            try:
                name = grp.getgrgid(gid).gr_name
            except KeyError:
                name = str(gid)
            group_names.append(name)
            seen_gids.add(gid)

    return group_names


# ---------------------------------------------------------------------------
# Entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then print groups."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"groups: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    users = result.arguments.get("users")
    if isinstance(users, str):
        users = [users]

    if not users:
        # No arguments — print groups for the current user.
        try:
            group_names = get_user_groups()
        except KeyError as exc:
            print(str(exc).strip("'\""), file=sys.stderr)
            raise SystemExit(1) from None
        print(" ".join(group_names))
    else:
        # Print groups for each specified user.
        for username in users:
            try:
                group_names = get_user_groups(username)
            except KeyError as exc:
                print(str(exc).strip("'\""), file=sys.stderr)
                raise SystemExit(1) from None
            print(f"{username} : {' '.join(group_names)}")


if __name__ == "__main__":
    main()
