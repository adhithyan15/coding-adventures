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

Two more ``win32`` rules (WD-100b):

* **ASCII-only case folding.**  ``PureWindowsPath`` equality folds with the full
  Unicode ``lower``, which says the Kelvin sign equals ``k`` and that
  ``\u0130`` is ``i`` plus a combining dot.  Containment fails *closed*, so
  names are compared after mapping ``A``..``Z`` to ``a``..``z`` and nothing
  else (:func:`ascii_fold`).
* **Reserved names.**  A child with a Windows device name (``CON``, ``nul.txt``,
  ``COM1``..) or a component ending in a dot or space is never inside.
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


_ASCII_FOLD = {code: code + 32 for code in range(ord("A"), ord("Z") + 1)}

#: The Windows device names, upper-case (VER04i 9.1, WD-100b).
RESERVED_NAMES = frozenset(
    {"CON", "PRN", "AUX", "NUL"}
    | {f"COM{n}" for n in range(1, 10)}
    | {f"LPT{n}" for n in range(1, 10)}
)


def ascii_fold(text: str) -> str:
    """``text`` with ``A``..``Z`` mapped to ``a``..``z`` and nothing else."""
    return text.translate(_ASCII_FOLD)


def refused_component(name: str) -> bool:
    """Is this a reserved device name, or a component ending in a dot or space?

    ``CON``, ``con``, ``CON.txt``, ``com1.tar.gz``, ``NUL.`` and ``nul `` are
    refused; ``console.txt``, ``COM10``, ``LPT0`` and ``CONx`` are ordinary.
    The part before the first dot is compared with the device names, ASCII
    case-insensitively (a name that ends in a dot or space is refused first,
    because Windows strips them and two spellings would name one file).
    """
    if name.endswith((".", " ")):
        return True
    stem = ascii_fold(name.split(".", 1)[0])
    return stem.upper() in RESERVED_NAMES and stem.isascii()


#: A path reduced to ``(anchor, components)``, original case kept.
Lexical = tuple[str, tuple[str, ...]]


def lexical(text: str, flavor: str) -> Lexical | None:
    """``text`` as a normalised ``(anchor, components)``; ``None`` if never inside.

    ``None`` means a device-namespace path (``\\\\?\\``, ``\\\\.\\``, including
    ``\\\\?\\UNC\\``) or a POSIX path with exactly two leading slashes (POSIX
    leaves it implementation-defined, so it is refused).  Windows separators are
    normalised **first**: ``\\/?\\D:`` and ``//?/D:`` are device paths too.
    Case is *not* folded here; :func:`relative_inside` folds ``win32`` names
    with :func:`ascii_fold` when it compares them.
    """
    if flavor == "posix":
        if text.startswith("//") and not text.startswith("///"):
            return None
        path = PurePosixPath(posixpath.normpath(text))
        return path.anchor, path.parts[1:] if path.anchor else path.parts
    if flavor == "win32":
        text = text.replace("/", "\\")
        if text.startswith(_DEVICE_PREFIXES):
            return None
        path = PureWindowsPath(ntpath.normpath(text))
        return path.anchor, path.parts[1:] if path.anchor else path.parts
    raise ValueError(f"unknown path flavor {flavor!r}")


def relative_inside(
    root: str | PurePath, child: str | PurePath, flavor: str | None = None
) -> str | None:
    """``child`` relative to ``root`` if at or beneath it (POSIX-separated), else None.

    The root itself yields ``""``; callers that need a strict descendant
    reject that too.  Both arguments are normalised lexically with ``flavor``,
    which defaults to the host's (:data:`HOST_FLAVOR`): there is no variant that
    skips the device rule, because production must not call one.  Under
    ``win32`` the comparison folds ASCII only and a child with a reserved
    device name or a trailing dot/space component is outside (WD-100b).
    """
    flavor = flavor or HOST_FLAVOR
    base = lexical(str(root), flavor)
    target = lexical(str(child), flavor)
    if base is None or target is None:
        return None
    fold = ascii_fold if flavor == "win32" else str
    if fold(base[0]) != fold(target[0]) or len(target[1]) < len(base[1]):
        return None
    if any(fold(a) != fold(b) for a, b in zip(base[1], target[1], strict=False)):
        return None
    if flavor == "win32" and any(refused_component(part) for part in target[1]):
        return None
    return "/".join(target[1][len(base[1]) :])
