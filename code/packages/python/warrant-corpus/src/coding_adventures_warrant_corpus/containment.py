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
import posixpath
from pathlib import PurePath, PurePosixPath, PureWindowsPath

_DEVICE_PREFIXES = ("\\\\?\\", "\\\\.\\", "//?/", "//./")


def lexical(text: str, flavor: str) -> PurePath | None:
    """``text`` as a normalised pure path of ``flavor``; ``None`` for a device path."""
    if flavor == "posix":
        return PurePosixPath(posixpath.normpath(text))
    if flavor == "win32":
        if text.startswith(_DEVICE_PREFIXES):
            return None
        return PureWindowsPath(ntpath.normpath(text))
    raise ValueError(f"unknown path flavor {flavor!r}")


def relative_inside(
    root: str | PurePath, child: str | PurePath, flavor: str | None = None
) -> str | None:
    """``child`` relative to ``root`` if at or beneath it (POSIX-separated), else None.

    The root itself yields ``""``; callers that need a strict descendant
    reject that too.  With ``flavor`` the arguments are text, normalised
    lexically; without it they are already-resolved pure paths of one flavour.
    """
    if flavor is not None:
        base, target = lexical(str(root), flavor), lexical(str(child), flavor)
        if base is None or target is None:
            return None
    else:
        base, target = PurePath(root), PurePath(child)  # type: ignore[assignment]
    if target != base and base not in target.parents:
        return None
    if target == base:
        return ""
    return target.relative_to(base).as_posix()
