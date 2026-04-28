"""cas-multivariate — multivariate polynomial operations and Gröbner bases.

This package implements:

* **Gröbner basis computation** (Buchberger's algorithm) over Q[x₁, …, xₙ].
* **Polynomial reduction** (division with remainder in multiple variables).
* **Ideal solving** — find all rational solutions of a polynomial system
  via lex Gröbner basis + back-substitution.

All arithmetic is exact (uses :class:`fractions.Fraction`).  Floating-point
is never used, so results are always mathematically correct.

Quick start::

    from fractions import Fraction
    from cas_multivariate.polynomial import MPoly
    from cas_multivariate.groebner import buchberger
    from cas_multivariate.solve import ideal_solve

    # Represent x + y - 1 and x - y in Q[x, y]
    F = Fraction
    f1 = MPoly({(1,0): F(1), (0,1): F(1), (0,0): F(-1)}, 2)  # x + y - 1
    f2 = MPoly({(1,0): F(1), (0,1): F(-1)}, 2)                # x - y

    # Gröbner basis:
    G = buchberger([f1, f2], order="grlex")

    # Solve: x = 1/2, y = 1/2
    solutions = ideal_solve([f1, f2])
    # → [[Fraction(1, 2), Fraction(1, 2)]]

VM integration::

    from cas_multivariate import build_multivariate_handler_table
    # Returns {"Groebner": ..., "PolyReduce": ..., "IdealSolve": ...}

MACSYMA surface syntax (after wiring in the name table)::

    groebner([x^2+y-1, x+y^2-1], [x, y])
    poly_reduce(x^2, [x-1], [x])
    ideal_solve([x+y-1, x-y], [x, y])
"""

from cas_multivariate.groebner import GrobnerError, buchberger
from cas_multivariate.handlers import (
    GROEBNER,
    IDEAL_SOLVE,
    POLY_REDUCE,
    build_multivariate_handler_table,
    groebner_handler,
    ideal_solve_handler,
    poly_reduce_handler,
)
from cas_multivariate.monomial import (
    cmp_monomials,
    divides,
    lcm_monomial,
    monomial_key,
    total_degree,
)
from cas_multivariate.polynomial import MPoly, make_var
from cas_multivariate.reduce import reduce_poly, s_poly
from cas_multivariate.solve import ideal_solve

__version__ = "0.1.0"

__all__ = [
    # Core types
    "MPoly",
    "make_var",
    # Monomial utilities
    "monomial_key",
    "cmp_monomials",
    "lcm_monomial",
    "divides",
    "total_degree",
    # Algorithms
    "buchberger",
    "reduce_poly",
    "s_poly",
    "ideal_solve",
    # Errors
    "GrobnerError",
    # Handler IR heads
    "GROEBNER",
    "POLY_REDUCE",
    "IDEAL_SOLVE",
    # VM handlers
    "groebner_handler",
    "poly_reduce_handler",
    "ideal_solve_handler",
    "build_multivariate_handler_table",
]
