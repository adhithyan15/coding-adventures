"""mv -- move (rename) files.

=== What This Program Does ===

This is a reimplementation of the GNU ``mv`` utility. It moves files
and directories from one location to another. Moving is conceptually
two operations:

1. Copy the file to the new location.
2. Remove the original.

However, when source and destination are on the **same filesystem**,
``mv`` is implemented as a simple *rename* — no data is copied at all.
This makes ``mv`` extremely fast for same-filesystem moves, regardless
of file size.

=== Rename vs Cross-Filesystem Move ===

::

    Same filesystem:
    ┌──────────┐       ┌──────────┐
    │ old_name │──────>│ new_name │   (just updates directory entry)
    └──────────┘       └──────────┘
    Time: O(1), regardless of file size

    Different filesystems:
    ┌──────────┐  copy  ┌──────────┐  delete  ┌──────────┐
    │  source  │──────> │   dest   │   ──>    │ (source) │
    └──────────┘        └──────────┘          └──────────┘
    Time: O(n), proportional to file size

Python's ``shutil.move`` handles both cases automatically.

=== Overwrite Behavior ===

Like ``cp``, ``mv`` has three mutually exclusive overwrite modes:

+-----------+----------------------------------------------------------+
| Flag      | Behavior                                                 |
+===========+==========================================================+
| ``-f``    | Never prompt — just overwrite (default behavior).        |
| ``-i``    | Prompt before overwriting.                               |
| ``-n``    | Never overwrite — silently skip.                         |
+-----------+----------------------------------------------------------+

=== The -u Flag (Update) ===

With ``-u``, mv only moves when the source is *newer* than the
destination (or the destination doesn't exist). This is useful for
incremental moves.

=== CLI Builder Integration ===

The entire CLI is defined in ``mv.json``. CLI Builder handles flag
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

SPEC_FILE = str(Path(__file__).parent / "mv.json")


# ---------------------------------------------------------------------------
# Business logic
# ---------------------------------------------------------------------------


def move_file(
    src: str,
    dst: str,
    *,
    force: bool = False,
    interactive: bool = False,
    no_clobber: bool = False,
    update: bool = False,
    verbose: bool = False,
) -> bool:
    """Move a single file or directory from *src* to *dst*.

    This function handles all overwrite-mode flag combinations. It
    delegates to ``shutil.move``, which automatically chooses between
    ``os.rename`` (same filesystem) and copy-then-delete (cross-
    filesystem).

    Args:
        src: Source file or directory path.
        dst: Destination path (file or directory).
        force: If True, never prompt before overwriting (``-f``).
        interactive: If True, prompt before overwriting (``-i``).
        no_clobber: If True, never overwrite existing files (``-n``).
        update: If True, only move when source is newer (``-u``).
        verbose: If True, explain what is being done (``-v``).

    Returns:
        True on success, False on error.
    """
    # --- Check that source exists ---
    if not os.path.lexists(src):
        print(
            f"mv: cannot stat '{src}': No such file or directory",
            file=sys.stderr,
        )
        return False

    # --- Resolve destination ---
    # If dst is an existing directory, move *into* it.
    actual_dst = dst
    if os.path.isdir(dst) and not os.path.islink(dst):
        actual_dst = os.path.join(dst, os.path.basename(src))

    # --- Overwrite checks ---
    if os.path.lexists(actual_dst):
        if no_clobber:
            return True  # Silently skip — not an error.

        if update:
            # Only move if source is newer than destination.
            src_mtime = os.stat(src).st_mtime
            dst_mtime = os.stat(actual_dst).st_mtime
            if src_mtime <= dst_mtime:
                return True

        if interactive and not force:
            try:
                response = input(f"mv: overwrite '{actual_dst}'? ").strip().lower()
            except EOFError:
                return True
            if response not in ("y", "yes"):
                return True

    # --- Perform the move ---
    try:
        shutil.move(src, actual_dst)
    except PermissionError:
        print(
            f"mv: cannot move '{src}' to '{actual_dst}': Permission denied",
            file=sys.stderr,
        )
        return False
    except OSError as e:
        print(
            f"mv: cannot move '{src}' to '{actual_dst}': {e.strerror}",
            file=sys.stderr,
        )
        return False

    if verbose:
        print(f"renamed '{src}' -> '{actual_dst}'")

    return True


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then move files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"mv: {error.message}", file=sys.stderr)
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

    force = result.flags.get("force", False)
    interactive = result.flags.get("interactive", False)
    no_clobber = result.flags.get("no_clobber", False)
    update = result.flags.get("update", False)
    verbose = result.flags.get("verbose", False)
    target_directory = result.flags.get("target_directory", None)

    # --- Step 4: Determine sources and destination -------------------------
    sources = result.arguments.get("sources", [])
    if isinstance(sources, str):
        sources = [sources]

    if target_directory:
        dst = target_directory
        srcs = sources
    else:
        dst = sources[-1]
        srcs = sources[:-1]

    # --- Step 5: Move each source ------------------------------------------
    exit_code = 0
    for src in srcs:
        if not move_file(
            src,
            dst,
            force=force,
            interactive=interactive,
            no_clobber=no_clobber,
            update=update,
            verbose=verbose,
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
