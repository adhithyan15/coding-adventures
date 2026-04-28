"""Univariate polynomial factoring over Z (Phase 1–3: rational-root + Kronecker + BZH).

Quick start::

    from cas_factor import factor_integer_polynomial

    factor_integer_polynomial([-1, 0, 1])
    # (1, [([-1, 1], 1), ([1, 1], 1)])
    # 1 * (x - 1) * (x + 1)

    factor_integer_polynomial([4, 0, 0, 0, 1])
    # (1, [([2, 2, 1], 1), ([2, -2, 1], 1)])
    # 1 * (x^2 + 2x + 2) * (x^2 - 2x + 2)

    factor_integer_polynomial([-1, 0, 0, 0, 0, 1])
    # (1, [([-1, 1], 1), ([1, 1, 1, 1, 1], 1)])
    # 1 * (x - 1) * (x^4 + x^3 + x^2 + x + 1)   [BZH handles the degree-4 cyclotomic]

    factor_integer_polynomial([2, 0, 0, 0, 2])
    # (2, [([1, 0, 0, 0, 1], 1)])
    # 2 * (x^4 + 1)  — irreducible over Q  [content + BZH confirms irreducible]
"""

from cas_factor.bzh import bzh_factor
from cas_factor.factor import FactorList, factor_integer_polynomial
from cas_factor.heads import FACTOR, IRREDUCIBLE
from cas_factor.kronecker import kronecker_factor
from cas_factor.polynomial import (
    Poly,
    content,
    degree,
    divide_linear,
    divisors,
    evaluate,
    normalize,
    primitive_part,
)
from cas_factor.rational_roots import extract_linear_factors, find_integer_roots

__all__ = [
    "FACTOR",
    "FactorList",
    "IRREDUCIBLE",
    "Poly",
    "bzh_factor",
    "content",
    "degree",
    "divide_linear",
    "divisors",
    "evaluate",
    "extract_linear_factors",
    "factor_integer_polynomial",
    "find_integer_roots",
    "kronecker_factor",
    "normalize",
    "primitive_part",
]
