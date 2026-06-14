"""Tests for monomial ordering utilities."""

from __future__ import annotations

import pytest

from cas_multivariate.monomial import (
    cmp_monomials,
    div_monomial,
    divides,
    lcm_monomial,
    monomial_key,
    total_degree,
)

# ---------------------------------------------------------------------------
# grlex ordering
# ---------------------------------------------------------------------------


def test_grlex_order_same_degree():
    """(2,1) > (1,2) in grlex because same total degree but (2,1) wins lex."""
    key = monomial_key("grlex")
    assert key((2, 1)) > key((1, 2))


def test_grlex_order_different_degree():
    """(3, 0) > (1, 1) in grlex because total degree 3 > 2."""
    key = monomial_key("grlex")
    assert key((3, 0)) > key((1, 1))


def test_grlex_order_zero_vs_nonzero():
    """(1, 0) > (0, 0) in grlex."""
    key = monomial_key("grlex")
    assert key((1, 0)) > key((0, 0))


def test_grlex_sort_three_monomials():
    """Sort [(0,3),(2,1),(1,2)] in grlex descending gives [(2,1),(1,2),(0,3)]."""
    monomials = [(0, 3), (2, 1), (1, 2)]
    sorted_monomials = sorted(monomials, key=monomial_key("grlex"), reverse=True)
    assert sorted_monomials == [(2, 1), (1, 2), (0, 3)]


# ---------------------------------------------------------------------------
# lex ordering
# ---------------------------------------------------------------------------


def test_lex_order_basic():
    """(2,0) > (1,5) in lex because first coordinate 2 > 1."""
    key = monomial_key("lex")
    assert key((2, 0)) > key((1, 5))


def test_lex_order_tiebreak():
    """(1, 3) > (1, 2) in lex (first coords tie, second 3 > 2)."""
    key = monomial_key("lex")
    assert key((1, 3)) > key((1, 2))


def test_lex_order_equal():
    """(1, 2) == (1, 2) in lex."""
    key = monomial_key("lex")
    assert key((1, 2)) == key((1, 2))


def test_lex_vs_grlex_differ():
    """lex and grlex differ: lex puts (1,1) > (0,3); grlex puts (0,3) > (1,1)."""
    lex = monomial_key("lex")
    grlex = monomial_key("grlex")
    # In lex: first coord 1 > 0 so (1,1) > (0,3).
    assert lex((1, 1)) > lex((0, 3))
    # In grlex: total degree of (0,3) is 3 > 2 of (1,1), so (0,3) > (1,1).
    assert grlex((0, 3)) > grlex((1, 1))


# ---------------------------------------------------------------------------
# grevlex ordering
# ---------------------------------------------------------------------------


def test_grevlex_order():
    """(1,1,1) vs (3,0,0) in grevlex: both degree 3.
    grevlex compares reversed tuples negated. (1,1,1)→ neg rev: (-1,-1,-1),
    (3,0,0) → neg rev: (0,0,-3). So (-1,-1,-1) < (0,0,-3), meaning (3,0,0) > (1,1,1)
    ... actually let me recompute:
    neg rev of (1,1,1) = (-1,-1,-1)
    neg rev of (3,0,0) = (0,0,-3)
    (-1,-1,-1) < (0,0,-3) because at first position -1 < 0.
    So grevlex key of (3,0,0) > grevlex key of (1,1,1).
    """
    key = monomial_key("grevlex")
    assert key((3, 0, 0)) > key((1, 1, 1))


def test_grevlex_same_degree():
    """Two monomials of same degree, grevlex uses reversed tiebreak."""
    key = monomial_key("grevlex")
    # (2,1,0) vs (1,2,0): degree 3 both.
    # neg rev (2,1,0) = (0,-1,-2); neg rev (1,2,0) = (0,-2,-1)
    # Compare: (0,-1,-2) vs (0,-2,-1): first equal, second -1 > -2 → (2,1,0) wins.
    assert key((2, 1, 0)) > key((1, 2, 0))


def test_invalid_order():
    """Unknown ordering name raises ValueError."""
    with pytest.raises(ValueError, match="Unknown monomial order"):
        monomial_key("blex")


# ---------------------------------------------------------------------------
# cmp_monomials
# ---------------------------------------------------------------------------


def test_cmp_monomials_gt():
    """cmp_monomials returns +1 when first > second."""
    assert cmp_monomials((2, 1), (1, 2), "grlex") == 1


def test_cmp_monomials_lt():
    """cmp_monomials returns -1 when first < second."""
    assert cmp_monomials((1, 2), (2, 1), "grlex") == -1


def test_cmp_monomials_eq():
    """cmp_monomials returns 0 when equal."""
    assert cmp_monomials((1, 1), (1, 1), "grlex") == 0


# ---------------------------------------------------------------------------
# lcm_monomial
# ---------------------------------------------------------------------------


def test_lcm_monomial_basic():
    """lcm((2,1,0), (1,2,3)) = (2,2,3)."""
    assert lcm_monomial((2, 1, 0), (1, 2, 3)) == (2, 2, 3)


def test_lcm_monomial_zeros():
    """lcm((0,0), (0,0)) = (0,0)."""
    assert lcm_monomial((0, 0), (0, 0)) == (0, 0)


def test_lcm_monomial_one_side_zero():
    """lcm((3,0), (0,2)) = (3,2)."""
    assert lcm_monomial((3, 0), (0, 2)) == (3, 2)


# ---------------------------------------------------------------------------
# divides
# ---------------------------------------------------------------------------


def test_divides_true():
    """(1,1) divides (2,3)."""
    assert divides((1, 1), (2, 3)) is True


def test_divides_false():
    """(2,1) does not divide (1,2)."""
    assert divides((2, 1), (1, 2)) is False


def test_divides_one_divides_all():
    """(0,0) divides everything."""
    assert divides((0, 0), (5, 7)) is True


def test_divides_self():
    """Every monomial divides itself."""
    assert divides((3, 2, 1), (3, 2, 1)) is True


# ---------------------------------------------------------------------------
# div_monomial
# ---------------------------------------------------------------------------


def test_div_monomial_basic():
    """(3,2) / (1,1) = (2,1)."""
    assert div_monomial((3, 2), (1, 1)) == (2, 1)


def test_div_monomial_self():
    """Monomial / itself = (0,0)."""
    assert div_monomial((2, 0), (2, 0)) == (0, 0)


# ---------------------------------------------------------------------------
# total_degree
# ---------------------------------------------------------------------------


def test_total_degree_basic():
    """total_degree((3,2,1)) = 6."""
    assert total_degree((3, 2, 1)) == 6


def test_total_degree_zero():
    """total_degree((0,0)) = 0."""
    assert total_degree((0, 0)) == 0
