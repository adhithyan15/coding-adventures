"""Univariate polynomial factoring over Z (Phase 1).

Quick start::

    from cas_factor import factor_integer_polynomial

    factor_integer_polynomial([-1, 0, 1])
    # (1, [([-1, 1], 1), ([1, 1], 1)])
    # 1 * (x - 1) * (x + 1)
"""

from cas_factor.factor import FactorList, factor_integer_polynomial
from cas_factor.heads import FACTOR, IRREDUCIBLE
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
    "content",
    "degree",
    "divide_linear",
    "divisors",
    "evaluate",
    "extract_linear_factors",
    "factor_integer_polynomial",
    "find_integer_roots",
    "normalize",
    "primitive_part",
]
