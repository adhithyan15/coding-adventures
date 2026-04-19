"""Numeric helpers used by arithmetic handlers.

We pass arithmetic through Python's :class:`fractions.Fraction` whenever
both operands are exact (``IRInteger`` or ``IRRational``). As soon as
one operand is an :class:`~symbolic_ir.IRFloat`, the whole expression
collapses to a ``float`` — the same contamination rule SymPy and
Mathematica use to mark a result as inexact.

The two functions here — :func:`to_number` and :func:`from_number` —
are the only place the VM converts between IR literals and Python
numbers, so the "exact vs. inexact" choice lives in one spot.
"""

from __future__ import annotations

from fractions import Fraction

from symbolic_ir import IRFloat, IRInteger, IRNode, IRRational

# A "numeric" is anything ``to_number`` might return. It's purposefully
# narrow so callers know exactly what ``va + vb`` can mean.
Numeric = Fraction | float


def to_number(node: IRNode) -> Numeric | None:
    """Return the Python numeric value of an IR literal, or ``None``.

    Non-numeric IR nodes (symbols, applies, strings) return ``None``;
    callers treat that as "this node is symbolic, don't try to fold".
    """
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    if isinstance(node, IRFloat):
        return node.value
    return None


def from_number(value: Numeric) -> IRNode:
    """Lift a Python number back to its most specific IR literal.

    - ``Fraction`` with denominator 1 → :class:`IRInteger`.
    - Other ``Fraction`` → :class:`IRRational`.
    - ``float`` → :class:`IRFloat`.
    """
    if isinstance(value, Fraction):
        if value.denominator == 1:
            return IRInteger(value.numerator)
        return IRRational(value.numerator, value.denominator)
    if isinstance(value, float):
        return IRFloat(value)
    # Plain ``int`` — happens when a caller returned an int literal
    # instead of going through Fraction. We handle it defensively.
    if isinstance(value, int):
        return IRInteger(value)
    raise TypeError(f"not a numeric value: {value!r}")


def is_zero(value: Numeric | None) -> bool:
    """True if ``value`` is numerically zero. ``None`` is not zero."""
    return value is not None and value == 0


def is_one(value: Numeric | None) -> bool:
    """True if ``value`` is numerically one."""
    return value is not None and value == 1
