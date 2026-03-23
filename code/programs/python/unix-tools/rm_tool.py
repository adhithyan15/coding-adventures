"""rm — remove files or directories.

=== What This Program Does ===

This is a reimplementation of the GNU ``rm`` utility. It removes
(deletes) files and directories. Unlike ``rmdir``, which only removes
empty directories, ``rm -r`` can remove entire directory trees.

=== Safety Features ===

rm is one of the most dangerous Unix commands. A mistyped ``rm -rf /``
could destroy an entire system. GNU rm includes several safety features:

1. **--preserve-root** (default on): Refuses to operate on ``/``.
2. **-i** (interactive): Prompts before each removal.
3. **-I** (interactive once): Prompts once before removing more than
   three files or when removing recursively.
4. **No implicit recursion**: ``rm directory`` fails without ``-r``.

=== How Recursive Removal Works ===

When ``-r`` is specified, rm traverses the directory tree bottom-up:

1. List all entries in the directory.
2. For each entry: if it's a directory, recurse into it first.
3. Remove all files in the directory.
4. Remove the now-empty directory itself.

This bottom-up approach is necessary because you can't remove a
directory that still contains files.

=== The -f Flag (Force) ===

With ``-f``, rm ignores nonexistent files and never prompts. This is
commonly combined with ``-r`` as ``rm -rf`` to unconditionally remove
a directory tree.

=== CLI Builder Integration ===

The entire CLI is defined in ``rm.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "rm.json")


def confirm_removal(prompt: str) -> bool:
    """Ask the user for confirmation.

    Args:
        prompt: The prompt message to display.

    Returns:
        True if the user confirms (y/yes), False otherwise.
    """
    try:
        response = input(prompt).strip().lower()
    except EOFError:
        return False
    return response in ("y", "yes")


def remove_file(
    filepath: str,
    *,
    force: bool,
    interactive: bool,
    recursive: bool,
    verbose: bool,
    dir_flag: bool,
    preserve_root: bool,
) -> bool:
    """Remove a single file or directory.

    Args:
        filepath: The path to remove.
        force: If True, ignore nonexistent files and never prompt.
        interactive: If True, prompt before each removal.
        recursive: If True, remove directories recursively.
        verbose: If True, explain what is being done.
        dir_flag: If True, remove empty directories.
        preserve_root: If True, refuse to remove '/'.

    Returns:
        True on success, False on failure.
    """
    # Safety check: refuse to remove root.
    if preserve_root and os.path.abspath(filepath) == "/":
        print(
            "rm: it is dangerous to operate recursively on '/'",
            file=sys.stderr,
        )
        print(
            "rm: use --no-preserve-root to override this failsafe",
            file=sys.stderr,
        )
        return False

    # Check if the file exists.
    if not os.path.lexists(filepath):
        if not force:
            print(
                f"rm: cannot remove '{filepath}': No such file or directory",
                file=sys.stderr,
            )
        return force  # Force mode treats missing files as success.

    # Handle directories.
    if os.path.isdir(filepath) and not os.path.islink(filepath):
        if recursive:
            if interactive:
                if not confirm_removal(f"rm: descend into directory '{filepath}'? "):
                    return True

            try:
                shutil.rmtree(filepath)
            except PermissionError:
                print(
                    f"rm: cannot remove '{filepath}': Permission denied",
                    file=sys.stderr,
                )
                return False
            except OSError as e:
                print(f"rm: cannot remove '{filepath}': {e.strerror}", file=sys.stderr)
                return False

            if verbose:
                print(f"removed directory '{filepath}'")
            return True

        if dir_flag:
            # Try to remove as an empty directory.
            try:
                os.rmdir(filepath)
            except OSError:
                print(
                    f"rm: cannot remove '{filepath}': Directory not empty",
                    file=sys.stderr,
                )
                return False

            if verbose:
                print(f"removed directory '{filepath}'")
            return True

        # Not recursive and not dir_flag.
        print(
            f"rm: cannot remove '{filepath}': Is a directory",
            file=sys.stderr,
        )
        return False

    # Handle regular files and symlinks.
    if interactive and not force:
        if not confirm_removal(f"rm: remove file '{filepath}'? "):
            return True

    try:
        os.unlink(filepath)
    except PermissionError:
        if not force:
            print(
                f"rm: cannot remove '{filepath}': Permission denied",
                file=sys.stderr,
            )
        return False
    except OSError as e:
        if not force:
            print(f"rm: cannot remove '{filepath}': {e.strerror}", file=sys.stderr)
        return False

    if verbose:
        print(f"removed '{filepath}'")

    return True


def main() -> None:
    """Entry point: parse args via CLI Builder, then remove files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"rm: {error.message}", file=sys.stderr)
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

    force = result.flags.get("force", False)
    interactive = result.flags.get("interactive", False)
    interactive_once = result.flags.get("interactive_once", False)
    recursive = result.flags.get("recursive", False)
    verbose = result.flags.get("verbose", False)
    dir_flag = result.flags.get("dir", False)
    preserve_root = result.flags.get("preserve_root", True)

    # Get the list of files.
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    # Handle -I (prompt once for bulk operations).
    if interactive_once and not force:
        if len(files) > 3 or recursive:
            if not confirm_removal(
                f"rm: remove {len(files)} argument{'s' if len(files) != 1 else ''}? "
            ):
                raise SystemExit(0)

    exit_code = 0
    for filepath in files:
        if not remove_file(
            filepath,
            force=force,
            interactive=interactive,
            recursive=recursive,
            verbose=verbose,
            dir_flag=dir_flag,
            preserve_root=preserve_root,
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
