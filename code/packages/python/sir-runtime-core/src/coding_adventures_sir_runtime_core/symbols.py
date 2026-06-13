"""Interned symbols — the SIR ``Symbol`` value type.

A Lisp/Ruby *symbol* (``:foo``) is an interned, immutable name with
**identity** semantics: two symbols with the same text are the *same*
object, so equality is a pointer compare.  Python has no native symbol
type — the closest native thing, a ``str``, has *value* semantics and no
distinct identity — so symbols are a genuine SIR quirk that lives here in
the runtime library rather than being faked inline in emitted code.

Interning is process-global and single-threaded (CPython's GIL makes the
table safe in-process), mirroring how the original inlined runtime worked.

Truth table — what counts as "the same symbol":

    intern("a") is intern("a")   -> True   (same text -> same object)
    intern("a") is intern("b")   -> False  (different text)
    intern("a") == intern("a")   -> True
    Symbol("a") == Symbol("a")   -> True   (value equality still holds)
"""

from __future__ import annotations


class Symbol:
    """An interned symbol.  Construct via :func:`intern`, not directly,
    so identity (``is``) comparisons hold across the program."""

    __slots__ = ("name",)

    def __init__(self, name: str) -> None:
        self.name = name

    def __eq__(self, other: object) -> bool:
        return isinstance(other, Symbol) and self.name == other.name

    def __hash__(self) -> int:
        # Namespaced so a Symbol never collides with the bare string in a
        # dict that mixes the two.
        return hash(("__SIR_SYM__", self.name))

    def __repr__(self) -> str:
        return self.name


_symbol_table: dict[str, Symbol] = {}


def intern(name: str) -> Symbol:
    """Return the canonical :class:`Symbol` for ``name``, creating it on
    first sight.  Repeated calls with the same text return the *same*
    object."""
    existing = _symbol_table.get(name)
    if existing is None:
        existing = Symbol(name)
        _symbol_table[name] = existing
    return existing
