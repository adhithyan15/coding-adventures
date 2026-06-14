"""Tests for Buchberger's algorithm and S-polynomial."""

from __future__ import annotations

from fractions import Fraction

import pytest

from cas_multivariate.groebner import GrobnerError, buchberger
from cas_multivariate.polynomial import MPoly
from cas_multivariate.reduce import reduce_poly, s_poly

F = Fraction


# ---------------------------------------------------------------------------
# S-polynomial tests
# ---------------------------------------------------------------------------


def test_spoly_basic():
    """S(x^2, x*y) in Q[x, y] (grlex)."""
    # f = x^2,  g = x*y
    # lm(f) = (2,0), lm(g) = (1,1)
    # lcm = (2,1)
    # S = (y * x^2) - (x * xy) = x^2 y - x^2 y = 0
    f = MPoly({(2, 0): F(1)}, 2)
    g = MPoly({(1, 1): F(1)}, 2)
    sp = s_poly(f, g, "grlex")
    assert sp.is_zero()


def test_spoly_nonzero():
    """S-polynomial of x^2 + y and x*y + 1."""
    # f = x^2 + y,  g = x*y + 1
    # lt(f) = x^2, lt(g) = x*y (grlex, both degree 2; (2,0)>(1,1) lex)
    # lcm((2,0),(1,1)) = (2,1)
    # S = y*(x^2+y) - x*(xy+1) = x^2y + y^2 - x^2y - x = y^2 - x
    f = MPoly({(2, 0): F(1), (0, 1): F(1)}, 2)
    g = MPoly({(1, 1): F(1), (0, 0): F(1)}, 2)
    sp = s_poly(f, g, "grlex")
    expected = MPoly({(0, 2): F(1), (1, 0): F(-1)}, 2)
    assert sp == expected


def test_spoly_linear():
    """S-polynomial of two linear polynomials."""
    # f = x + y - 1,  g = x - y
    # lt(f) = x (lex), lt(g) = x
    # lcm = x
    # S = (x/(x)) * (x+y-1) - (x/x) * (x-y) = (x+y-1) - (x-y) = 2y - 1
    f = MPoly({(1, 0): F(1), (0, 1): F(1), (0, 0): F(-1)}, 2)
    g = MPoly({(1, 0): F(1), (0, 1): F(-1)}, 2)
    sp = s_poly(f, g, "lex")
    expected = MPoly({(0, 1): F(2), (0, 0): F(-1)}, 2)
    assert sp == expected


def test_spoly_zero_raises():
    """S-polynomial with zero polynomial raises AssertionError."""
    f = MPoly({(1, 0): F(1)}, 2)
    z = MPoly.zero(2)
    with pytest.raises(AssertionError):
        s_poly(f, z)


# ---------------------------------------------------------------------------
# Reduction tests
# ---------------------------------------------------------------------------


def test_reduce_to_zero():
    """x^2 - 1 reduces to zero by [x - 1, x + 1]."""
    f = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)    # x^2 - 1 (in Q[x,y])
    g1 = MPoly({(1, 0): F(1), (0, 0): F(-1)}, 2)   # x - 1
    # f = (x+1)(x-1), so it reduces to 0 by [x-1] after 2 steps.
    r = reduce_poly(f, [g1], "lex")
    # x^2 - 1 / (x-1) = x+1 remainder 0
    assert r.is_zero()


def test_reduce_remainder():
    """x^2 + y reduces by [x^2 - 1] to y + 1."""
    f = MPoly({(2, 0): F(1), (0, 1): F(1)}, 2)     # x^2 + y
    g = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)    # x^2 - 1
    r = reduce_poly(f, [g], "grlex")
    expected = MPoly({(0, 1): F(1), (0, 0): F(1)}, 2)  # y + 1
    assert r == expected


def test_reduce_by_empty():
    """Reducing by empty list returns f unchanged."""
    f = MPoly({(2, 0): F(1), (0, 1): F(1)}, 2)
    r = reduce_poly(f, [], "grlex")
    assert r == f


def test_reduce_zero():
    """Reducing zero gives zero."""
    r = reduce_poly(MPoly.zero(2), [MPoly({(1, 0): F(1)}, 2)], "grlex")
    assert r.is_zero()


