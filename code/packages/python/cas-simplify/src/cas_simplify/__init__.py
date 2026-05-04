"""Canonical form, identity-rule simplification, and Phase 21 suite.

Quick start::

    from cas_simplify import simplify, canonical
    from symbolic_ir import ADD, MUL, IRApply, IRInteger, IRSymbol

    x = IRSymbol("x")
    expr = IRApply(MUL, (IRApply(ADD, (x, IRInteger(0))), IRInteger(1)))
    simplify(expr)
    # IRSymbol("x")

Phase 21 additions::

    from cas_simplify import (
        AssumptionContext,
        radcan, logcontract, logexpand,
        exponentialize, demoivre,
    )

    ctx = AssumptionContext()
    # ... record facts, then pass ctx to radcan/logexpand
"""

from cas_simplify.assumptions import AssumptionContext
from cas_simplify.canonical import canonical
from cas_simplify.exponentialize import demoivre, exponentialize
from cas_simplify.heads import (
    ASSUME,
    CANONICAL,
    DEMOIVRE,
    EXPONENTIALIZE,
    FORGET,
    IS,
    LOGCONTRACT,
    LOGEXPAND,
    RADCAN,
    SIGN,
    SIMPLIFY,
    is_commutative_flat,
)
from cas_simplify.logcontract import logcontract, logexpand
from cas_simplify.numeric_fold import numeric_fold
from cas_simplify.radcan import radcan
from cas_simplify.rules import IDENTITY_RULES
from cas_simplify.simplify import simplify

__all__ = [
    # Core passes (original 0.1.0)
    "CANONICAL",
    "IDENTITY_RULES",
    "SIMPLIFY",
    "canonical",
    "is_commutative_flat",
    "numeric_fold",
    "simplify",
    # Phase 21 — assumption framework heads
    "ASSUME",
    "FORGET",
    "IS",
    "SIGN",
    # Phase 21 — simplification heads
    "RADCAN",
    "LOGCONTRACT",
    "LOGEXPAND",
    "EXPONENTIALIZE",
    "DEMOIVRE",
    # Phase 21 — assumption store
    "AssumptionContext",
    # Phase 21 — transformation functions
    "radcan",
    "logcontract",
    "logexpand",
    "exponentialize",
    "demoivre",
]
