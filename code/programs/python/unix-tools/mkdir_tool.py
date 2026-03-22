"""mkdir — create directories.

=== What This Program Does ===

This is a reimplementation of the GNU ``mkdir`` utility. It creates one
or more directories. By default, it creates a single level of directory;
with ``-p``, it creates the full path of parent directories as needed.

=== How Directory Creation Works ===

Creating a directory is one of the fundamental filesystem operations.
When you call ``mkdir foo``, the operating system:

1. Checks that the parent directory exists (e.g., the current directory).
2. Checks that ``foo`` does not already exist.
3. Allocates an inode for the new directory.
4. Creates the ``.`` (self) and ``..`` (parent) entries.
5. Updates the parent directory to include ``foo``.

=== The -p Flag (Parents) ===

Without ``-p``, ``mkdir a/b/c`` fails if ``a/b`` doesn't exist.
With ``-p``, mkdir creates ``a``, then ``a/b``, then ``a/b/c`` — the
entire chain of missing directories. It also silently succeeds if the
directory already exists, which makes it safe to use in scripts.

This is analogous to Python's ``os.makedirs()`` vs ``os.mkdir()``.

=== The -m Flag (Mode) ===

The ``-m`` flag sets the permission bits of the new directory. It accepts
an octal string like ``755`` or ``0700``. Without this flag, the default
permissions are determined by the process's umask.

=== The -v Flag (Verbose) ===

With ``-v``, mkdir prints a message for each directory it creates::

    $ mkdir -pv a/b/c
    mkdir: created directory 'a'
    mkdir: created directory 'a/b'
    mkdir: created directory 'a/b/c'

=== CLI Builder Integration ===

The entire CLI is defined in ``mkdir.json``. CLI Builder handles flag
parsing, help text, and version output. This file implements the
directory creation logic.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "mkdir.json")


def create_directory(
    path: str,
    *,
    parents: bool,
    mode: int | None,
    verbose: bool,
) -> bool:
    """Create a directory, optionally with parents.

    This function wraps Python's ``os.mkdir`` and ``os.makedirs`` to
    provide the behavior of GNU mkdir. It returns True on success and
    False on failure (after printing an error message).

    Args:
        path: The directory path to create.
        parents: If True, create parent directories as needed.
        mode: Octal permission mode, or None for default.
        verbose: If True, print each directory created.

    Returns:
        True if the directory was created successfully, False otherwise.
    """
    # Determine the permission mode to use.
    effective_mode = mode if mode is not None else 0o777

    try:
        if parents:
            # os.makedirs creates the full chain of directories.
            # exist_ok=True means it won't fail if the dir already exists,
            # matching the GNU mkdir -p behavior.
            os.makedirs(path, mode=effective_mode, exist_ok=True)
        else:
            os.mkdir(path, mode=effective_mode)
    except FileExistsError:
        print(
            f"mkdir: cannot create directory '{path}': File exists",
            file=sys.stderr,
        )
        return False
    except FileNotFoundError:
        # This happens when a parent directory doesn't exist and -p
        # was not specified.
        print(
            f"mkdir: cannot create directory '{path}': No such file or directory",
            file=sys.stderr,
        )
        return False
    except PermissionError:
        print(
            f"mkdir: cannot create directory '{path}': Permission denied",
            file=sys.stderr,
        )
        return False

    if verbose:
        print(f"mkdir: created directory '{path}'")

    return True


def parse_mode(mode_str: str) -> int | None:
    """Parse an octal mode string like '755' into an integer.

    Args:
        mode_str: An octal string representing file permissions.

    Returns:
        The integer value of the octal mode, or None if invalid.
    """
    try:
        return int(mode_str, 8)
    except ValueError:
        return None


def main() -> None:
    """Entry point: parse args via CLI Builder, then create directories."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"mkdir: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Business logic --------------------------------------------
    assert isinstance(result, ParseResult)

    parents = result.flags.get("parents", False)
    mode_str = result.flags.get("mode")
    verbose = result.flags.get("verbose", False)

    # Parse the mode if provided.
    mode = None
    if mode_str is not None:
        mode = parse_mode(mode_str)
        if mode is None:
            print(
                f"mkdir: invalid mode '{mode_str}'",
                file=sys.stderr,
            )
            raise SystemExit(1)

    # Get the list of directories.
    directories = result.arguments.get("directories", [])
    if isinstance(directories, str):
        directories = [directories]

    # Create each directory.
    exit_code = 0
    for directory in directories:
        if not create_directory(directory, parents=parents, mode=mode, verbose=verbose):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
