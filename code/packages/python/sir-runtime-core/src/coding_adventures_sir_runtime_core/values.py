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


def to_display(v: Any) -> str:
    """SIR display form of a value.

    Distinct from ``repr``: ``nil`` prints as ``nil``, booleans as
    ``#t`` / ``#f``, a symbol as its bare name, a pair as a Lisp list.
    Everything else falls back to ``str`` (which renders ints/floats/
    sequences/maps natively, and pairs via :meth:`Pair.__repr__`)."""
    if v is None:
        return "nil"
    if isinstance(v, bool):
        return "#t" if v else "#f"
    if isinstance(v, Symbol):
        return v.name
    if isinstance(v, Pair):
        return repr(v)
    return str(v)
