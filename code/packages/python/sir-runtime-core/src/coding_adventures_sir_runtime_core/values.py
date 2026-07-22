"""Value-level SIR semantics: truthiness, equality, display, predicates.

These are the behaviours that differ from Python's native ones and so
cannot be emitted as bare native code.

**SIR truthiness is false/nil-only.**  Only ``False`` and ``nil`` (``None``)
are falsy.  Everything else — including ``0``, ``0.0``, ``""``, ``[]``,
``{}``, a symbol, a pair — is **truthy**.  This is the Lisp/Ruby
convention and is the single most important reason this library exists:
Python's native ``bool()`` would (wrongly, for SIR) call ``0``/``""``/``[]``
falsy.

    truthy(False) -> False     truthy(None) -> False
    truthy(0)     -> True       truthy("")   -> True
    truthy([])    -> True       truthy(0.0)  -> True
"""

from __future__ import annotations

from typing import Any

from .pairs import Pair
from .symbols import Symbol


def truthy(v: Any) -> bool:
    """SIR truthiness: everything is true except ``False`` and ``nil``."""
    return v is not False and v is not None


def is_null(v: Any) -> bool:
    """True iff ``v`` is ``nil`` (``None``)."""
    return v is None


def is_number(v: Any) -> bool:
    """True iff ``v`` is an integer (``bool`` is excluded — in Python
    ``bool`` is an ``int`` subclass, but SIR keeps them distinct)."""
    return isinstance(v, int) and not isinstance(v, bool)


def is_symbol(v: Any) -> bool:
    """True iff ``v`` is a :class:`Symbol`."""
    return isinstance(v, Symbol)


def eq(a: Any, b: Any) -> bool:
    """SIR equality.  Symbol-aware (two symbols are equal iff their names
    match); otherwise native ``==``."""
    if isinstance(a, Symbol) and isinstance(b, Symbol):
        return a.name == b.name
    return bool(a == b)


def ne(a: Any, b: Any) -> bool:
    """SIR inequality — the exact negation of :func:`eq`, so it inherits the
    same symbol-awareness (`:x != :x` is false).  The Ruby frontend lowers
    `a != b` to a `!=` builtin; defining it as `not eq(...)` keeps `==` and
    `!=` from ever disagreeing (matching the C backend's `_sir_ne`)."""
    return not eq(a, b)


# ── source-language display convention (SIR display-convention spec) ──
#
# The default convention is ``"lisp"`` (Twig/Scheme: booleans as ``#t`` / ``#f``),
# matching this library's original behaviour.  A Ruby-sourced emitted program
# calls :func:`set_display_convention` with ``"ruby"`` once at startup so
# ``puts true`` prints ``true``.  Module-level state (each emitted program is its
# own process) keeps ``to_display`` and every call site convention-aware without
# threading a parameter through the whole display path.
_DISPLAY_CONVENTION = "lisp"


def set_display_convention(name: str) -> None:
    """Select the value-display convention: ``"ruby"`` or ``"lisp"`` (default).

    An unrecognised name falls back to the ``"lisp"`` default rather than
    raising, so a forward-compatible emitter can never crash an older runtime."""
    global _DISPLAY_CONVENTION
    _DISPLAY_CONVENTION = "ruby" if name == "ruby" else "lisp"


def to_display(v: Any) -> str:
    """SIR display form of a value.

    Distinct from ``repr``: ``nil`` prints as ``nil``, a symbol as its bare
    name, a pair as a Lisp list.  Booleans follow the active display convention
    (see :func:`set_display_convention`): ``true`` / ``false`` under ``"ruby"``,
    else the default Lisp ``#t`` / ``#f``.  Everything else falls back to
    ``str`` (which renders ints/floats/sequences/maps natively, and pairs via
    :meth:`Pair.__repr__`)."""
    if v is None:
        return "nil"
    if isinstance(v, bool):
        if _DISPLAY_CONVENTION == "ruby":
            return "true" if v else "false"
        return "#t" if v else "#f"
    if isinstance(v, Symbol):
        return v.name
    if isinstance(v, Pair):
        return repr(v)
    return str(v)