# ---------------------------------------------------------------------------
# Buchberger's algorithm
# ---------------------------------------------------------------------------


def test_groebner_empty():
    """Empty input gives empty basis."""
    G = buchberger([], order="grlex")
    assert G == []


def test_groebner_all_zeros():
    """All-zero input gives empty basis."""
    G = buchberger([MPoly.zero(2), MPoly.zero(2)], order="grlex")
    assert G == []


def test_groebner_single():
    """Single polynomial — basis is just that polynomial (monic)."""
    f = MPoly({(1, 0): F(2), (0, 0): F(-4)}, 2)   # 2x - 4 → monic: x - 2
    G = buchberger([f], order="lex")
    assert len(G) == 1
    g = G[0]
    # Should be monic
    assert g.lc("lex") == F(1)


def test_groebner_xy_system():
    """[x + y - 1, x - y] → Gröbner basis (lex) contains x - 1/2, y - 1/2."""
    f1 = MPoly({(1, 0): F(1), (0, 1): F(1), (0, 0): F(-1)}, 2)  # x + y - 1
    f2 = MPoly({(1, 0): F(1), (0, 1): F(-1)}, 2)                  # x - y
    G = buchberger([f1, f2], order="lex")
    # Every basis element should reduce f1 and f2 to zero.
    assert reduce_poly(f1, G, "lex").is_zero()
    assert reduce_poly(f2, G, "lex").is_zero()
    # There should be 2 elements in the reduced basis.
    assert len(G) == 2


def test_groebner_quadratic():
    """[x^2 - 1, y - x] has a basis that encodes x=±1, y=x."""
    f1 = MPoly({(2, 0): F(1), (0, 0): F(-1)}, 2)   # x^2 - 1
    f2 = MPoly({(0, 1): F(1), (1, 0): F(-1)}, 2)   # y - x
    G = buchberger([f1, f2], order="lex")
    assert reduce_poly(f1, G, "lex").is_zero()
    assert reduce_poly(f2, G, "lex").is_zero()


def test_groebner_already_basis():
    """[x, y] is already a Gröbner basis (independent linear forms)."""
    f1 = MPoly({(1, 0): F(1)}, 2)   # x
    f2 = MPoly({(0, 1): F(1)}, 2)   # y
    G = buchberger([f1, f2], order="grlex")
    assert reduce_poly(f1, G, "grlex").is_zero()
    assert reduce_poly(f2, G, "grlex").is_zero()


def test_groebner_linear_system():
    """Linear system [x + 2y - 5, 2x - y - 1] has unique solution x=11/5, y=9/5."""
    # x + 2y = 5, 2x - y = 1
    f1 = MPoly({(1, 0): F(1), (0, 1): F(2), (0, 0): F(-5)}, 2)
    f2 = MPoly({(1, 0): F(2), (0, 1): F(-1), (0, 0): F(-1)}, 2)
    G = buchberger([f1, f2], order="lex")
    assert reduce_poly(f1, G, "lex").is_zero()
    assert reduce_poly(f2, G, "lex").is_zero()


def test_groebner_univariate():
    """Univariate polynomial stays a valid basis."""
    # p(x) = x^2 - 3x + 2 = (x-1)(x-2) in Q[x,y]
    f = MPoly({(2, 0): F(1), (1, 0): F(-3), (0, 0): F(2)}, 2)
    G = buchberger([f], order="lex")
    assert reduce_poly(f, G, "lex").is_zero()


def test_groebner_basis_is_generating_set():
    """Every element of the original input reduces to zero by the basis."""
    f1 = MPoly({(2, 1): F(1), (0, 0): F(-1)}, 2)   # x^2*y - 1
    f2 = MPoly({(1, 2): F(1), (0, 0): F(-1)}, 2)   # x*y^2 - 1
    G = buchberger([f1, f2], order="grlex")
    assert reduce_poly(f1, G, "grlex").is_zero()
    assert reduce_poly(f2, G, "grlex").is_zero()


def test_groebner_degree_limit():
    """Polynomial with degree > 8 raises GrobnerError."""
    # x^9 (degree 9 > 8)
    f = MPoly({(9, 0): F(1)}, 2)
    with pytest.raises(GrobnerError):
        buchberger([f], order="grlex")
