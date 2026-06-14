"""Tests for the MPoly multivariate polynomial class."""

from __future__ import annotations

from fractions import Fraction

import pytest

from cas_multivariate.polynomial import MPoly, div_reduction_step, make_var

F = Fraction


# ---------------------------------------------------------------------------
# Construction and zero checks
# ---------------------------------------------------------------------------


def test_mpoly_zero():
    """MPoly.zero(2) is the zero polynomial."""
    p = MPoly.zero(2)
    assert p.is_zero()
    assert not bool(p)
    assert p.coeffs == {}


def test_mpoly_constant():
    """MPoly.constant creates a constant polynomial."""
    p = MPoly.constant(F(3, 2), 2)
    assert not p.is_zero()
    assert p.coeffs == {(0, 0): F(3, 2)}


def test_mpoly_constant_zero():
    """MPoly.constant(0, n) is the zero polynomial."""
    p = MPoly.constant(0, 3)
    assert p.is_zero()


def test_mpoly_monomial_poly():
    """MPoly.monomial_poly creates a single-term polynomial."""
    p = MPoly.monomial_poly((2, 1), F(3), 2)
    assert p.coeffs == {(2, 1): F(3)}


def test_mpoly_zero_coeff_cleaned():
    """Zero coefficients are automatically cleaned out."""
    p = MPoly({(2, 0): F(1), (0, 1): F(0), (0, 0): F(-1)}, 2)
    assert (0, 1) not in p.coeffs


# ---------------------------------------------------------------------------
# Leading term
# ---------------------------------------------------------------------------


def test_mpoly_lm_grlex():
    """Leading monomial in grlex order."""
    p = MPoly({(2, 1): F(1), (0, 3): F(2)}, 2)
    assert p.lm("grlex") == (2, 1)  # same degree, (2,1) > (0,3) lex


def test_mpoly_lm_lex():
    """Leading monomial in lex order."""
    p = MPoly({(2, 1): F(1), (0, 3): F(2)}, 2)
    assert p.lm("lex") == (2, 1)


def test_mpoly_lc():
    """Leading coefficient."""
    p = MPoly({(2, 1): F(3), (0, 3): F(2)}, 2)
    assert p.lc("grlex") == F(3)


def test_mpoly_lt():
    """Leading term is a single-term polynomial."""
    p = MPoly({(2, 1): F(3), (0, 1): F(1)}, 2)
    lt = p.lt("grlex")
    assert lt.coeffs == {(2, 1): F(3)}


def test_mpoly_lm_zero_raises():
    """Leading monomial of zero raises ValueError."""
    p = MPoly.zero(2)
    with pytest.raises(ValueError):
        p.lm("grlex")


# ---------------------------------------------------------------------------
# Arithmetic
# ---------------------------------------------------------------------------


def test_mpoly_add():
    """Add two polynomials."""
    p = MPoly({(1, 0): F(2)}, 2)
    q = MPoly({(1, 0): F(-1), (0, 1): F(3)}, 2)
    r = p + q
    assert r.coeffs == {(1, 0): F(1), (0, 1): F(3)}


def test_mpoly_add_cancels():
    """Addition that cancels a term."""
    p = MPoly({(1, 0): F(1)}, 2)
    q = MPoly({(1, 0): F(-1)}, 2)
    r = p + q
    assert r.is_zero()


def test_mpoly_neg():
    """Negation flips signs."""
    p = MPoly({(2, 0): F(2), (0, 0): F(-1)}, 2)
    r = -p
    assert r.coeffs == {(2, 0): F(-2), (0, 0): F(1)}


def test_mpoly_sub():
    """Subtraction."""
    p = MPoly({(1, 0): F(3)}, 2)
    q = MPoly({(1, 0): F(1), (0, 1): F(2)}, 2)
    r = p - q
    assert r.coeffs == {(1, 0): F(2), (0, 1): F(-2)}


def test_mpoly_mul():
    """(x + 1)(x - 1) = x^2 - 1."""
    p = MPoly({(1, 0): F(1), (0, 0): F(1)}, 2)   # x + 1 (nvars=2)
    q = MPoly({(1, 0): F(1), (0, 0): F(-1)}, 2)  # x - 1
    r = p * q
    assert r.coeffs == {(2, 0): F(1), (0, 0): F(-1)}


def test_mpoly_mul_two_vars():
    """(x + y)(x - y) = x^2 - y^2."""
    p = MPoly({(1, 0): F(1), (0, 1): F(1)}, 2)   # x + y
    q = MPoly({(1, 0): F(1), (0, 1): F(-1)}, 2)  # x - y
    r = p * q
    assert r.coeffs == {(2, 0): F(1), (0, 2): F(-1)}


def test_mpoly_scale():
    """Scalar multiplication."""
    p = MPoly({(2, 0): F(2)}, 2)
    r = p.scale(F(3, 2))
    assert r.coeffs == {(2, 0): F(3)}


