"""SIR arithmetic and comparison.

These differ from Python's native operators in two ways that justify a
runtime helper rather than a bare ``a + b``:

1. **Variadic** — ``add`` / ``sub`` / ``mul`` fold over any number of
   arguments (``add()`` is ``0``, ``mul()`` is ``1``), matching the SIR
   builtin contract shared with the Lisp/Twig frontends.
2. **Truncating integer division** — ``div`` truncates toward zero
   (``div(7, 2) == 3``, ``div(-7, 2) == -3``) to match SIR semantics,
   where Python's ``/`` would yield a float.

Comparisons (``lt`` / ``gt``) are thin wrappers kept here so the dispatch
table can expose them by SIR name.
"""

from __future__ import annotations

from typing import Any


def add(*args: Any) -> Any:
    """Variadic sum; ``add()`` is ``0``."""
    total: Any = 0
    for a in args:
        total += a
    return total


def sub(*args: Any) -> Any:
    """Variadic difference; ``sub(x)`` negates, ``sub()`` is ``0``."""
    if not args:
        return 0
    if len(args) == 1:
        return -args[0]
    acc = args[0]
    for a in args[1:]:
        acc -= a
    return acc


def mul(*args: Any) -> Any:
    """Variadic product; ``mul()`` is ``1``."""
    acc: Any = 1
    for a in args:
        acc *= a
    return acc


def div(*args: Any) -> Any:
    """Variadic quotient with **truncating integer** division (toward
    zero), matching SIR semantics rather than Python's float ``/``."""
    if not args:
        return 0
    acc = args[0]
    for a in args[1:]:
        acc = int(acc / a)
    return acc


def lt(a: Any, b: Any) -> bool:
    """Less-than."""
    return bool(a < b)


def gt(a: Any, b: Any) -> bool:
    """Greater-than."""
    return bool(a > b)
