"""Identity rule database used by :func:`cas_simplify.simplify`.

Every rule is built from :mod:`cas_pattern_matching` primitives, so the
matcher does the heavy lifting and the rules read like math::

    Add(x_, 0) -> x

This module is intentionally a *table*, not a function tree — adding
a new identity is one entry, no boilerplate.
"""

from __future__ import annotations

from cas_pattern_matching import Blank, Pattern, Rule
from symbolic_ir import (
    ADD,
    COS,
    DIV,
    EXP,
    LOG,
    MUL,
    POW,
    SIN,
    SUB,
    IRApply,
    IRInteger,
    IRSymbol,
)

# Pattern variables shared across rules.
X = Pattern("x", Blank())


def _build_rules() -> list[IRApply]:
    """Construct the immutable list of identity rules.

    Each entry is an ``IRApply(Rule, (lhs, rhs))`` consumed by
    ``cas_pattern_matching.rewrite``.
    """
    zero = IRInteger(0)
    one = IRInteger(1)

    return [
        # Add identities.
        Rule(IRApply(ADD, (X, zero)), X),
        Rule(IRApply(ADD, (zero, X)), X),
        # Mul identities.
        Rule(IRApply(MUL, (X, one)), X),
        Rule(IRApply(MUL, (one, X)), X),
        Rule(IRApply(MUL, (X, zero)), zero),
        Rule(IRApply(MUL, (zero, X)), zero),
        # Pow identities.
        Rule(IRApply(POW, (X, zero)), one),
        Rule(IRApply(POW, (X, one)), X),
        Rule(IRApply(POW, (one, X)), one),
        # Sub / Div identities.
        Rule(IRApply(SUB, (X, X)), zero),
        Rule(IRApply(DIV, (X, X)), one),
        # Inverse-function identities.
        Rule(IRApply(LOG, (IRApply(EXP, (X,)),)), X),
        Rule(IRApply(EXP, (IRApply(LOG, (X,)),)), X),
        # Trig at zero.
        Rule(IRApply(SIN, (zero,)), zero),
        Rule(IRApply(COS, (zero,)), one),
    ]


# Use a frozen list so callers can't accidentally mutate it.
IDENTITY_RULES: list[IRApply] = _build_rules()


__all__ = ["IDENTITY_RULES"]


# Reference avoids unused-import warnings for IRSymbol (some IRs alias
# the heads through it; keep the import explicit for clarity).
_ = IRSymbol
