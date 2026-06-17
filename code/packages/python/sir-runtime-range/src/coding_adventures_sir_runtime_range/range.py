"""Ranges — the SIR ``Range`` value type (``a..b`` / ``a...b``, begin/endless).

A Ruby *range* is a first-class object, not a loop: ``1..5`` is a value you can
iterate, test membership against (``r.include?(3)``), or materialise
(``r.to_a``).  Python's ``range`` is close but is **half-open only** (no
inclusive form), is integer-stride only, and cannot represent the begin/endless
forms Ruby allows — so the SIR ``Range`` is a quirk that lives here as a
per-concern runtime, exactly like cons :mod:`pairs`.

A range carries three fields:

==============  ====================================================
``start``       the low bound, or ``None`` for a *beginless* range ``..b``
``stop``        the high bound, or ``None`` for an *endless* range ``a..``
``exclusive``   ``False`` for ``a..b`` (includes ``b``);
                ``True`` for ``a...b`` (excludes ``b``)
==============  ====================================================

Truth table for membership ``v in r`` (``s`` = start, ``e`` = stop):

============  ==========  =======================================
form          example     ``includes(v)`` is true when…
============  ==========  =======================================
``s..e``      ``1..5``    ``s <= v <= e``
``s...e``     ``1...5``   ``s <= v <  e``
``s..``       ``1..``     ``s <= v``               (endless)
``..e``       ``..5``     ``v <= e``               (beginless)
``...e``      ``...5``    ``v <  e``               (beginless, excl)
============  ==========  =======================================

Iteration walks integers from ``start`` upward.  An *endless* range yields
forever (use it lazily — ``next``/``islice``); a *beginless* range has no first
element, so iterating one (or calling :func:`to_list` on an unbounded range)
raises ``TypeError`` rather than hanging or guessing.  This mirrors Ruby, where
``(..5).each`` is a ``TypeError`` ("can't iterate from NilClass").

This package depends on **nothing** (numeric ranges need no richer display), so
it ships standalone just like :mod:`coding_adventures_sir_runtime_pairs`.

See ``code/specs/sir-runtime.md``.
"""

from __future__ import annotations

from collections.abc import Iterator
from typing import Any

# The SIR universal value type at this package's boundary.  A range's bounds are
# normally integers but the type alias keeps the surface uniform with the other
# SIR runtimes (a bound may be ``None`` for the begin/endless forms).
Val = Any


class Range:
    """An immutable Ruby-style range value.

    Construct via :func:`range_` (the SIR backend emits a call to it).  A
    ``Range`` is iterable, supports ``in`` via :meth:`__contains__`, and renders
    in Ruby's ``a..b`` / ``a...b`` notation.
    """

    __slots__ = ("start", "stop", "exclusive")

    def __init__(self, start: Val, stop: Val, exclusive: Val) -> None:
        self.start = start
        self.stop = stop
        # Coerce to a real bool so ``range_(1, 5, None)`` behaves like ``..``.
        self.exclusive = bool(exclusive)

    def __iter__(self) -> Iterator[Val]:
        # A beginless range has no first element to count up from.
        if self.start is None:
            raise TypeError("cannot iterate a beginless range (no start)")
        value = self.start
        if self.stop is None:
            # Endless range: yield forever.  Callers must consume lazily.
            while True:
                yield value
                value += 1
        elif self.exclusive:
            while value < self.stop:
                yield value
                value += 1
        else:
            while value <= self.stop:
                yield value
                value += 1

    def __contains__(self, value: Val) -> bool:
        return self.includes(value)

    def includes(self, value: Val) -> bool:
        """True iff ``value`` falls within the range (see the module truth table).

        Works for every form — the ``None`` bounds of begin/endless ranges drop
        the corresponding side of the comparison.
        """
        if self.start is not None and value < self.start:
            return False
        if self.stop is not None:
            if self.exclusive:
                if value >= self.stop:
                    return False
            elif value > self.stop:
                return False
        return True

    def to_list(self) -> list[Val]:
        """Materialise the range as a list (Ruby's ``to_a``).

        Raises ``TypeError`` for an unbounded range (beginless **or** endless),
        since neither can produce a finite list — matching Ruby's
        ``RangeError`` / ``TypeError`` for those cases.
        """
        if self.start is None:
            raise TypeError("cannot convert a beginless range to a list")
        if self.stop is None:
            raise TypeError("cannot convert an endless range to a list")
        return list(self)

    def __eq__(self, other: object) -> bool:
        if not isinstance(other, Range):
            return NotImplemented
        return (
            self.start == other.start
            and self.stop == other.stop
            and self.exclusive == other.exclusive
        )

    def __hash__(self) -> int:
        return hash((self.start, self.stop, self.exclusive))

    def __repr__(self) -> str:
        # Ruby notation: ".." inclusive, "..." exclusive; an absent bound (the
        # begin/endless forms) renders as the empty string.
        op = "..." if self.exclusive else ".."
        left = "" if self.start is None else repr(self.start)
        right = "" if self.stop is None else repr(self.stop)
        return f"{left}{op}{right}"


def range_(start: Val, stop: Val, exclusive: Val) -> Range:
    """Construct a :class:`Range` ``start..stop`` (or ``start...stop``).

    This is the entry point the SIR Python backend targets: a Ruby ``a..b``
    lowers to ``BuiltinCall("range", [a, b, false])`` and the emitter renders
    ``_sir_range(a, b, False)``.  Either bound may be ``None`` for the
    begin/endless forms.  The trailing underscore avoids shadowing the builtin
    :func:`range`; the public re-export aliases it as ``range``.
    """
    return Range(start, stop, exclusive)


def includes(r: Range, value: Val) -> bool:
    """Free-function form of :meth:`Range.includes` (Ruby ``r.include?(v)``)."""
    return r.includes(value)


def to_list(r: Range) -> list[Val]:
    """Free-function form of :meth:`Range.to_list` (Ruby ``r.to_a``)."""
    return r.to_list()


def is_range(value: Val) -> bool:
    """True iff ``value`` is a :class:`Range`."""
    return isinstance(value, Range)
