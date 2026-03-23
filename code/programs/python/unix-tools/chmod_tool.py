"""chmod -- change file mode bits.

=== What This Program Does ===

This is a reimplementation of the GNU ``chmod`` utility. It changes the
permission bits (mode) of files and directories::

    chmod 755 script.sh          # Octal mode: rwxr-xr-x
    chmod u+x script.sh          # Symbolic: add execute for user
    chmod -R go-w directory/     # Recursive: remove write for group/other

=== How Unix Permissions Work ===

Every file in Unix has a 12-bit mode that controls access. The lower
9 bits form three groups of three::

    rwx  rwx  rwx
    ^^^  ^^^  ^^^
    |    |    |
    |    |    +-- Other (everyone else)
    |    +------- Group (members of the file's group)
    +------------ User/Owner (the file's owner)

Each group has three bits:

+------+-------+-------+
| Bit  | Octal | Meaning |
+======+=======+=========+
| r    | 4     | Read    |
| w    | 2     | Write   |
| x    | 1     | Execute |
+------+-------+-------+

So ``755`` means:
- User: 7 = 4+2+1 = rwx (read, write, execute)
- Group: 5 = 4+0+1 = r-x (read, execute)
- Other: 5 = 4+0+1 = r-x (read, execute)

=== Octal vs Symbolic Modes ===

**Octal** (e.g., ``755``, ``644``): Directly sets all permission bits.
This is absolute — the old permissions are completely replaced.

**Symbolic** (e.g., ``u+x``, ``go-w``, ``a=r``): Modifies specific
bits relative to the current permissions. The syntax is::

    [ugoa][+-=][rwxXst]

Where:
- ``u`` = user, ``g`` = group, ``o`` = other, ``a`` = all
- ``+`` = add, ``-`` = remove, ``=`` = set exactly
- ``r`` = read, ``w`` = write, ``x`` = execute
- ``X`` = execute only if directory or already executable
- ``s`` = setuid/setgid, ``t`` = sticky bit

Multiple clauses can be comma-separated: ``u+x,go-w``

=== CLI Builder Integration ===

The CLI is defined in ``chmod.json``. CLI Builder handles flag parsing.
"""

from __future__ import annotations

import os
import re
import stat
import sys
from pathlib import Path

# ---------------------------------------------------------------------------
# Locate the JSON spec file.
# ---------------------------------------------------------------------------

SPEC_FILE = str(Path(__file__).parent / "chmod.json")


# ---------------------------------------------------------------------------
# Parse permission modes -- octal and symbolic.
# ---------------------------------------------------------------------------
# This is the heart of chmod. We need to handle both "755" (octal)
# and "u+rwx,go+rx" (symbolic) formats.


def parse_octal_mode(mode_str: str) -> int | None:
    """Try to parse a mode string as an octal number.

    Valid octal modes are 1-4 digit strings using digits 0-7.
    For example: "755", "0644", "1777" (with sticky bit).

    Args:
        mode_str: The mode string to parse.

    Returns:
        The mode as an integer, or None if it's not a valid octal mode.
    """
    if re.match(r"^[0-7]{1,4}$", mode_str):
        return int(mode_str, 8)
    return None


# ---------------------------------------------------------------------------
# Permission bit maps for symbolic mode parsing.
# ---------------------------------------------------------------------------
# These maps convert symbolic characters to their corresponding bit
# positions. We use the stat module constants for clarity.

_WHO_BITS = {
    "u": stat.S_IRWXU,  # 0o700 — user bits
    "g": stat.S_IRWXG,  # 0o070 — group bits
    "o": stat.S_IRWXO,  # 0o007 — other bits
}

_PERM_BITS = {
    "r": stat.S_IRUSR | stat.S_IRGRP | stat.S_IROTH,  # Read bits for all
    "w": stat.S_IWUSR | stat.S_IWGRP | stat.S_IWOTH,  # Write bits for all
    "x": stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH,  # Execute bits for all
    "s": stat.S_ISUID | stat.S_ISGID,                   # Setuid/setgid
    "t": stat.S_ISVTX,                                  # Sticky bit
}


