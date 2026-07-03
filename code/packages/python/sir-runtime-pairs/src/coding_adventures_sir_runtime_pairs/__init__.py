"""coding-adventures-sir-runtime-pairs — the SIR cons-pair value type.

A *pair* is the Lisp cons cell: an immutable ``(car . cdr)`` record.  Linked
pairs build lists, which Python has no native equivalent for, so they ship
here as a per-concern SIR runtime::

    from coding_adventures_sir_runtime_pairs import cons, car, cdr
    p = cons(1, cons(2, cons(3, None)))   # the proper list (1 2 3)
    car(p)                                # 1
    repr(p)                               # "(1 2 3)"

**Extraction + injection design.**  The general SIR value display lives in
:mod:`coding_adventures_sir_runtime_core`.  A pair wants to render its
elements with that richer display, but core also needs to display pairs —
importing each other would form a load-time cycle.  We break it by inverting
the dependency: **this package depends on nothing** and exposes a module-level
display *hook* (default :func:`str`).  Core calls :func:`set_display` once at
import to inject its ``to_display``; standalone, pairs still print with
``str``.  Pairs never import core.

See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from .pairs import Pair, Val, car, cdr, cons, is_pair, set_display

__all__ = [
    "Pair",
    "Val",
    "car",
    "cdr",
    "cons",
    "is_pair",
    "set_display",
]
