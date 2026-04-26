"""Equation solving (Phase 1: linear + quadratic).

Quick start::

    from fractions import Fraction
    from cas_solve import solve_linear, solve_quadratic

    solve_linear(Fraction(2), Fraction(3))            # 2x + 3 = 0 → -3/2
    solve_quadratic(Fraction(1), Fraction(-5), Fraction(6))  # → [2, 3]
    solve_quadratic(Fraction(1), Fraction(0), Fraction(1))   # → [%i, -%i]
"""

from cas_solve.heads import NSOLVE, ROOTS, SOLVE
from cas_solve.linear import ALL, solve_linear
from cas_solve.quadratic import solve_quadratic

__all__ = [
    "ALL",
    "NSOLVE",
    "ROOTS",
    "SOLVE",
    "solve_linear",
    "solve_quadratic",
]
