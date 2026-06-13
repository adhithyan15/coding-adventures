"""Cons pairs — the SIR ``Pair`` value type (``cons`` / ``car`` / ``cdr``).

A *pair* is the Lisp cons cell: an immutable two-field record holding a
``car`` (first) and ``cdr`` (rest).  Linked pairs build lists.  Python has
no native cons cell — a ``tuple`` is close but is variadic and has no list
*display* — so pairs are an SIR quirk that lives here.

A proper list ``(1 2 3)`` is ``cons(1, cons(2, cons(3, nil)))`` where
``nil`` is ``None``.  The display follows Lisp convention: space-separated
inside parens, with a dotted tail when the final ``cdr`` is not ``nil``:

    cons(1, cons(2, None))   ->  "(1 2)"
    cons(1, 2)               ->  "(1 . 2)"     (improper / dotted pair)
"""

from __future__ import annotations

from typing import TYPE_CHECKING, Any

if TYPE_CHECKING:  # pragma: no cover - import cycle guard for type-checkers
    pass


class Pair:
    """An immutable cons cell with ``car`` and ``cdr`` fields."""

    __slots__ = ("car", "cdr")

    def __init__(self, car: Any, cdr: Any) -> None:
        self.car = car
        self.cdr = cdr

    def __repr__(self) -> str:
        # Lisp list display.  Deferred import of ``to_display`` avoids a
        # module-load cycle (values -> pairs -> values).
        from .values import to_display

        parts = ["(", to_display(self.car)]
        rest: Any = self.cdr
        while isinstance(rest, Pair):
            parts.append(" ")
            parts.append(to_display(rest.car))
            rest = rest.cdr
        if rest is not None:
            parts.append(" . ")
            parts.append(to_display(rest))
        parts.append(")")
        return "".join(parts)


def cons(a: Any, b: Any) -> Pair:
    """Construct a pair ``(a . b)``."""
    return Pair(a, b)


def car(p: Any) -> Any:
    """First field of a pair.  Errors on a non-pair (SIR has no silent nil
    coercion here)."""
    if not isinstance(p, Pair):
        raise TypeError("car on non-pair")
    return p.car


def cdr(p: Any) -> Any:
    """Rest field of a pair.  Errors on a non-pair."""
    if not isinstance(p, Pair):
        raise TypeError("cdr on non-pair")
    return p.cdr


def is_pair(v: Any) -> bool:
    """True iff ``v`` is a pair."""
    return isinstance(v, Pair)