def test_mpoly_scale_zero():
    """Scale by zero gives zero."""
    p = MPoly({(2, 0): F(2)}, 2)
    assert p.scale(0).is_zero()


def test_mpoly_mul_monomial():
    """Multiply by monomial x·(x+1) = x^2 + x."""
    p = MPoly({(1, 0): F(1), (0, 0): F(1)}, 2)
    r = p.mul_monomial((1, 0))
    assert r.coeffs == {(2, 0): F(1), (1, 0): F(1)}


# ---------------------------------------------------------------------------
# Equality
# ---------------------------------------------------------------------------


def test_mpoly_eq():
    """Two polynomials with the same coefficients are equal."""
    p = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)
    q = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)
    assert p == q


def test_mpoly_neq():
    """Different polynomials are not equal."""
    p = MPoly({(2, 0): F(1)}, 2)
    q = MPoly({(1, 0): F(1)}, 2)
    assert p != q


def test_mpoly_eq_not_ir():
    """MPoly equality with non-MPoly returns NotImplemented."""
    p = MPoly({(1, 0): F(1)}, 2)
    assert p.__eq__(42) is NotImplemented


# ---------------------------------------------------------------------------
# Utility methods
# ---------------------------------------------------------------------------


def test_mpoly_total_degree():
    """Total degree is the max sum of exponents."""
    p = MPoly({(2, 1): F(1), (0, 3): F(1)}, 2)
    assert p.total_degree() == 3


def test_mpoly_total_degree_zero():
    """Zero polynomial has total degree 0."""
    assert MPoly.zero(2).total_degree() == 0


def test_mpoly_is_univariate():
    """Polynomial in x only."""
    p = MPoly({(2, 0): F(3), (0, 0): F(1)}, 2)
    assert p.is_univariate() == 0


def test_mpoly_is_univariate_y():
    """Polynomial in y only."""
    p = MPoly({(0, 2): F(1)}, 2)
    assert p.is_univariate() == 1


def test_mpoly_is_not_univariate():
    """x*y is not univariate."""
    p = MPoly({(1, 1): F(1)}, 2)
    assert p.is_univariate() is None


def test_mpoly_to_univariate_coeffs():
    """x^2 - 1 → [F(-1), F(0), F(1)] (ascending degree)."""
    p = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)
    assert p.to_univariate_coeffs(0) == [F(-1), F(0), F(1)]


def test_mpoly_diff():
    """Partial derivative d/dx (x^2 y + x) = 2xy + 1."""
    p = MPoly({(2, 1): F(1), (1, 0): F(1)}, 2)
    dp = p.diff(0)
    assert dp.coeffs == {(1, 1): F(2), (0, 0): F(1)}


def test_mpoly_eval_at():
    """Substitute x=2 into x^2 + y → 4 + y."""
    p = MPoly({(2, 0): F(1), (0, 1): F(1)}, 2)
    result = p.eval_at(0, F(2))
    assert result.coeffs == {(0, 1): F(1), (0, 0): F(4)}


def test_make_var():
    """make_var creates a single-variable polynomial."""
    x = make_var(0, 2)
    assert x.coeffs == {(1, 0): F(1)}
    y = make_var(1, 2)
    assert y.coeffs == {(0, 1): F(1)}


# ---------------------------------------------------------------------------
# div_reduction_step
# ---------------------------------------------------------------------------


def test_div_reduction_step_applies():
    """One reduction step: x^2 / x = x (coefficient 1)."""
    f = MPoly({(2, 0): F(1)}, 2)
    g = MPoly({(1, 0): F(1)}, 2)
    result = div_reduction_step(f, g, "lex")
    assert result is not None
    term, new_f = result
    assert term.coeffs == {(1, 0): F(1)}
    assert new_f.is_zero()


def test_div_reduction_step_no_apply():
    """No reduction step when lm(g) does not divide lm(f)."""
    f = MPoly({(0, 1): F(1)}, 2)  # y
    g = MPoly({(1, 0): F(1)}, 2)  # x
    result = div_reduction_step(f, g, "lex")
    assert result is None


def test_div_reduction_step_zero_f():
    """No reduction step when f is zero."""
    f = MPoly.zero(2)
    g = MPoly({(1, 0): F(1)}, 2)
    result = div_reduction_step(f, g, "lex")
    assert result is None


def test_mpoly_repr():
    """repr returns something sensible."""
    p = MPoly({(2, 0): F(1)}, 2)
    r = repr(p)
    assert "MPoly" in r


def test_mpoly_monomials_descending():
    """monomials_descending returns monomials in descending order."""
    p = MPoly({(2, 0): F(1), (0, 2): F(1), (1, 1): F(1)}, 2)
    monomials = p.monomials_descending("grlex")
    # All degree 2; lex order: (2,0) > (1,1) > (0,2)
    assert monomials == [(2, 0), (1, 1), (0, 2)]
