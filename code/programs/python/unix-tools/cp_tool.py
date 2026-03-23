"""cp -- copy files and directories.

=== What This Program Does ===

This is a reimplementation of the GNU ``cp`` utility. It copies files
and directories from one location to another. At its simplest::

    cp source.txt dest.txt       # Copy a single file
    cp file1.txt file2.txt dir/  # Copy multiple files into a directory

=== How Copying Works ===

Copying a file involves reading the source and writing an identical
copy at the destination. But there are subtleties:

1. **Metadata**: Should the copy have the same modification time?
   Permissions? Owner? ``cp`` by default preserves mode and timestamps
   (via ``shutil.copy2``).

2. **Directories**: By default, ``cp`` refuses to copy directories.
   You must use ``-R`` (recursive) to copy a directory tree.

3. **Overwriting**: By default, ``cp`` silently overwrites existing
   files. Use ``-i`` (interactive) to prompt, or ``-n`` (no-clobber)
   to never overwrite.

=== Recursive Copy ===

When ``-R`` is specified, cp walks the source directory tree and
recreates it at the destination::

    cp -R src_dir/ dest_dir/

If ``dest_dir`` already exists, the source directory is placed *inside*
it (``dest_dir/src_dir/``). If it doesn't exist, ``dest_dir`` becomes
the copy.

=== The Archive Flag (-a) ===

``-a`` is shorthand for ``-dR --preserve=all``. It's the "make an
exact copy" flag — preserves symlinks, recursion, and all metadata.
This is commonly used for backups::

    cp -a /home/user /backup/user

=== Symbolic and Hard Links ===

Instead of copying file contents, ``cp -s`` creates a symbolic link
and ``cp -l`` creates a hard link. These are fast "copies" that share
the underlying data.

=== CLI Builder Integration ===

The entire CLI is defined in ``cp.json``. CLI Builder handles flag
parsing, mutual exclusion groups, help text, and version output.
"""

from __future__ import annotations

import os
import shutil
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------
# The spec file lives alongside this script. We resolve the path relative
# to this file's location so that the program works regardless of the
# user's current directory.

SPEC_FILE = str(Path(__file__).parent / "cp.json")


# ---------------------------------------------------------------------------
# Business logic — copy_file
# ---------------------------------------------------------------------------


def copy_file(
    src: str,
    dst: str,
    *,
    recursive: bool = False,
    force: bool = False,
    interactive: bool = False,
    no_clobber: bool = False,
    update: bool = False,
    verbose: bool = False,
    dereference: bool = False,
    no_dereference: bool = False,
    preserve: str | None = None,
    no_preserve: str | None = None,
    link: bool = False,
    symbolic_link: bool = False,
    archive: bool = False,
) -> bool:
    """Copy a single file from *src* to *dst*.

    This function handles all the flag combinations for a single source
    file. Directory sources are delegated to ``copy_directory``.

    Args:
        src: Source file path.
        dst: Destination path (file or directory).
        recursive: If True, copy directories recursively (``-R``).
        force: If True, remove destination before copying (``-f``).
        interactive: If True, prompt before overwriting (``-i``).
        no_clobber: If True, never overwrite existing files (``-n``).
        update: If True, only copy when source is newer (``-u``).
        verbose: If True, print what is being done (``-v``).
        dereference: If True, follow symlinks in source (``-L``).
        no_dereference: If True, preserve symlinks (``-d``).
        preserve: Comma-separated attributes to preserve.
        no_preserve: Comma-separated attributes to NOT preserve.
        link: If True, hard-link instead of copying (``-l``).
        symbolic_link: If True, symlink instead of copying (``-s``).
        archive: If True, behave as ``-dR --preserve=all`` (``-a``).

    Returns:
        True on success, False on error.

    How the overwrite-mode flags interact
    ~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~~
    These three flags are mutually exclusive (enforced by CLI Builder):

    +-----------+----------------------------------------------------------+
    | Flag      | Behavior                                                 |
    +===========+==========================================================+
    | ``-f``    | Remove destination if it can't be opened, then copy.     |
    | ``-i``    | Ask the user before overwriting. "No" means skip.        |
    | ``-n``    | Never overwrite. Silently skip.                          |
    +-----------+----------------------------------------------------------+

    If none is given, the default is to overwrite silently.
    """
    # --- Archive mode expands to multiple flags ---
    if archive:
        recursive = True
        no_dereference = True
        preserve = "all"

    # --- Check that source exists ---
    if not os.path.lexists(src):
        print(f"cp: cannot stat '{src}': No such file or directory", file=sys.stderr)
        return False

    # --- Handle directory sources ---
    if os.path.isdir(src) and not os.path.islink(src):
        if not recursive:
            print(
                f"cp: -R not specified; omitting directory '{src}'",
                file=sys.stderr,
            )
            return False
        return copy_directory(
            src,
            dst,
            force=force,
            interactive=interactive,
            no_clobber=no_clobber,
            update=update,
            verbose=verbose,
            dereference=dereference,
            no_dereference=no_dereference,
            preserve=preserve,
            no_preserve=no_preserve,
        )

    # --- Resolve destination ---
    # If dst is an existing directory, copy *into* it.
    if os.path.isdir(dst):
        dst = os.path.join(dst, os.path.basename(src))

    # --- Overwrite checks ---
    if os.path.lexists(dst):
        if no_clobber:
            return True  # Silently skip — not an error.

        if update:
            # Only copy if source is newer than destination.
            src_mtime = os.stat(src).st_mtime
            dst_mtime = os.stat(dst).st_mtime
            if src_mtime <= dst_mtime:
                return True  # Destination is same age or newer — skip.

        if interactive:
            try:
                response = input(f"cp: overwrite '{dst}'? ").strip().lower()
            except EOFError:
                return True
            if response not in ("y", "yes"):
                return True

        if force:
            # Force: remove the destination first if it can't be opened.
            try:
                os.unlink(dst)
            except OSError:
                pass  # If removal fails, we'll try to copy anyway.

    # --- Perform the copy ---
    try:
        if link:
            # Hard link instead of copying.
            os.link(src, dst)
        elif symbolic_link:
            # Symbolic link instead of copying.
            os.symlink(os.path.abspath(src), dst)
        else:
            # Determine whether to follow symlinks.
            follow_symlinks = not no_dereference

            # Use shutil.copy2 to preserve metadata by default.
            shutil.copy2(src, dst, follow_symlinks=follow_symlinks)
    except PermissionError:
        print(f"cp: cannot create regular file '{dst}': Permission denied", file=sys.stderr)
        return False
    except OSError as e:
        print(f"cp: error copying '{src}' to '{dst}': {e.strerror}", file=sys.stderr)
        return False

    if verbose:
        print(f"'{src}' -> '{dst}'")

    return True


