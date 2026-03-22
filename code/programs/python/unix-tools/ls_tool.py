"""ls -- list directory contents.

=== What This Program Does ===

This is a reimplementation of the GNU ``ls`` utility. It lists files
and directories, optionally with detailed metadata like permissions,
sizes, and timestamps.

=== Output Formats ===

``ls`` has several output modes:

1. **Default** (multi-column): Names are listed in columns, sorted
   alphabetically. Hidden files (starting with ``.``) are omitted.

2. **Long format** (``-l``): One entry per line, with metadata::

       -rw-r--r-- 1 alice staff  4096 Jan 15 10:30 file.txt
       drwxr-xr-x 3 alice staff   96 Jan 15 10:30 subdir/

   The columns are: permissions, link count, owner, group, size,
   modification time, and name.

3. **One-per-line** (``-1``): Just names, one per line. Useful for
   piping into other commands.

=== Sorting ===

By default, entries are sorted alphabetically. Sort can be changed:

+-------+--------------------------------+
| Flag  | Sort order                     |
+=======+================================+
| (none)| Alphabetical (default)         |
| ``-S``| By file size, largest first    |
| ``-t``| By modification time, newest   |
| ``-X``| By file extension              |
| ``-v``| By version number (natural)    |
| ``-U``| Unsorted (directory order)     |
+-------+--------------------------------+

All sort orders can be reversed with ``-r``.

=== Hidden Files ===

+-------+------------------------------------------+
| Flag  | Behavior                                 |
+=======+==========================================+
| (none)| Hide entries starting with ``.``         |
| ``-a``| Show all entries, including ``.`` and ``..``|
| ``-A``| Show hidden entries, but omit ``.`` and ``..``|
+-------+------------------------------------------+

=== Human-Readable Sizes ===

With ``-h``, sizes are formatted with K, M, G suffixes (powers of
1024). With ``--si``, powers of 1000 are used instead.

=== The -F Flag (Classify) ===

Appends an indicator character to each entry:

- ``/`` for directories
- ``*`` for executables
- ``@`` for symlinks
- ``|`` for FIFOs
- ``=`` for sockets

=== CLI Builder Integration ===

The entire CLI is defined in ``ls.json``.
"""

from __future__ import annotations

import grp
import os
import pwd
import stat
import sys
import time
from pathlib import Path
from typing import Any

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "ls.json")


# ---------------------------------------------------------------------------
# Data types
# ---------------------------------------------------------------------------


class LsOptions:
    """Container for all ls option flags.

    Rather than passing a dozen booleans around, we bundle them into
    this class. Each field corresponds to a CLI flag.
    """

    def __init__(self, **kwargs: Any) -> None:  # noqa: ANN401
        self.show_all: bool = kwargs.get("show_all", False)
        self.almost_all: bool = kwargs.get("almost_all", False)
        self.long_format: bool = kwargs.get("long_format", False)
        self.human_readable: bool = kwargs.get("human_readable", False)
        self.si: bool = kwargs.get("si", False)
        self.reverse: bool = kwargs.get("reverse", False)
        self.recursive: bool = kwargs.get("recursive", False)
        self.sort_by_size: bool = kwargs.get("sort_by_size", False)
        self.sort_by_time: bool = kwargs.get("sort_by_time", False)
        self.sort_by_extension: bool = kwargs.get("sort_by_extension", False)
        self.sort_by_version: bool = kwargs.get("sort_by_version", False)
        self.unsorted: bool = kwargs.get("unsorted", False)
        self.directory: bool = kwargs.get("directory", False)
        self.classify: bool = kwargs.get("classify", False)
        self.inode: bool = kwargs.get("inode", False)
        self.no_group: bool = kwargs.get("no_group", False)
        self.numeric_uid_gid: bool = kwargs.get("numeric_uid_gid", False)
        self.one_per_line: bool = kwargs.get("one_per_line", False)
        self.dereference: bool = kwargs.get("dereference", False)


# ---------------------------------------------------------------------------
# Business logic — formatting helpers
# ---------------------------------------------------------------------------


