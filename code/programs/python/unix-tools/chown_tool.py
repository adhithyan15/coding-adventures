"""chown -- change file owner and group.

=== What This Program Does ===

This is a reimplementation of the GNU ``chown`` utility. It changes
the owner and/or group of files and directories::

    chown alice file.txt          # Change owner to alice
    chown alice:staff file.txt    # Change owner and group
    chown :staff file.txt         # Change group only
    chown -R alice directory/     # Recursive ownership change

=== How Unix Ownership Works ===

Every file in Unix has two ownership attributes:

1. **Owner (UID)**: The user who owns the file. Usually the file's
   creator. The owner can always change the file's permissions.

2. **Group (GID)**: A group of users who share access. Every user
   belongs to one or more groups.

These are stored as numeric IDs (UID and GID) in the file's inode.
The ``chown`` command translates between human-readable names and
these numeric IDs.

=== Owner:Group Syntax ===

The ``OWNER[:GROUP]`` argument supports several formats:

+----------------+---------------------------------------------+
| Format         | Effect                                      |
+================+=============================================+
| ``OWNER``      | Change only the owner                       |
| ``OWNER:GROUP``| Change both owner and group                 |
| ``OWNER:``     | Change owner, set group to owner's login grp|
| ``:GROUP``     | Change only the group                       |
| ``OWNER.GROUP``| Same as OWNER:GROUP (legacy syntax)         |
+----------------+---------------------------------------------+

=== Symlink Handling ===

By default, ``chown`` follows symbolic links and changes the target.
With ``-h`` (``--no-dereference``), it changes the symlink itself.
Note that ``os.lchown`` is used for the no-dereference case.

=== Permission Requirements ===

Only root can change file ownership on most Unix systems. Regular
users can change the group to any group they belong to, but cannot
change the owner. Our implementation gracefully handles permission
errors.

=== CLI Builder Integration ===

The CLI is defined in ``chown.json``. CLI Builder handles flag parsing.
"""

from __future__ import annotations

import grp
import os
import pwd
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "chown.json")


# ---------------------------------------------------------------------------
# Parse the OWNER[:GROUP] argument.
# ---------------------------------------------------------------------------


def parse_owner_group(
    spec: str,
) -> tuple[int | None, int | None]:
    """Parse an OWNER[:GROUP] specification into UID and GID.

    This function handles all the different formats:

    - ``"alice"``      -> (uid_of_alice, None)
    - ``"alice:staff"``-> (uid_of_alice, gid_of_staff)
    - ``"alice:"``     -> (uid_of_alice, primary_gid_of_alice)
    - ``":staff"``     -> (None, gid_of_staff)
    - ``"1000"``       -> (1000, None)
    - ``"1000:1000"``  -> (1000, 1000)

    Names are resolved via ``pwd.getpwnam`` and ``grp.getgrnam``.
    Numeric strings are used as literal UIDs/GIDs if name lookup fails.

    Args:
        spec: The owner:group specification string.

    Returns:
        A tuple of (uid, gid), where either can be None if not specified.

    Raises:
        ValueError: If the spec contains an invalid user or group name.
    """
    # Determine the separator (: or . for legacy compatibility).
    if ":" in spec:
        separator = ":"
    elif "." in spec:
        separator = "."
    else:
        separator = None

    if separator:
        owner_str, group_str = spec.split(separator, 1)
    else:
        owner_str = spec
        group_str = ""

    uid: int | None = None
    gid: int | None = None

    # --- Resolve owner ---
    if owner_str:
        uid = _resolve_user(owner_str)
        if uid is None:
            raise ValueError(f"invalid user: '{owner_str}'")

        # If the spec is "OWNER:" (with separator but empty group),
        # use the owner's primary group.
        if separator and not group_str:
            try:
                pw = pwd.getpwuid(uid)
                gid = pw.pw_gid
            except KeyError:
                pass

    # --- Resolve group ---
    if group_str:
        gid = _resolve_group(group_str)
        if gid is None:
            raise ValueError(f"invalid group: '{group_str}'")

    return uid, gid


def _resolve_user(name: str) -> int | None:
    """Resolve a username or numeric UID to a UID.

    First tries to look up by name. If that fails, tries to interpret
    as a numeric UID.

    Args:
        name: Username or numeric UID string.

    Returns:
        The UID as an integer, or None if resolution fails.
    """
    try:
        return pwd.getpwnam(name).pw_uid
    except KeyError:
        pass
    try:
        uid = int(name)
        return uid
    except ValueError:
        return None


def _resolve_group(name: str) -> int | None:
    """Resolve a group name or numeric GID to a GID.

    First tries to look up by name. If that fails, tries to interpret
    as a numeric GID.

    Args:
        name: Group name or numeric GID string.

    Returns:
        The GID as an integer, or None if resolution fails.
    """
    try:
        return grp.getgrnam(name).gr_gid
    except KeyError:
        pass
    try:
        gid = int(name)
        return gid
    except ValueError:
        return None


# ---------------------------------------------------------------------------
# Business logic -- change file ownership.
# ---------------------------------------------------------------------------


