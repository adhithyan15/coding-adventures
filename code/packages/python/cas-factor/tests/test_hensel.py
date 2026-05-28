"""Tests for bivariate Hensel lifting in ``cas_factor.hensel``.

The acceptance test (Track D1 of the MACSYMA finish plan) is
``factor(x²+xy-2y²) → (x+2y)(x-y)``.  The other five cases pin the
algorithm's behaviour on standard shapes plus the must-fall-through
unfactorable and univariate boundary cases.

Test strategy: verify the algorithm returns the right number of
factors and that their product reconstructs the input.  We don't fix a
canonical ordering — Hensel may legitimately return ``[g, h]`` or
``[h, g]`` depending on which y₀ wins the lucky-substitution search.
"""

from __future__ import annotations

from fractions import Fraction

from cas_factor import BiPoly, try_bivariate_hensel
from cas_factor.hensel import _bi_mul, _bi_normalize


def _make(*terms: tuple[int, int, int]) -> BiPoly:
    """Construct a bivariate polynomial from ``(x_exp, y_exp, coef)`` triples."""
    out: BiPoly = {}
    for i, j, c in terms:
        if c != 0:
            out[(i, j)] = out.get((i, j), Fraction(0)) + Fraction(c)
    return _bi_normalize(out)


def _verify_product(factors: list[BiPoly], expected: BiPoly) -> bool:
    """Multiply ``factors`` and compare with the expected normalised form."""
    prod: BiPoly = {(0, 0): Fraction(1)}
    for f in factors:
        prod = _bi_mul(prod, f)
    return prod == _bi_normalize(expected)


def test_acceptance_x_squared_plus_xy_minus_2y_squared() -> None:
    """Acceptance: ``x² + xy − 2y² = (x + 2y)(x − y)``."""
    # x² + xy − 2y²: (2,0,1), (1,1,1), (0,2,-2)
    f = _make((2, 0, 1), (1, 1, 1), (0, 2, -2))
    result = try_bivariate_hensel(f)
    assert result is not None
    assert len(result) == 2
    assert _verify_product(result, f)


def test_non_unit_leading_coefficient() -> None:
    """``2x² + 3xy − 2y² = (2x − y)(x + 2y)`` — leading coef ≠ 1."""
    f = _make((2, 0, 2), (1, 1, 3), (0, 2, -2))
    result = try_bivariate_hensel(f)
    assert result is not None
    assert len(result) >= 2
    assert _verify_product(result, f)


def test_cubic_difference_of_cubes() -> None:
    """``x³ − y³ = (x − y)(x² + xy + y²)`` — degree-3 bivariate split.

    The univariate image at ``y = 1`` is ``x³ − 1 = (x − 1)(x² + x + 1)``,
    which is squarefree.  Hensel lifts the linear and the quadratic
    factor back to bivariate.  This exercises the multi-degree
    (degree-1 × degree-2) lift path.
    """
    # x³ - y³: (3,0,1), (0,3,-1)
    f = _make((3, 0, 1), (0, 3, -1))
    result = try_bivariate_hensel(f)
    assert result is not None
    assert len(result) == 2
    assert _verify_product(result, f)


def test_irreducible_bivariate_returns_none() -> None:
    """``x² + y² + 1`` is irreducible over ℚ → ``None``."""
    f = _make((2, 0, 1), (0, 2, 1), (0, 0, 1))
    result = try_bivariate_hensel(f)
    assert result is None


def test_already_linear_returns_none() -> None:
    """A bare ``x + y`` cannot be factored further → ``None``."""
    f = _make((1, 0, 1), (0, 1, 1))
    result = try_bivariate_hensel(f)
    # Linear is irreducible — Hensel returns None either because the
    # univariate image is linear (degree < 2 → no factorisation) or no
    # lucky substitution exists.
    assert result is None


def test_univariate_falls_through() -> None:
    """Pure univariate polynomial ``x² − 1`` returns ``None`` so the
    existing univariate path keeps handling it."""
    f = _make((2, 0, 1), (0, 0, -1))
    result = try_bivariate_hensel(f)
    assert result is None
