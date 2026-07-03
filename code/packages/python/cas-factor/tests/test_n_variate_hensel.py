"""Tests for n-variate (n ≥ 3) Hensel lifting in ``cas_factor.hensel``.

Track K1 of ``code/specs/macsyma-truly-finish-plan.md``: extend the
bivariate Hensel lift to handle 3+ variables via iterated lifting.

Test strategy:

* For each test polynomial we know factors, multiply the factors out
  via :func:`_n_mul`, run :func:`try_n_variate_hensel`, and check that
  (a) the routine returned at least two factors and (b) their product
  equals the input.  Factor ordering is not pinned — different
  specialisation tuples may yield ``[g, h]`` or ``[h, g]``.
* For irreducibles and degenerate inputs we assert ``None``, so the
  caller cleanly falls through to other handlers.
* Regression: the bivariate routine continues to handle n=2 cases via
  the n-variate front door — they should give the same factorisation.
"""

from __future__ import annotations

from fractions import Fraction

from cas_factor import NPoly, try_n_variate_hensel
from cas_factor.hensel import _n_mul, _n_normalize


def _make(num_vars: int, *terms: tuple[tuple[int, ...], int]) -> NPoly:
    """Construct an n-variate polynomial from ``(exponent_tuple, coef)`` pairs."""
    out: NPoly = {}
    for k, c in terms:
        assert len(k) == num_vars, "exponent tuple length must match num_vars"
        if c != 0:
            out[k] = out.get(k, Fraction(0)) + Fraction(c)
    return _n_normalize(out)


def _verify_product(num_vars: int, factors: list[NPoly], expected: NPoly) -> bool:
    """Multiply ``factors`` and check equality with ``expected``."""
    prod: NPoly = {tuple([0] * num_vars): Fraction(1)}
    for f in factors:
        prod = _n_mul(prod, f)
    return prod == _n_normalize(expected)


# ---------------------------------------------------------------------------
# Acceptance: the trivariate quadratic from the spec.
# ---------------------------------------------------------------------------


def test_trivariate_quadratic_three_factors_known() -> None:
    """``x² − y² − z² − 2yz = (x + y + z)(x − y − z)`` — the canonical
    acceptance case from Track K1.

    The univariate image at any non-degenerate ``(y₀, z₀)`` is a degree-2
    polynomial with rational roots; Kronecker (or even just the rational
    root test) finds the linear factors and the n-variate lift glues
    them back together.
    """
    # x² − y² − z² − 2yz
    poly = _make(
        3,
        ((2, 0, 0), 1),
        ((0, 2, 0), -1),
        ((0, 0, 2), -1),
        ((0, 1, 1), -2),
    )
    result = try_n_variate_hensel(poly, 3)
    assert result is not None, "expected non-trivial trivariate factorisation"
    assert len(result) >= 2
    assert _verify_product(3, result, poly)


def test_trivariate_product_of_two_linear() -> None:
    """``(x + y + z)(x + 2y + 3z) = x² + 3xy + 4xz + 2y² + 5yz + 3z²``.

    A second canonical trivariate target — exercises the lift over a
    factoring whose second factor depends asymmetrically on the two
    auxiliary variables.
    """
    poly = _make(
        3,
        ((2, 0, 0), 1),
        ((1, 1, 0), 3),
        ((1, 0, 1), 4),
        ((0, 2, 0), 2),
        ((0, 1, 1), 5),
        ((0, 0, 2), 3),
    )
    result = try_n_variate_hensel(poly, 3)
    assert result is not None
    assert len(result) == 2
    assert _verify_product(3, result, poly)


def test_trivariate_cubic_linear_times_quadratic() -> None:
    """``(x + y + z)(x² + y² + z² − xy − yz − xz)`` — the
    sum-of-cubes companion identity::

        x³ + y³ + z³ − 3xyz = (x + y + z)(x² + y² + z² − xy − yz − xz).

    Expand the right-hand side to construct ``poly`` then verify Hensel
    recovers a non-trivial factorisation whose product equals it.
    """
    factor_a = _make(3, ((1, 0, 0), 1), ((0, 1, 0), 1), ((0, 0, 1), 1))
    factor_b = _make(
        3,
        ((2, 0, 0), 1),
        ((0, 2, 0), 1),
        ((0, 0, 2), 1),
        ((1, 1, 0), -1),
        ((0, 1, 1), -1),
        ((1, 0, 1), -1),
    )
    poly = _n_mul(factor_a, factor_b)
    result = try_n_variate_hensel(poly, 3)
    assert result is not None
    assert len(result) >= 2
    assert _verify_product(3, result, poly)


