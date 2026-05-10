"""Limit and Taylor series for the symbolic IR.

Phase 1 (0.1.0)
---------------
Direct-substitution limit and polynomial Taylor expansion.

Phase 20 (0.2.0)
----------------
Full limit evaluation with L'Hôpital's rule, limits at ±∞, all standard
indeterminate forms, and one-sided limit direction support.

Quick start::

    from cas_limit_series import limit_direct, limit_advanced, taylor_polynomial
    from symbolic_ir import IRApply, IRInteger, IRSymbol, ADD, DIV, POW, SIN

    x = IRSymbol("x")
    expr = IRApply(ADD, (IRApply(POW, (x, IRInteger(2))), IRInteger(1)))

    limit_direct(expr, x, IRInteger(2))      # 5  (un-simplified Add(4, 1))
    taylor_polynomial(expr, x, IRInteger(0), 2)
    # Add(1, x^2)   — the Taylor expansion at 0 of x^2 + 1 to order 2

    # L'Hôpital via injected diff_fn:
    sin_over_x = IRApply(DIV, (IRApply(SIN, (x,)), x))
    limit_advanced(sin_over_x, x, IRInteger(0), diff_fn=my_diff, eval_fn=my_eval)
    # → IRInteger(1)
"""

from cas_limit_series.heads import BIG_O, INF, LIMIT, MINF, SERIES, TAYLOR
from cas_limit_series.limit import limit_direct
from cas_limit_series.limit_advanced import limit_advanced
from cas_limit_series.taylor import PolynomialError, taylor_polynomial

__all__ = [
    "BIG_O",
    "INF",
    "LIMIT",
    "MINF",
    "PolynomialError",
    "SERIES",
    "TAYLOR",
    "limit_advanced",
    "limit_direct",
    "taylor_polynomial",
]
