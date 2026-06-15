"""Cons pairs — the SIR ``Pair`` value type (``cons`` / ``car`` / ``cdr``).

A *pair* is the Lisp cons cell: an immutable two-field record holding a
``car`` (first) and ``cdr`` (rest).  Linked pairs build lists.  Python has
no native cons cell — a ``tuple`` is close but is variadic and has no list
*display* — so pairs are an SIR quirk that lives here.

A proper list ``(1 2 3)`` is ``cons(1, cons(2, cons(3, nil)))`` where
``nil`` is ``None``.  The display follows Lisp convention: space-separated
inside parens, with a dotted tail when the final ``cdr`` is not ``nil``::

    cons(1, cons(2, None))   ->  "(1 2)"
    cons(1, 2)               ->  "(1 . 2)"     (improper / dotted pair)

**Why a separate package, and why a display *hook*.**  The general SIR value
display lives in :mod:`coding_adventures_sir_runtime_core` (``to_display``).
A pair must render its elements with that richer display so that, say, a
booleans inside a list print as ``#t``/``#f`` rather than Python's
``True``/``False``.  But core *also* needs to know about pairs (to display a
pair nested inside some other value), which would make pairs and core import
each other — a load-time cycle.

We break the cycle by *inverting the dependency*: this package depends on
**nothing** and exposes a module-level display *hook*.  Out of the box the
hook is plain :func:`str`.  When core is present it calls :func:`set_display`
once at import time to inject its ``to_display``, and from then on pairs
render as proper Lisp lists.  Used standalone (no core), a pair still prints
sensibly — just with ``str`` for each element.

::

    pairs ◀───── set_display(to_display) ───── core   (core knows pairs;
      │                                                 pairs never imports
      └─ depends on nothing ────────────────────────── core)
"""

from __future__ import annotations

from collections.abc import Callable
from typing import Any

# The SIR universal value type at this package's boundary.  A pair's ``car``
# and ``cdr`` can each hold any SIR value (a number, a string, ``None`` for
# ``nil``, or another :class:`Pair`).
Val = Any


# ── The injectable display hook ──────────────────────────────────────────────


def _default_display(v: Any) -> str:
    """Fallback element renderer used until/unless core injects its own.

    Defined as a real named function (not a ``lambda``) so the module global
    below is a plain assignment — ruff's E731 forbids ``name = lambda …``.
    """
    return str(v)


# The current element renderer.  :class:`Pair` calls this for every element it
# prints.  Core overwrites it via :func:`set_display`; standalone it stays
# :func:`_default_display`.
_display: Callable[[Any], str] = _default_display


def set_display(fn: Callable[[Any], str]) -> None:
    """Inject the element renderer that :class:`Pair` uses for its display.

    :mod:`coding_adventures_sir_runtime_core` calls this once at import time
    with its richer ``to_display`` so pairs render as proper Lisp lists
    (booleans as ``#t``/``#f``, nested pairs recursively, and so on).  Without
    that injection the renderer falls back to :func:`str`, which is why this
    package can be used on its own with no dependency on core.
    """
    global _display
    _display = fn


# ── The SIR pair value ───────────────────────────────────────────────────────


class Pair:
    """An immutable cons cell with ``car`` and ``cdr`` fields.

    ``__repr__`` renders the Lisp list display, calling the injected
    :data:`_display` hook for each element (so the same pair prints richly
    under core and plainly standalone — see the module docstring).
    """

    __slots__ = ("car", "cdr")

    def __init__(self, car: Val, cdr: Val) -> None:
        self.car = car
        self.cdr = cdr

    def __repr__(self) -> str:
        # Open paren + the first element, then walk the ``cdr`` chain appending
        # each subsequent ``car``.  A non-``None`` final tail is an improper
        # (dotted) pair and prints with the Lisp `` . `` separator.
        parts = ["(", _display(self.car)]
        rest: Val = self.cdr
        while isinstance(rest, Pair):
            parts.append(" ")
            parts.append(_display(rest.car))
            rest = rest.cdr
        if rest is not None:
            parts.append(" . ")
            parts.append(_display(rest))
        parts.append(")")
        return "".join(parts)


def cons(a: Val, b: Val) -> Pair:
    """Construct a pair ``(a . b)``."""
    return Pair(a, b)


def car(p: Val) -> Val:
    """First field of a pair.  Errors on a non-pair (SIR has no silent nil
    coercion here)."""
    if not isinstance(p, Pair):
        raise TypeError("car on non-pair")
    return p.car


def cdr(p: Val) -> Val:
    """Rest field of a pair.  Errors on a non-pair."""
    if not isinstance(p, Pair):
        raise TypeError("cdr on non-pair")
    return p.cdr


def is_pair(v: Val) -> bool:
    """True iff ``v`` is a pair."""
    return isinstance(v, Pair)