def format_size(size: int, human_readable: bool = False, si: bool = False) -> str:
    """Format a file size for display.

    Args:
        size: Size in bytes.
        human_readable: If True, use K/M/G suffixes (powers of 1024).
        si: If True, use k/M/G suffixes (powers of 1000).

    Returns:
        Formatted size string.

    Examples::

        >>> format_size(4096)
        '4096'
        >>> format_size(4096, human_readable=True)
        '4.0K'
        >>> format_size(1500, si=True)
        '1.5k'
    """
    if not human_readable and not si:
        return str(size)

    base = 1000 if si else 1024
    # The suffixes differ slightly between -h and --si.
    suffixes = ["", "k", "M", "G", "T", "P"] if si else ["", "K", "M", "G", "T", "P"]

    value = float(size)
    for suffix in suffixes:
        if abs(value) < base:
            if suffix == "":
                return str(size)
            return f"{value:.1f}{suffix}"
        value /= base

    # Extremely large files.
    return f"{value:.1f}{suffixes[-1]}"


def format_permissions(mode: int) -> str:
    """Convert a numeric mode to the classic ``-rwxr-xr-x`` string.

    The permission string is 10 characters:

    - Character 0: file type (``-`` file, ``d`` directory, ``l`` symlink, etc.)
    - Characters 1-3: owner read/write/execute
    - Characters 4-6: group read/write/execute
    - Characters 7-9: other read/write/execute

    Special bits (setuid, setgid, sticky) modify the execute character:

    - setuid: ``x`` -> ``s``, ``-`` -> ``S``
    - setgid: ``x`` -> ``s``, ``-`` -> ``S``
    - sticky: ``x`` -> ``t``, ``-`` -> ``T``

    Args:
        mode: The file mode (from os.stat).

    Returns:
        A 10-character permission string.
    """
    # File type character.
    if stat.S_ISDIR(mode):
        file_type = "d"
    elif stat.S_ISLNK(mode):
        file_type = "l"
    elif stat.S_ISFIFO(mode):
        file_type = "p"
    elif stat.S_ISSOCK(mode):
        file_type = "s"
    elif stat.S_ISBLK(mode):
        file_type = "b"
    elif stat.S_ISCHR(mode):
        file_type = "c"
    else:
        file_type = "-"

    # Permission bits.
    perms = ""
    for who, shift in [("USR", 6), ("GRP", 3), ("OTH", 0)]:
        r = "r" if mode & (stat.S_IRUSR >> (6 - shift)) else "-"
        w = "w" if mode & (stat.S_IWUSR >> (6 - shift)) else "-"
        x_bit = mode & (stat.S_IXUSR >> (6 - shift))

        if who == "USR" and mode & stat.S_ISUID:
            x = "s" if x_bit else "S"
        elif who == "GRP" and mode & stat.S_ISGID:
            x = "s" if x_bit else "S"
        elif who == "OTH" and mode & stat.S_ISVTX:
            x = "t" if x_bit else "T"
        else:
            x = "x" if x_bit else "-"

        perms += r + w + x

    return file_type + perms


def format_time(mtime: float) -> str:
    """Format a modification time for long listing.

    GNU ls uses two formats depending on age:

    - Files modified within the last 6 months: ``Mon DD HH:MM``
    - Older files: ``Mon DD  YYYY``

    Args:
        mtime: Modification time as a Unix timestamp.

    Returns:
        Formatted time string.
    """
    now = time.time()
    six_months = 180 * 24 * 3600

    t = time.localtime(mtime)
    if abs(now - mtime) < six_months:
        return time.strftime("%b %d %H:%M", t)
    else:
        return time.strftime("%b %d  %Y", t)


def classify_suffix(mode: int) -> str:
    """Return the classification suffix for -F flag.

    Args:
        mode: The file mode.

    Returns:
        A single character or empty string.
    """
    if stat.S_ISDIR(mode):
        return "/"
    if stat.S_ISLNK(mode):
        return "@"
    if stat.S_ISFIFO(mode):
        return "|"
    if stat.S_ISSOCK(mode):
        return "="
    if mode & (stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH):
        return "*"
    return ""


# ---------------------------------------------------------------------------
# Business logic — entry formatting
# ---------------------------------------------------------------------------


