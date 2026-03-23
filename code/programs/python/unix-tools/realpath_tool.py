"""realpath — print the resolved absolute path.

=== What This Program Does ===

This is a reimplementation of the GNU ``realpath`` utility. It resolves
each given path to an absolute path by:

1. Converting relative paths to absolute paths.
2. Resolving symbolic links (following the chain until a real file).
3. Removing ``.`` (current directory) and ``..`` (parent directory)
   components.
4. Removing redundant ``/`` separators.

=== Why realpath Is Useful ===

File paths in Unix can be ambiguous. Consider::

    $ ls -la /usr/bin/python
    lrwxr-xr-x  1 root  wheel  7 Jan  1 12:00 /usr/bin/python -> python3

    $ realpath /usr/bin/python
    /usr/local/bin/python3.12

realpath shows you where the file *actually* lives on disk, following
all symlinks. This is essential for scripts that need to know the
canonical location of a file.

=== Modes of Operation ===

realpath has three modes for handling path existence:

- **Default**: Resolve symlinks for existing components; the final
  component need not exist.
- ``-e`` (canonicalize-existing): ALL components must exist.
- ``-m`` (canonicalize-missing): NO components need exist.

=== Relative Output ===

By default, realpath prints absolute paths. The ``--relative-to``
and ``--relative-base`` flags change the output to relative paths:

- ``--relative-to=DIR``: Print the path relative to DIR.
- ``--relative-base=DIR``: Print relative paths only if the resolved
  path starts with DIR.

=== CLI Builder Integration ===

The entire CLI is defined in ``realpath.json``. CLI Builder handles
flag parsing, help text, and version output.
"""

from __future__ import annotations

import os
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "realpath.json")


def resolve_path(
    filepath: str,
    *,
    canonicalize_existing: bool,
    canonicalize_missing: bool,
    no_symlinks: bool,
) -> str | None:
    """Resolve a file path to its canonical form.

    This function implements the three modes of path resolution:

    1. Default: Resolve symlinks, final component need not exist.
    2. -e mode: All components must exist.
    3. -m mode: No components need exist.

    When ``no_symlinks`` is True, symlinks are not resolved; only
    ``.`` and ``..`` are processed and the path is made absolute.

    Args:
        filepath: The path to resolve.
        canonicalize_existing: If True, all components must exist.
        canonicalize_missing: If True, no components need exist.
        no_symlinks: If True, don't resolve symlinks.

    Returns:
        The resolved path as a string, or None on error.
    """
    if no_symlinks:
        # Absolute path without symlink resolution.
        return os.path.abspath(filepath)

    if canonicalize_existing:
        # All components must exist. os.path.realpath resolves symlinks
        # but doesn't check existence, so we check manually.
        resolved = os.path.realpath(filepath)
        if not os.path.exists(resolved):
            return None
        return resolved

    if canonicalize_missing:
        # No components need exist. Just resolve what we can.
        return os.path.realpath(filepath)

    # Default mode: resolve symlinks, but don't require the final
    # component to exist. os.path.realpath handles this well.
    return os.path.realpath(filepath)


def make_relative(
    resolved: str,
    *,
    relative_to: str | None,
    relative_base: str | None,
) -> str:
    """Convert an absolute path to a relative one if requested.

    Args:
        resolved: The resolved absolute path.
        relative_to: If set, compute the path relative to this directory.
        relative_base: If set, only relativize paths under this directory.

    Returns:
        The (possibly relative) path.
    """
    if relative_to is not None:
        base = os.path.realpath(relative_to)
        return os.path.relpath(resolved, base)

    if relative_base is not None:
        base = os.path.realpath(relative_base)
        if resolved.startswith(base + os.sep) or resolved == base:
            return os.path.relpath(resolved, base)
        # Path is not under base; return absolute.
        return resolved

    return resolved


def main() -> None:
    """Entry point: parse args via CLI Builder, then resolve paths."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"realpath: {error.message}", file=sys.stderr)
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

    canonicalize_existing = result.flags.get("canonicalize_existing", False)
    canonicalize_missing = result.flags.get("canonicalize_missing", False)
    no_symlinks = result.flags.get("no_symlinks", False)
    quiet = result.flags.get("quiet", False)
    relative_to = result.flags.get("relative_to")
    relative_base = result.flags.get("relative_base")
    zero = result.flags.get("zero", False)

    # The line terminator: NUL for -z, newline otherwise.
    terminator = "\0" if zero else "\n"

    # Get the list of files.
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    exit_code = 0
    for filepath in files:
        resolved = resolve_path(
            filepath,
            canonicalize_existing=canonicalize_existing,
            canonicalize_missing=canonicalize_missing,
            no_symlinks=no_symlinks,
        )

        if resolved is None:
            if not quiet:
                print(
                    f"realpath: {filepath}: No such file or directory",
                    file=sys.stderr,
                )
            exit_code = 1
            continue

        # Apply relative path conversion.
        output = make_relative(
            resolved, relative_to=relative_to, relative_base=relative_base
        )

        sys.stdout.write(output + terminator)

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