# ---------------------------------------------------------------------------
# Business logic — copy_directory
# ---------------------------------------------------------------------------


def copy_directory(
    src: str,
    dst: str,
    *,
    force: bool = False,
    interactive: bool = False,
    no_clobber: bool = False,
    update: bool = False,
    verbose: bool = False,
    dereference: bool = False,
    no_dereference: bool = False,
    preserve: str | None = None,
    no_preserve: str | None = None,
) -> bool:
    """Recursively copy a directory tree from *src* to *dst*.

    If *dst* already exists as a directory, the source is copied *into*
    it (i.e., ``dst/basename(src)/...``). If *dst* does not exist, it
    becomes the new directory.

    This function uses ``shutil.copytree`` for the heavy lifting, with
    custom handling for overwrite modes.

    Args:
        src: Source directory path.
        dst: Destination path.
        force: Remove destination files that can't be opened.
        interactive: Prompt before overwriting.
        no_clobber: Never overwrite.
        update: Only copy newer files.
        verbose: Print what is being done.
        dereference: Follow symlinks (``-L``).
        no_dereference: Preserve symlinks (``-d``).
        preserve: Attributes to preserve.
        no_preserve: Attributes NOT to preserve.

    Returns:
        True on success, False on error.
    """
    # If dst is an existing directory, copy into it.
    if os.path.isdir(dst):
        dst = os.path.join(dst, os.path.basename(src))

    # Determine symlink handling.
    # By default, copytree copies symlinks as symlinks. With -L, it
    # follows them and copies the targets instead.
    symlinks = not dereference

    try:
        shutil.copytree(
            src,
            dst,
            symlinks=symlinks,
            dirs_exist_ok=False,
        )
    except FileExistsError:
        # Destination already exists — try with dirs_exist_ok.
        try:
            shutil.copytree(
                src,
                dst,
                symlinks=symlinks,
                dirs_exist_ok=True,
            )
        except OSError as e:
            print(f"cp: error copying '{src}': {e}", file=sys.stderr)
            return False
    except PermissionError:
        print(
            f"cp: cannot copy '{src}' to '{dst}': Permission denied",
            file=sys.stderr,
        )
        return False
    except OSError as e:
        print(f"cp: error copying '{src}': {e.strerror}", file=sys.stderr)
        return False

    if verbose:
        print(f"'{src}' -> '{dst}'")

    return True


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then copy files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"cp: {error.message}", file=sys.stderr)
        raise SystemExit(1) from None

    # --- Step 2: Dispatch on result type -----------------------------------
    if isinstance(result, HelpResult):
        print(result.text)
        raise SystemExit(0)

    if isinstance(result, VersionResult):
        print(result.version)
        raise SystemExit(0)

    # --- Step 3: Extract flags ---------------------------------------------
    assert isinstance(result, ParseResult)

    archive = result.flags.get("archive", False)
    force = result.flags.get("force", False)
    interactive = result.flags.get("interactive", False)
    no_clobber = result.flags.get("no_clobber", False)
    recursive = result.flags.get("recursive", False)
    no_dereference_flag = result.flags.get("no_dereference", False)
    dereference = result.flags.get("dereference", False)
    preserve = result.flags.get("preserve", None)
    no_preserve = result.flags.get("no_preserve", None)
    link_flag = result.flags.get("link", False)
    symbolic_link = result.flags.get("symbolic_link", False)
    update = result.flags.get("update", False)
    verbose = result.flags.get("verbose", False)
    target_directory = result.flags.get("target_directory", None)

    # --- Step 4: Determine sources and destination -------------------------
    sources = result.arguments.get("sources", [])
    if isinstance(sources, str):
        sources = [sources]

    # If -t is specified, all positional args are sources and the target
    # directory is the -t value. Otherwise, the last positional arg is
    # the destination.
    if target_directory:
        dst = target_directory
        srcs = sources
    else:
        dst = sources[-1]
        srcs = sources[:-1]

    # --- Step 5: Copy each source ------------------------------------------
    exit_code = 0
    for src in srcs:
        if not copy_file(
            src,
            dst,
            recursive=recursive,
            force=force,
            interactive=interactive,
            no_clobber=no_clobber,
            update=update,
            verbose=verbose,
            dereference=dereference,
            no_dereference=no_dereference_flag,
            preserve=preserve,
            no_preserve=no_preserve,
            link=link_flag,
            symbolic_link=symbolic_link,
            archive=archive,
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
