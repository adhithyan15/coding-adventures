"""Equation solving (Phases 1–6: linear, quadratic, cubic, quartic, numeric, systems,
Phase 26 transcendental families, and Phase 27 polynomial inequality solving).

Quick start::

    from fractions import Fraction
    from cas_solve import solve_linear, solve_quadratic, solve_cubic, solve_quartic
    from cas_solve import solve_linear_system, nsolve_fraction_poly

    solve_linear(Fraction(2), Fraction(3))             # 2x + 3 = 0 → -3/2
    solve_quadratic(Fraction(1), Fraction(-5), Fraction(6))   # → [2, 3]
    solve_cubic(Fraction(1), Fraction(-6), Fraction(11), Fraction(-6))  # → [1, 2, 3]
    solve_quartic(Fraction(1), Fraction(0), Fraction(-5), Fraction(0), Fraction(4))  # → [-2,-1,1,2]

Transcendental solving (Phase 26)::

    from symbolic_ir import IRSymbol, IRApply, IRRational, EQUAL
    from cas_solve import try_solve_transcendental

    x = IRSymbol("x")
    # sin(x) = 1/2 → [arcsin(1/2) + 2k*pi, pi - arcsin(1/2) + 2k*pi]
    eq = IRApply(EQUAL, (IRApply(IRSymbol("Sin"), (x,)), IRRational(1, 2)))
    solutions = try_solve_transcendental(eq, x)
"""

from cas_solve.cubic import solve_cubic
from cas_solve.durand_kerner import nsolve_fraction_poly, nsolve_poly, roots_to_ir
from cas_solve.heads import NSOLVE, ROOTS, SOLVE
from cas_solve.linear import ALL, solve_linear
from cas_solve.linear_system import solve_linear_system
from cas_solve.quadratic import solve_quadratic
from cas_solve.quartic import solve_quartic
from cas_solve.inequality import try_solve_inequality
from cas_solve.transcendental import try_solve_transcendental

__all__ = [
    "ALL",
    "NSOLVE",
    "ROOTS",
    "SOLVE",
    "nsolve_fraction_poly",
    "nsolve_poly",
    "roots_to_ir",
    "solve_cubic",
    "solve_linear",
    "solve_linear_system",
    "solve_quadratic",
    "solve_quartic",
    "try_solve_inequality",
    "try_solve_transcendental",
]
