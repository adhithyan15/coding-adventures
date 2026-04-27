"""Equation solving (Phases 1–3: linear, quadratic, cubic, quartic).

Quick start::

    from fractions import Fraction
    from cas_solve import solve_linear, solve_quadratic, solve_cubic, solve_quartic

    solve_linear(Fraction(2), Fraction(3))             # 2x + 3 = 0 → -3/2
    solve_quadratic(Fraction(1), Fraction(-5), Fraction(6))   # → [2, 3]
    solve_cubic(Fraction(1), Fraction(-6), Fraction(11), Fraction(-6))  # → [1, 2, 3]
    solve_quartic(Fraction(1), Fraction(0), Fraction(-5), Fraction(0), Fraction(4))  # → [-2,-1,1,2]
"""

from cas_solve.cubic import solve_cubic
from cas_solve.heads import NSOLVE, ROOTS, SOLVE
from cas_solve.linear import ALL, solve_linear
from cas_solve.quadratic import solve_quadratic
from cas_solve.quartic import solve_quartic

__all__ = [
    "ALL",
    "NSOLVE",
    "ROOTS",
    "SOLVE",
    "solve_linear",
    "solve_quadratic",
    "solve_cubic",
    "solve_quartic",
]
