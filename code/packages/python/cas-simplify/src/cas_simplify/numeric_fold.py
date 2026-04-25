"""Constant-folding inside ``Add`` and ``Mul`` arg lists.

When canonicalization brings every numeric literal to the front of an
argument list, we can collapse them into a single literal::

    Add(2, 3, x)  →  Add(5, x)
    Mul(2, 3, x)  →  Mul(6, x)

Mixed integer + rational arithmetic stays exact (uses
:class:`fractions.Fraction` internally and re-promotes to ``IRInteger``
when the result is whole). Float contamination collapses everything to
``IRFloat``.

Nothing is consumed across heads — ``Add(2, Mul(3, 4))`` does NOT
fold the inner ``Mul`` because ``simplify`` runs canonical → fold
in a fixed-point loop, so the inner ``Mul`` will fold first, leaving
``Add(2, 12)`` for the next pass.
"""

from __future__ import annotations

from collections.abc import Callable
from fractions import Fraction

from symbolic_ir import (
    IRApply,
    IRFloat,
    IRInteger,
    IRNode,
    IRRational,
    IRSymbol,
)

# A "number" we accumulate while folding — either an exact ``Fraction``
# or a contaminated ``float``. We never mix the two: a single float in
# the args promotes the accumulator to float for the rest of the pass.
_Number = Fraction | float


# ---------------------------------------------------------------------------
# Public entry
# ---------------------------------------------------------------------------


def numeric_fold(node: IRNode) -> IRNode:
    """Recursively fold numeric literals inside ``Add`` and ``Mul`` args."""
    if isinstance(node, IRApply):
        new_head = numeric_fold(node.head)
        new_args = tuple(numeric_fold(a) for a in node.args)
        if isinstance(new_head, IRSymbol):
            if new_head.name == "Add":
                new_args = _fold(new_args, identity=Fraction(0), op=_add)
            elif new_head.name == "Mul":
                new_args = _fold(new_args, identity=Fraction(1), op=_mul)
        # Singleton drop: Add(x) → x, Mul(x) → x.
        if (
            isinstance(new_head, IRSymbol)
            and new_head.name in {"Add", "Mul"}
            and len(new_args) == 1
        ):
            return new_args[0]
        if new_head is node.head and new_args == node.args:
            return node
        return IRApply(new_head, new_args)
    return node


# ---------------------------------------------------------------------------
# Internals
# ---------------------------------------------------------------------------


def _add(a: _Number, b: _Number) -> _Number:
    return a + b


def _mul(a: _Number, b: _Number) -> _Number:
    return a * b


def _fold(
    args: tuple[IRNode, ...],
    *,
    identity: Fraction,
    op: Callable[[_Number, _Number], _Number],
) -> tuple[IRNode, ...]:
    """Combine numeric literals in ``args`` via ``op``; return new arg tuple.

    Strategy:

    1. Walk args once; for each literal, accumulate into ``acc`` (with
       float contamination promoted as it appears).
    2. Non-literal args go into ``other`` in their original order.
    3. If no literal was seen, return ``args`` unchanged.
    4. If the accumulator equals the identity AND there are non-literal
       args, drop the literal.
    5. Otherwise prepend the folded literal to the non-literal args.
    """
    acc: _Number = identity
    has_float = False
    saw_literal = False
    other: list[IRNode] = []

    for arg in args:
        value = _to_number(arg)
        if value is None:
            other.append(arg)
            continue
        saw_literal = True
        if isinstance(value, float):
            has_float = True
        acc = op(float(acc), float(value)) if has_float else op(acc, value)

    if not saw_literal:
        return args

    folded = _from_number(acc)

    # If the folded literal is the identity AND there are other args,
    # drop the literal — it's redundant.
    if folded == _identity_node(identity) and other:
        return tuple(other)

    return (folded, *other)


def _to_number(node: IRNode) -> _Number | None:
    """Return the numeric value of ``node`` or None if it isn't a literal."""
    if isinstance(node, IRInteger):
        return Fraction(node.value)
    if isinstance(node, IRRational):
        return Fraction(node.numer, node.denom)
    if isinstance(node, IRFloat):
        return node.value
    return None


def _from_number(value: _Number) -> IRNode:
    """Wrap a folded number back into the appropriate IR literal type."""
    if isinstance(value, float):
        return IRFloat(value)
    if value.denominator == 1:
        return IRInteger(value.numerator)
    return IRRational(value.numerator, value.denominator)


def _identity_node(identity: Fraction) -> IRNode:
    return IRInteger(0 if identity == Fraction(0) else 1)
