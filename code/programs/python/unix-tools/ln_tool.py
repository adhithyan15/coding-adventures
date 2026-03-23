"""ln — make links between files.

=== What This Program Does ===

This is a reimplementation of the GNU ``ln`` utility. It creates links
between files. Links come in two flavors:

1. **Hard links** (default): A hard link is a second directory entry
   pointing to the same inode (the actual data on disk). Both names
   are equally "real" — there is no original and no copy. Deleting one
   name leaves the other intact. Hard links cannot cross filesystem
   boundaries and cannot link to directories.

2. **Symbolic links** (``-s``): A symbolic link (symlink) is a small
   file that contains the path to another file. It's like a shortcut.
   Symlinks can cross filesystems and can point to directories, but
   they break if the target is moved or deleted.

=== Link Name Resolution ===

ln has two calling patterns:

1. ``ln TARGET LINK_NAME`` — Create LINK_NAME pointing to TARGET.
2. ``ln TARGET... DIRECTORY`` — Create links in DIRECTORY for each TARGET.

If only one argument is given and it's not a directory, a link is
created in the current directory with the same basename as the target.

=== The -f Flag (Force) ===

With ``-f``, ln removes existing destination files before creating the
link. Without it, ln fails if the destination already exists.

=== The -r Flag (Relative) ===

With ``-r`` (only meaningful with ``-s``), ln computes a relative path
from the link location to the target, rather than using the target path
as-is. This is useful when you want symlinks that work regardless of
where the directory tree is mounted.

=== CLI Builder Integration ===

The entire CLI is defined in ``ln.json``. CLI Builder handles flag
parsing, help text, and version output.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "ln.json")


def make_link(
    target: str,
    link_name: str,
    *,
    symbolic: bool,
    force: bool,
    verbose: bool,
    relative: bool,
    no_dereference: bool,
) -> bool:
    """Create a single link from link_name to target.

    Args:
        target: The file to link to.
        link_name: The name of the link to create.
        symbolic: If True, create a symbolic link. Otherwise, hard link.
        force: If True, remove existing destination files.
        verbose: If True, print what's being done.
        relative: If True, compute a relative path for symlinks.
        no_dereference: If True, treat link_name as a normal file even
            if it's a symlink to a directory.

    Returns:
        True on success, False on failure.
    """
    # If the link_name is a directory (and we're not using -T/-n),
    # create the link inside that directory using the target's basename.
    if not no_dereference and os.path.isdir(link_name):
        link_name = os.path.join(link_name, os.path.basename(target))

    # If force is enabled, remove the existing file.
    if force and os.path.lexists(link_name):
        try:
            os.unlink(link_name)
        except OSError as e:
            print(f"ln: cannot remove '{link_name}': {e.strerror}", file=sys.stderr)
            return False

    # Compute relative path for symlinks if requested.
    actual_target = target
    if symbolic and relative:
        # The relative path is computed from the directory containing
        # the link to the target.
        link_dir = os.path.dirname(os.path.abspath(link_name))
        actual_target = os.path.relpath(os.path.abspath(target), link_dir)

    try:
        if symbolic:
            os.symlink(actual_target, link_name)
        else:
            os.link(target, link_name)
    except FileExistsError:
        print(
            f"ln: failed to create {('symbolic ' if symbolic else '')}link "
            f"'{link_name}': File exists",
            file=sys.stderr,
        )
        return False
    except FileNotFoundError:
        print(
            f"ln: failed to create {('symbolic ' if symbolic else '')}link "
            f"'{link_name}': No such file or directory",
            file=sys.stderr,
        )
        return False
    except PermissionError:
        print(
            f"ln: failed to create {('symbolic ' if symbolic else '')}link "
            f"'{link_name}': Permission denied",
            file=sys.stderr,
        )
        return False
    except OSError as e:
        print(
            f"ln: failed to create {('symbolic ' if symbolic else '')}link "
            f"'{link_name}': {e.strerror}",
            file=sys.stderr,
        )
        return False

    if verbose:
        arrow = " -> " if symbolic else " => "
        print(f"'{link_name}'{arrow}'{actual_target}'")

    return True


def main() -> None:
    """Entry point: parse args via CLI Builder, then create links."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"ln: {error.message}", file=sys.stderr)
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

    symbolic = result.flags.get("symbolic", False)
    force = result.flags.get("force", False)
    verbose = result.flags.get("verbose", False)
    relative = result.flags.get("relative", False)
    no_dereference = result.flags.get("no_dereference", False)
    no_target_dir = result.flags.get("no_target_directory", False)

    # Get the list of targets.
    targets = result.arguments.get("targets", [])
    if isinstance(targets, str):
        targets = [targets]

    if len(targets) < 1:
        print("ln: missing file operand", file=sys.stderr)
        raise SystemExit(1)

    if len(targets) == 1:
        # Single argument: create a link in the current directory
        # with the same name as the target's basename.
        target = targets[0]
        link_name = os.path.basename(target)
        success = make_link(
            target,
            link_name,
            symbolic=symbolic,
            force=force,
            verbose=verbose,
            relative=relative,
            no_dereference=no_target_dir or no_dereference,
        )
        raise SystemExit(0 if success else 1)

    if len(targets) == 2 and (no_target_dir or not os.path.isdir(targets[-1])):
        # Two arguments, last one is not a directory (or -T is set):
        # link targets[0] to targets[1].
        success = make_link(
            targets[0],
            targets[1],
            symbolic=symbolic,
            force=force,
            verbose=verbose,
            relative=relative,
            no_dereference=no_target_dir or no_dereference,
        )
        raise SystemExit(0 if success else 1)

    # Multiple targets: last argument must be a directory.
    destination = targets[-1]
    if not os.path.isdir(destination):
        print(
            f"ln: target '{destination}' is not a directory",
            file=sys.stderr,
        )
        raise SystemExit(1)

    exit_code = 0
    for target in targets[:-1]:
        link_name = os.path.join(destination, os.path.basename(target))
        if not make_link(
            target,
            link_name,
            symbolic=symbolic,
            force=force,
            verbose=verbose,
            relative=relative,
            no_dereference=True,  # Already computed the final path.
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
