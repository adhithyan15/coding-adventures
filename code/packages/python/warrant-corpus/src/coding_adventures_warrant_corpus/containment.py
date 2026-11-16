"""Is this path inside that directory?  (VER04i 9.1, containment profile)

A containment check is a security boundary: a crafted ``--corpus`` or binding
``import`` must not point the runner at files outside the repository.  The
check here is **lexical** and takes the path flavour as a parameter, so the
Windows rules can be exercised from any host (the shared conformance fixture
does exactly that).

Four traps the profile names, and how each is avoided:

* **Another drive or share.**  ``D:\\repo`` and ``C:\\tmp\\x`` have no common
  root.  Comparing *parsed* paths (drive, then components) says "outside"
  directly; there is no relative-path string to misread.  (The JavaScript
  ``path.relative`` returns an *absolute* path across drives, which a naive
  ``startsWith("..")`` check waves through.)
* **``..foo`` is a name.**  Components are compared whole, never as strings.
* **A string prefix is not a path prefix.**  ``/rr`` is not inside ``/r``.
* **``..`` at the root stays at the root.**  ``/../r/x`` is ``/r/x``.

Device-namespace paths (``\\\\?\\``, ``\\\\.\\``) are never inside anything.
Symlinks are not resolved here; callers resolve first (``Path.resolve``).
"""

from __future__ import annotations

import ntpath
import os
import posixpath
from pathlib import PurePath, PurePosixPath, PureWindowsPath

#: The path rules production uses.  Read at call time (not import time) so a
#: test can prove the production call path applies the same rules the shared
#: fixture drives.
HOST_FLAVOR = "win32" if os.name == "nt" else "posix"

_DEVICE_PREFIXES = ("\\\\?\\", "\\\\.\\")


def lexical(text: str, flavor: str) -> PurePath | None:
    """``text`` as a normalised pure path of ``flavor``; ``None`` if never inside.

    ``None`` means a device-namespace path (``\\\\?\\``, ``\\\\.\\``, including
    ``\\\\?\\UNC\\``) or a POSIX path with exactly two leading slashes (POSIX
    leaves it implementation-defined, so it is refused).  Windows separators are
    normalised **first**: ``\\/?\\D:`` and ``//?/D:`` are device paths too.
    """
    if flavor == "posix":
        if text.startswith("//") and not text.startswith("///"):
            return None
        return PurePosixPath(posixpath.normpath(text))
    if flavor == "win32":
        text = text.replace("/", "\\")
        if text.startswith(_DEVICE_PREFIXES):
            return None
        return PureWindowsPath(ntpath.normpath(text))
    raise ValueError(f"unknown path flavor {flavor!r}")


def relative_inside(
    root: str | PurePath, child: str | PurePath, flavor: str | None = None
) -> str | None:
    """``child`` relative to ``root`` if at or beneath it (POSIX-separated), else None.

    The root itself yields ``""``; callers that need a strict descendant
    reject that too.  Both arguments are normalised lexically with ``flavor``,
    which defaults to the host's (:data:`HOST_FLAVOR`): there is no variant that
    skips the device rule, because production must not call one.
    """
    base = lexical(str(root), flavor or HOST_FLAVOR)
    target = lexical(str(child), flavor or HOST_FLAVOR)
    if base is None or target is None:
        return None
    if target != base and base not in target.parents:
        return None
    if target == base:
        return ""
    return target.relative_to(base).as_posix()
