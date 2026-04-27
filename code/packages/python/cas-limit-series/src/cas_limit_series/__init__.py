"""Limit and Taylor series for the symbolic IR (Phase 1 foundation).

Quick start::

    from cas_limit_series import limit_direct, taylor_polynomial
    from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, POW

    x = IRSymbol("x")
    expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))

    limit_direct(expr, x, IRInteger(2))      # 5  (un-simplified Add(4, 1))
    taylor_polynomial(expr, x, IRInteger(0), 2)
    # Add(1, x^2)   — the Taylor expansion at 0 of x^2 + 1 to order 2
"""

from cas_limit_series.heads import BIG_O, LIMIT, SERIES, TAYLOR
from cas_limit_series.limit import limit_direct
from cas_limit_series.taylor import PolynomialError, taylor_polynomial

__all__ = [
    "BIG_O",
    "LIMIT",
    "PolynomialError",
    "SERIES",
    "TAYLOR",
    "limit_direct",
    "taylor_polynomial",
]