def test_quadrivariate_linear_times_linear() -> None:
    """``(x + y)(x + z + w)`` over four variables.

    Exercises the iterated lift across **two** auxiliary variables (the
    lift goes y → z → w, each step a separate Hensel iteration).
    """
    factor_a = _make(4, ((1, 0, 0, 0), 1), ((0, 1, 0, 0), 1))
    factor_b = _make(
        4, ((1, 0, 0, 0), 1), ((0, 0, 1, 0), 1), ((0, 0, 0, 1), 1)
    )
    poly = _n_mul(factor_a, factor_b)
    result = try_n_variate_hensel(poly, 4)
    assert result is not None
    assert len(result) == 2
    assert _verify_product(4, result, poly)


# ---------------------------------------------------------------------------
# Fall-through cases — Hensel cleanly declines.
# ---------------------------------------------------------------------------


def test_irreducible_trivariate_returns_none() -> None:
    """``x² + y² + z² + 1`` is irreducible over ℚ → ``None``."""
    poly = _make(
        3,
        ((2, 0, 0), 1),
        ((0, 2, 0), 1),
        ((0, 0, 2), 1),
        ((0, 0, 0), 1),
    )
    assert try_n_variate_hensel(poly, 3) is None


def test_single_variable_returns_none() -> None:
    """A polynomial mentioning only one variable falls through so the
    univariate factoring pipeline keeps handling it.

    We pad with zero-exponent slots for the other variables to model a
    poly registered in a 3-variable ring but actually univariate.
    """
    poly = _make(3, ((2, 0, 0), 1), ((0, 0, 0), -1))  # x² − 1
    assert try_n_variate_hensel(poly, 3) is None


def test_num_vars_less_than_two_returns_none() -> None:
    """The n-variate routine requires at least two variables; with
    ``num_vars=1`` it bows out and lets the univariate path handle it."""
    poly = _make(1, ((2,), 1), ((0,), -1))  # x² − 1 in a 1-var ring
    assert try_n_variate_hensel(poly, 1) is None


def test_constant_returns_none() -> None:
    """A pure constant has nothing to factor in this ring → ``None``."""
    poly = _make(3, ((0, 0, 0), 7))
    assert try_n_variate_hensel(poly, 3) is None


def test_empty_polynomial_returns_none() -> None:
    """The zero polynomial returns ``None`` (no factoring to discuss)."""
    assert try_n_variate_hensel({}, 3) is None


def test_linear_polynomial_returns_none() -> None:
    """A single linear polynomial ``x + y + z`` is already irreducible
    and the routine should return ``None``."""
    poly = _make(3, ((1, 0, 0), 1), ((0, 1, 0), 1), ((0, 0, 1), 1))
    assert try_n_variate_hensel(poly, 3) is None


# ---------------------------------------------------------------------------
# Regression: bivariate inputs (num_vars=2) go through the n-variate
# front door and yield identical factorings to ``try_bivariate_hensel``.
# ---------------------------------------------------------------------------


def test_bivariate_via_n_variate_x_squared_plus_xy_minus_2y_squared() -> None:
    """Regression: ``x² + xy − 2y²`` factors via the n-variate routine.

    Establishes that the n-variate dispatcher subsumes the existing
    bivariate case (the Track D1 acceptance criterion).
    """
    poly = _make(2, ((2, 0), 1), ((1, 1), 1), ((0, 2), -2))
    result = try_n_variate_hensel(poly, 2)
    assert result is not None
    assert len(result) == 2
    assert _verify_product(2, result, poly)


def test_bivariate_via_n_variate_x_cubed_minus_y_cubed() -> None:
    """Regression: ``x³ − y³`` factors into linear × quadratic via the
    n-variate front door."""
    poly = _make(2, ((3, 0), 1), ((0, 3), -1))
    result = try_n_variate_hensel(poly, 2)
    assert result is not None
    assert len(result) == 2
    assert _verify_product(2, result, poly)


# ---------------------------------------------------------------------------
# Robustness: bounded resource discipline.
# ---------------------------------------------------------------------------


def test_high_degree_irreducible_does_not_loop() -> None:
    """A degree-6 irreducible polynomial in three variables must return
    ``None`` in bounded time (it should not loop indefinitely searching
    for a lucky specialisation point).

    We pick a polynomial whose univariate image at any specialisation
    in our enumeration is irreducible.  The routine should burn through
    its bounded specialisation list and then give up cleanly.
    """
    # x^4 + 1 in x (irreducible over Q) + a tiny coupling to y, z that
    # doesn't change the univariate image's structure.
    poly = _make(
        3,
        ((4, 0, 0), 1),
        ((0, 0, 0), 1),
        ((0, 0, 2), 1),
        ((0, 2, 0), 1),
    )
    # x^4 + y^2 + z^2 + 1.  Specialisation y=z=0 gives x^4+1 (irreducible
    # over Q via BZH).  Specialisation y=z=1 gives x^4 + 3, also irreducible.
    result = try_n_variate_hensel(poly, 3)
    assert result is None