def format_entry(
    name: str,
    st: os.stat_result,
    opts: LsOptions,
    *,
    link_target: str | None = None,
) -> str:
    """Format a single directory entry for display.

    In long format, this produces a full metadata line. In short format,
    it just returns the name (possibly with a classify suffix).

    Args:
        name: The file name (just the basename, not the full path).
        st: The stat result for this entry.
        opts: Display options.
        link_target: If this is a symlink, the target path.

    Returns:
        Formatted string for this entry.
    """
    parts: list[str] = []

    # Inode number (if -i).
    if opts.inode:
        parts.append(f"{st.st_ino}")

    if opts.long_format:
        # Permissions.
        perms = format_permissions(st.st_mode)

        # Link count.
        nlink = str(st.st_nlink)

        # Owner and group.
        if opts.numeric_uid_gid:
            owner = str(st.st_uid)
            group = str(st.st_gid)
        else:
            try:
                owner = pwd.getpwuid(st.st_uid).pw_name
            except (KeyError, OSError):
                owner = str(st.st_uid)
            try:
                group = grp.getgrgid(st.st_gid).gr_name
            except (KeyError, OSError):
                group = str(st.st_gid)

        # Size.
        size = format_size(st.st_size, opts.human_readable, opts.si)

        # Time.
        mtime = format_time(st.st_mtime)

        # Assemble the line.
        if opts.no_group:
            parts.append(f"{perms} {nlink} {owner} {size:>8} {mtime} {name}")
        else:
            parts.append(f"{perms} {nlink} {owner} {group} {size:>8} {mtime} {name}")

        # Symlink target.
        if link_target is not None:
            parts[-1] += f" -> {link_target}"
    else:
        display = name
        if opts.classify:
            display += classify_suffix(st.st_mode)
        parts.append(display)

    return " ".join(parts)


# ---------------------------------------------------------------------------
# Business logic — directory listing
# ---------------------------------------------------------------------------


def list_directory(
    path: str,
    opts: LsOptions,
) -> list[str]:
    """List the contents of a directory and return formatted lines.

    This function handles all the sorting, filtering, and formatting
    logic. The result is a list of strings ready for printing.

    Args:
        path: The directory to list.
        opts: Display options.

    Returns:
        A list of formatted lines.

    How filtering works
    ~~~~~~~~~~~~~~~~~~~~
    +------------------+-------------------------------------------+
    | opts.show_all    | Show everything, including ``.`` and ``..``|
    | opts.almost_all  | Show hidden files, but omit ``.``/``..``  |
    | (neither)        | Hide all entries starting with ``.``       |
    +------------------+-------------------------------------------+
    """
    # --- Handle -d (list the directory itself, not its contents) ---
    if opts.directory:
        try:
            st = os.lstat(path) if not opts.dereference else os.stat(path)
        except OSError as e:
            return [f"ls: cannot access '{path}': {e.strerror}"]
        link_target = None
        if stat.S_ISLNK(st.st_mode):
            try:
                link_target = os.readlink(path)
            except OSError:
                pass
        return [format_entry(path, st, opts, link_target=link_target)]

    # --- Gather entries ---
    try:
        entries = list(os.scandir(path))
    except PermissionError:
        return [f"ls: cannot open directory '{path}': Permission denied"]
    except OSError as e:
        return [f"ls: cannot access '{path}': {e.strerror}"]

    # Convert scandir entries to (name, stat, link_target) tuples.
    items: list[tuple[str, os.stat_result, str | None]] = []
    for entry in entries:
        name = entry.name

        # Filter hidden files.
        if not opts.show_all and not opts.almost_all:
            if name.startswith("."):
                continue

        try:
            if opts.dereference:
                st = entry.stat(follow_symlinks=True)
            else:
                st = entry.stat(follow_symlinks=False)
        except OSError:
            continue

        link_target = None
        if entry.is_symlink():
            try:
                link_target = os.readlink(entry.path)
            except OSError:
                pass

        items.append((name, st, link_target))

    # Add . and .. if -a is specified.
    if opts.show_all:
        for dot in [".", ".."]:
            dot_path = os.path.join(path, dot)
            try:
                st = os.lstat(dot_path) if not opts.dereference else os.stat(dot_path)
                items.append((dot, st, None))
            except OSError:
                pass

    # --- Sort ---
    items = _sort_entries(items, opts)

    # --- Format ---
    lines: list[str] = []
    for name, st, link_target in items:
        lines.append(format_entry(name, st, opts, link_target=link_target))

    # --- Recursive listing ---
    if opts.recursive:
        for name, st, _ in items:
            if stat.S_ISDIR(st.st_mode) and name not in (".", ".."):
                subpath = os.path.join(path, name)
                lines.append("")
                lines.append(f"{subpath}:")
                lines.extend(list_directory(subpath, opts))

    return lines


