"""rmdir — remove empty directories.

=== What This Program Does ===

This is a reimplementation of the GNU ``rmdir`` utility. It removes
directories, but only if they are empty. This is a safety feature:
unlike ``rm -r``, rmdir will never accidentally delete files.

=== Why rmdir Only Removes Empty Directories ===

The restriction to empty directories is deliberate. It prevents
accidental data loss. If you want to remove a directory and everything
inside it, you must use ``rm -r`` instead. rmdir is the "safe" option.

=== The -p Flag (Parents) ===

With ``-p``, rmdir removes the directory and then tries to remove each
parent component of the path. For example::

    $ rmdir -p a/b/c

This first removes ``a/b/c``, then ``a/b``, then ``a`` — but only if
each is empty after the child is removed.

This is the inverse of ``mkdir -p a/b/c``.

=== The --ignore-fail-on-non-empty Flag ===

By default, rmdir prints an error if the directory is not empty.
The ``--ignore-fail-on-non-empty`` flag suppresses this specific error,
which is useful in scripts where you want to remove a directory only
if it happens to be empty.

=== CLI Builder Integration ===

The entire CLI is defined in ``rmdir.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "rmdir.json")


def remove_directory(
    path: str,
    *,
    verbose: bool,
    ignore_non_empty: bool,
) -> bool:
    """Remove an empty directory.

    Args:
        path: The directory to remove.
        verbose: If True, print a message for each removal.
        ignore_non_empty: If True, suppress errors for non-empty directories.

    Returns:
        True on success, False on failure.
    """
    try:
        os.rmdir(path)
    except FileNotFoundError:
        print(
            f"rmdir: failed to remove '{path}': No such file or directory",
            file=sys.stderr,
        )
        return False
    except OSError:
        # OSError covers "Directory not empty" on all platforms.
        if not ignore_non_empty:
            print(
                f"rmdir: failed to remove '{path}': Directory not empty",
                file=sys.stderr,
            )
        return False

    if verbose:
        print(f"rmdir: removing directory, '{path}'")

    return True


def remove_with_parents(
    path: str,
    *,
    verbose: bool,
    ignore_non_empty: bool,
) -> bool:
    """Remove a directory and then each parent in turn.

    For example, ``remove_with_parents("a/b/c")`` will try to remove
    ``a/b/c``, then ``a/b``, then ``a``.

    Args:
        path: The deepest directory to remove.
        verbose: If True, print a message for each removal.
        ignore_non_empty: If True, suppress errors for non-empty directories.

    Returns:
        True if all removals succeeded, False otherwise.
    """
    # Start with the given path and work up to the root.
    current = path
    success = True

    while current and current != os.sep:
        if not remove_directory(
            current, verbose=verbose, ignore_non_empty=ignore_non_empty
        ):
            success = False
            break

        # Move to the parent directory.
        parent = os.path.dirname(current)
        # Stop if we've reached the root or current directory.
        if parent == current:
            break
        current = parent

    return success


def main() -> None:
    """Entry point: parse args via CLI Builder, then remove directories."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"rmdir: {error.message}", file=sys.stderr)
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
    verbose = result.flags.get("verbose", False)
    ignore_non_empty = result.flags.get("ignore_fail_on_non_empty", False)

    # Get the list of directories.
    directories = result.arguments.get("directories", [])
    if isinstance(directories, str):
        directories = [directories]

    exit_code = 0
    for directory in directories:
        if parents:
            if not remove_with_parents(
                directory, verbose=verbose, ignore_non_empty=ignore_non_empty
            ):
                exit_code = 1
        else:
            if not remove_directory(
                directory, verbose=verbose, ignore_non_empty=ignore_non_empty
            ):
                exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
