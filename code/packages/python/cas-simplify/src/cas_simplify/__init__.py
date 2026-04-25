"""Canonical form and identity-rule simplification for the symbolic IR.

Quick start::

    from cas_simplify import simplify, canonical
    from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRSymbol

    x = IRSymbol("x")
    expr = IRApply(MUL, (IRApply(ADD, (x, IRInteger(0))), IRInteger(1)))
    simplify(expr)
    # IRSymbol("x")
"""

from cas_simplify.canonical import canonical
from cas_simplify.heads import CANONICAL, SIMPLIFY, is_commutative_flat
from cas_simplify.numeric_fold import numeric_fold
from cas_simplify.rules import IDENTITY_RULES
from cas_simplify.simplify import simplify

__all__ = [
    "CANONICAL",
    "IDENTITY_RULES",
    "SIMPLIFY",
    "canonical",
    "is_commutative_flat",
    "numeric_fold",
    "simplify",
]