def chown_file(
    path: str,
    uid: int | None,
    gid: int | None,
    *,
    recursive: bool = False,
    verbose: bool = False,
    changes: bool = False,
    silent: bool = False,
    no_dereference: bool = False,
) -> bool:
    """Change the owner and/or group of a file.

    Args:
        path: Path to the file or directory.
        uid: New owner UID, or None to leave unchanged.
        gid: New group GID, or None to leave unchanged.
        recursive: If True, apply to directory contents recursively.
        verbose: If True, print every file processed.
        changes: If True, print only when a change is made.
        silent: If True, suppress error messages.
        no_dereference: If True, change the symlink, not its target.

    Returns:
        True on success, False on error.
    """
    # Get current ownership for comparison and defaults.
    try:
        stat_fn = os.lstat if no_dereference else os.stat
        current_stat = stat_fn(path)
    except FileNotFoundError:
        if not silent:
            print(
                f"chown: cannot access '{path}': No such file or directory",
                file=sys.stderr,
            )
        return False
    except PermissionError:
        if not silent:
            print(
                f"chown: cannot access '{path}': Permission denied",
                file=sys.stderr,
            )
        return False

    # Use current values for unspecified uid/gid.
    effective_uid = uid if uid is not None else current_stat.st_uid
    effective_gid = gid if gid is not None else current_stat.st_gid

    # Determine if a change will occur.
    will_change = (
        effective_uid != current_stat.st_uid
        or effective_gid != current_stat.st_gid
    )

    # Apply the ownership change.
    try:
        if no_dereference:
            os.lchown(path, effective_uid, effective_gid)
        else:
            os.chown(path, effective_uid, effective_gid)
    except PermissionError:
        if not silent:
            print(
                f"chown: changing ownership of '{path}': Operation not permitted",
                file=sys.stderr,
            )
        return False
    except OSError as e:
        if not silent:
            print(
                f"chown: changing ownership of '{path}': {e.strerror}",
                file=sys.stderr,
            )
        return False

    # Report.
    if verbose or (changes and will_change):
        _report_change(path, current_stat.st_uid, current_stat.st_gid,
                       effective_uid, effective_gid)

    # Recurse into directories.
    is_dir = os.path.isdir(path)
    is_link = os.path.islink(path)
    is_directory = is_dir and (not is_link or not no_dereference)
    if recursive and is_directory:
        success = True
        try:
            for entry in os.listdir(path):
                child = os.path.join(path, entry)
                if not chown_file(
                    child, uid, gid,
                    recursive=True,
                    verbose=verbose,
                    changes=changes,
                    silent=silent,
                    no_dereference=no_dereference,
                ):
                    success = False
        except PermissionError:
            if not silent:
                print(
                    f"chown: cannot read directory '{path}': Permission denied",
                    file=sys.stderr,
                )
            return False
        return success

    return True


def _report_change(
    path: str,
    old_uid: int,
    old_gid: int,
    new_uid: int,
    new_gid: int,
) -> None:
    """Print a report about an ownership change.

    Tries to resolve UIDs/GIDs back to names for human-readable output.
    """
    old_user = _uid_to_name(old_uid)
    old_group = _gid_to_name(old_gid)
    new_user = _uid_to_name(new_uid)
    new_group = _gid_to_name(new_gid)
    print(
        f"ownership of '{path}' changed from "
        f"{old_user}:{old_group} to {new_user}:{new_group}"
    )


def _uid_to_name(uid: int) -> str:
    """Convert a UID to a username, falling back to the numeric UID."""
    try:
        return pwd.getpwuid(uid).pw_name
    except KeyError:
        return str(uid)


def _gid_to_name(gid: int) -> str:
    """Convert a GID to a group name, falling back to the numeric GID."""
    try:
        return grp.getgrgid(gid).gr_name
    except KeyError:
        return str(gid)


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then chown files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"chown: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    assert isinstance(result, ParseResult)

    # --- Extract flags ---
    recursive = result.flags.get("recursive", False)
    verbose = result.flags.get("verbose", False)
    changes_flag = result.flags.get("changes", False)
    silent = result.flags.get("silent", False)
    no_dereference = result.flags.get("no_dereference", False)
    reference = result.flags.get("reference", None)

    # --- Determine UID and GID ---
    if reference:
        try:
            ref_stat = os.stat(reference)
            uid = ref_stat.st_uid
            gid = ref_stat.st_gid
        except FileNotFoundError:
            print(
                f"chown: cannot stat '{reference}': No such file or directory",
                file=sys.stderr,
            )
            raise SystemExit(1) from None
    else:
        owner_group = result.arguments.get("owner_group", "")
        try:
            uid, gid = parse_owner_group(owner_group)
        except ValueError as e:
            print(f"chown: {e}", file=sys.stderr)
            raise SystemExit(1) from None

    # --- Extract files ---
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    # --- Apply chown ---
    exit_code = 0
    for filepath in files:
        if not chown_file(
            filepath, uid, gid,
            recursive=recursive,
            verbose=verbose,
            changes=changes_flag,
            silent=silent,
            no_dereference=no_dereference,
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