def apply_symbolic_mode(
    mode_str: str,
    current_mode: int,
    is_directory: bool = False,
) -> int:
    """Apply a symbolic mode string to an existing mode.

    This function parses symbolic mode strings like ``u+x``, ``go-w``,
    ``a=rwx``, or comma-separated combinations like ``u+x,go-w``.

    The parsing algorithm:

    1. Split on commas to get individual clauses.
    2. For each clause, extract:
       a. WHO: which permission groups to modify (u/g/o/a)
       b. OPERATOR: how to modify (+/-/=)
       c. PERMISSIONS: which bits to change (r/w/x/X/s/t)
    3. Apply the operator to compute the new mode.

    Args:
        mode_str: The symbolic mode string (e.g., "u+x,go-w").
        current_mode: The current file mode (integer).
        is_directory: Whether the target is a directory (affects X).

    Returns:
        The new mode (integer).
    """
    new_mode = current_mode

    for clause in mode_str.split(","):
        clause = clause.strip()
        if not clause:
            continue

        # Parse who (u/g/o/a).
        who_chars = ""
        i = 0
        while i < len(clause) and clause[i] in "ugoa":
            who_chars += clause[i]
            i += 1

        # If no who is specified, default to "a" (all).
        if not who_chars:
            who_chars = "a"

        # Parse operator (+/-/=).
        if i >= len(clause) or clause[i] not in "+-=":
            continue  # Invalid clause, skip.
        operator = clause[i]
        i += 1

        # Parse permissions (r/w/x/X/s/t).
        perm_chars = clause[i:]

        # Build the who mask (which groups are affected).
        if "a" in who_chars:
            who_mask = stat.S_IRWXU | stat.S_IRWXG | stat.S_IRWXO
        else:
            who_mask = 0
            for ch in who_chars:
                who_mask |= _WHO_BITS.get(ch, 0)

        # Build the permission bits.
        perm_bits = 0
        for ch in perm_chars:
            if ch == "X":
                # X = execute only if directory or already executable.
                exec_bits = stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
                if is_directory or (current_mode & exec_bits):
                    perm_bits |= stat.S_IXUSR | stat.S_IXGRP | stat.S_IXOTH
            elif ch == "s":
                # Setuid/setgid: only applies to user and group.
                if "u" in who_chars or "a" in who_chars:
                    perm_bits |= stat.S_ISUID
                if "g" in who_chars or "a" in who_chars:
                    perm_bits |= stat.S_ISGID
            elif ch == "t":
                perm_bits |= stat.S_ISVTX
            elif ch in _PERM_BITS:
                perm_bits |= _PERM_BITS[ch]

        # Apply the who mask to limit perm_bits to the specified groups.
        # (But special bits like setuid/sticky are not masked.)
        special_bits = perm_bits & (stat.S_ISUID | stat.S_ISGID | stat.S_ISVTX)
        regular_bits = perm_bits & ~(stat.S_ISUID | stat.S_ISGID | stat.S_ISVTX)
        effective_bits = (regular_bits & who_mask) | special_bits

        # Apply the operator.
        if operator == "+":
            new_mode |= effective_bits
        elif operator == "-":
            new_mode &= ~effective_bits
        elif operator == "=":
            # Clear the who bits, then set.
            new_mode &= ~who_mask
            new_mode |= effective_bits

    return new_mode


# ---------------------------------------------------------------------------
# Business logic -- change file permissions.
# ---------------------------------------------------------------------------


def chmod_file(
    path: str,
    mode_str: str,
    *,
    recursive: bool = False,
    verbose: bool = False,
    changes: bool = False,
    silent: bool = False,
) -> bool:
    """Change the permissions of a file or directory.

    Args:
        path: Path to the file or directory.
        mode_str: The mode to apply (octal or symbolic).
        recursive: If True, apply recursively to directories.
        verbose: If True, print every file processed.
        changes: If True, print only when a change is made.
        silent: If True, suppress error messages.

    Returns:
        True on success, False on error.
    """
    try:
        current_stat = os.stat(path)
    except FileNotFoundError:
        if not silent:
            print(
                f"chmod: cannot access '{path}': No such file or directory",
                file=sys.stderr,
            )
        return False
    except PermissionError:
        if not silent:
            print(
                f"chmod: cannot access '{path}': Permission denied",
                file=sys.stderr,
            )
        return False

    current_mode = stat.S_IMODE(current_stat.st_mode)
    is_directory = stat.S_ISDIR(current_stat.st_mode)

    # Determine the new mode.
    octal = parse_octal_mode(mode_str)
    if octal is not None:
        new_mode = octal
    else:
        new_mode = apply_symbolic_mode(mode_str, current_mode, is_directory)

    # Apply the mode.
    try:
        os.chmod(path, new_mode)
    except PermissionError:
        if not silent:
            print(
                f"chmod: changing permissions of '{path}': Operation not permitted",
                file=sys.stderr,
            )
        return False
    except OSError as e:
        if not silent:
            msg = f"chmod: changing permissions of '{path}': {e.strerror}"
            print(msg, file=sys.stderr)
        return False

    # Report changes.
    if verbose or (changes and new_mode != current_mode):
        old_str = format(current_mode, "o")
        new_str = format(new_mode, "o")
        print(f"mode of '{path}' changed from {old_str} to {new_str}")

    # Recurse into directories.
    if recursive and is_directory:
        success = True
        try:
            for entry in os.listdir(path):
                child = os.path.join(path, entry)
                if not chmod_file(
                    child, mode_str,
                    recursive=True,
                    verbose=verbose,
                    changes=changes,
                    silent=silent,
                ):
                    success = False
        except PermissionError:
            if not silent:
                print(
                    f"chmod: cannot read directory '{path}': Permission denied",
                    file=sys.stderr,
                )
            return False
        return success

    return True


# ---------------------------------------------------------------------------
# CLI entry point
# ---------------------------------------------------------------------------


def main() -> None:
    """Entry point: parse args via CLI Builder, then chmod files."""
    from cli_builder import HelpResult, ParseErrors, Parser, ParseResult, VersionResult

    try:
        result = Parser(SPEC_FILE, sys.argv).parse()
    except ParseErrors as exc:
        for error in exc.errors:
            print(f"chmod: {error.message}", file=sys.stderr)
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
    reference = result.flags.get("reference", None)

    # --- Extract arguments ---
    mode_str = result.arguments.get("mode", "")
    files = result.arguments.get("files", [])
    if isinstance(files, str):
        files = [files]

    # If --reference is used, get the mode from the reference file.
    if reference:
        try:
            ref_stat = os.stat(reference)
            mode_str = format(stat.S_IMODE(ref_stat.st_mode), "o")
        except FileNotFoundError:
            print(
                f"chmod: cannot stat '{reference}': No such file or directory",
                file=sys.stderr,
            )
            raise SystemExit(1) from None

    # --- Apply chmod ---
    exit_code = 0
    for filepath in files:
        if not chmod_file(
            filepath, mode_str,
            recursive=recursive,
            verbose=verbose,
            changes=changes_flag,
            silent=silent,
        ):
            exit_code = 1

    raise SystemExit(exit_code)


if __name__ == "__main__":
    main()