def _sort_entries(
    items: list[tuple[str, os.stat_result, str | None]],
    opts: LsOptions,
) -> list[tuple[str, os.stat_result, str | None]]:
    """Sort directory entries according to the specified sort mode.

    Args:
        items: List of (name, stat, link_target) tuples.
        opts: Display options containing sort preferences.

    Returns:
        Sorted list of tuples.
    """
    if opts.unsorted:
        return items

    if opts.sort_by_size:
        items.sort(key=lambda x: x[1].st_size, reverse=True)
    elif opts.sort_by_time:
        items.sort(key=lambda x: x[1].st_mtime, reverse=True)
    elif opts.sort_by_extension:
        items.sort(key=lambda x: _get_extension(x[0]))
    elif opts.sort_by_version:
        items.sort(key=lambda x: _version_key(x[0]))
    else:
        # Default: alphabetical (case-insensitive, then case-sensitive).
        items.sort(key=lambda x: x[0].lower())

    if opts.reverse:
        items.reverse()

    return items


def _get_extension(name: str) -> str:
    """Extract the extension from a filename for sorting.

    Files without extensions sort before files with extensions.
    The leading dot in hidden files is not treated as an extension.

    Args:
        name: The filename.

    Returns:
        The extension (including the dot), or empty string.
    """
    # Strip leading dots (hidden files).
    base = name.lstrip(".")
    if "." in base:
        return base[base.rfind("."):]
    return ""


def _version_key(name: str) -> list[int | str]:
    """Generate a sort key for version/natural sorting.

    This splits the name into alternating text and number segments,
    so that ``file2`` sorts before ``file10``.

    Args:
        name: The filename.

    Returns:
        A list of alternating strings and integers for comparison.

    Example::

        >>> _version_key("file10.txt")
        ['file', 10, '.txt']
    """
    import re

    parts: list[int | str] = []
    for segment in re.split(r"(\d+)", name):
        if segment.isdigit():
            parts.append(int(segment))
        else:
            parts.append(segment.lower())
    return parts


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then list files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    # --- Step 1: Parse arguments -------------------------------------------
    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"ls: {error.message}", file=sys.stderr)
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

    opts = LsOptions(
        show_all=result.flags.get("all", False),
        almost_all=result.flags.get("almost_all", False),
        long_format=result.flags.get("long", False),
        human_readable=result.flags.get("human_readable", False),
        si=result.flags.get("si", False),
        reverse=result.flags.get("reverse", False),
        recursive=result.flags.get("recursive", False),
        sort_by_size=result.flags.get("sort_by_size", False),
        sort_by_time=result.flags.get("sort_by_time", False),
        sort_by_extension=result.flags.get("sort_by_extension", False),
        sort_by_version=result.flags.get("sort_by_version", False),
        unsorted=result.flags.get("unsorted", False),
        directory=result.flags.get("directory", False),
        classify=result.flags.get("classify", False),
        inode=result.flags.get("inode", False),
        no_group=result.flags.get("no_group", False),
        numeric_uid_gid=result.flags.get("numeric_uid_gid", False),
        one_per_line=result.flags.get("one_per_line", False),
        dereference=result.flags.get("dereference", False),
    )

    # If -n is given, it implies -l.
    if opts.numeric_uid_gid:
        opts.long_format = True

    # --- Step 4: Determine files to list -----------------------------------
    files = result.arguments.get("files", ["."])
    if isinstance(files, str):
        files = [files]

    # --- Step 5: List each target ------------------------------------------
    show_headers = len(files) > 1

    for i, path in enumerate(files):
        if show_headers:
            if i > 0:
                print()
            print(f"{path}:")

        # If the path is a file (not a directory), show it directly.
        if os.path.isfile(path) or (os.path.islink(path) and not os.path.isdir(path)):
            try:
                st = os.lstat(path) if not opts.dereference else os.stat(path)
            except OSError as e:
                print(f"ls: cannot access '{path}': {e.strerror}", file=sys.stderr)
                continue
            link_target = None
            if os.path.islink(path):
                try:
                    link_target = os.readlink(path)
                except OSError:
                    pass
            print(format_entry(os.path.basename(path), st, opts, link_target=link_target))
        else:
            lines = list_directory(path, opts)
            for line in lines:
                print(line)


if __name__ == "__main__":
    main()
