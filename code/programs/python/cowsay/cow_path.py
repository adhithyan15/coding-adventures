"""Choosing a cow file safely.

``-f NAME`` / ``--file NAME`` picks which cow to draw. The name is glued into a
path -- ``<cows dir>/NAME.cow`` -- so it is untrusted input that decides which
file this program opens and echoes to stdout. Left unchecked, a name can walk
out of the cows directory (issue #12169)::

    name given                    path actually opened
    ----------------------------- ------------------------------------
    tux                           code/specs/cows/tux.cow     (fine)
    ../../../../home/me/notes     /home/me/notes.cow          (escape)
    /etc/secret                   /etc/secret.cow             (escape -- joining
                                  an absolute path DISCARDS the base directory)
    C:\\Users\\me\\x                 C:\\Users\\me\\x.cow           (Windows)

The forced ``.cow`` suffix limits what can be read, and a local CLI already
runs with its caller's privileges -- but a wrapper script, web service or CI
job that forwards someone else's string to ``cowsay -f`` would hand that
someone a file-read primitive. So we defend in two layers, mirroring the C#,
F#, Java, Kotlin, Perl, Haskell, Dart, Lua and Swift ports:

1. **Syntactic check** (:func:`is_safe_cow_name`). A cow name is a bare file
   stem, so anything that could mean "some other directory" is refused:

   ======  ==========================================================
   ``/``   path separator
   ``\\``   path separator on Windows
   ``..``  parent-directory step
   ``:``   Windows drive letter (``C:x``) / alternate data stream
   NUL     C APIs stop reading at it, so the OS would see a different
           name from the one we checked
   ``""``  the empty name
   ======  ==========================================================

   An absolute path always contains ``/``, ``\\`` or ``:``, so it falls to the
   same rule.

2. **Containment check** (:func:`resolve_cow_path`). After joining, both the
   cows directory and the candidate are canonicalized with
   :func:`os.path.realpath` (symlinks and ``.``/``..`` resolved) and the
   candidate must still sit inside the cows directory. This catches what
   string inspection cannot see -- e.g. a symlink planted in the cows
   directory that points somewhere else.

A name failing either layer is treated exactly like a cow that does not exist:
we quietly draw ``default.cow``, matching every other port and cowsay's
long-standing "unknown cow -> default cow" behaviour. Nothing is URL-decoded:
``%2F`` is three literal characters to the operating system, never a slash.

This module deliberately uses only the standard library so it can be tested
without installing the CLI's own dependencies.
"""

from __future__ import annotations

import os

# Characters that let a "name" smuggle in a directory, a drive, or a truncation.
_FORBIDDEN_CHARS = frozenset("/\\:\0")


def is_safe_cow_name(cow_name: str) -> bool:
    """Return True when ``cow_name`` is a bare file stem naming no other directory."""
    # A non-string (None, bytes) is refused rather than raising, like the
    # Ruby, Elixir and TypeScript ports.
    if not isinstance(cow_name, str) or not cow_name or ".." in cow_name:
        return False
    return not any(ch in _FORBIDDEN_CHARS for ch in cow_name)


def resolve_cow_path(cow_name: str, cows_dir: str) -> str:
    """Map ``cow_name`` to a ``.cow`` file inside ``cows_dir``.

    Falls back to ``cows_dir/default.cow`` when the name is unsafe, the file is
    missing, or the resolved file escapes ``cows_dir``.
    """
    default_path = os.path.join(cows_dir, "default.cow")
    if not is_safe_cow_name(cow_name):
        return default_path

    root = os.path.realpath(cows_dir)
    resolved = os.path.realpath(os.path.join(cows_dir, f"{cow_name}.cow"))

    # commonpath raises ValueError when the two paths sit on different Windows
    # drives -- which is itself proof that the candidate is outside the root.
    try:
        inside = os.path.commonpath([root, resolved]) == root
    except ValueError:
        inside = False

    if inside and os.path.isfile(resolved):
        return resolved
    return default_path
