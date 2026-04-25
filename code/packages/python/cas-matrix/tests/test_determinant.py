"""determinant and inverse via cofactor expansion."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRSymbol

from cas_matrix import (
    MatrixError,
    determinant,
    inverse,
    matrix,
    num_cols,
    num_rows,
)


def Is(*vals: int) -> list[IRInteger]:
    return [IRInteger(v) for v in vals]


# ---- determinant ---------------------------------------------------------


def test_det_1x1() -> None:
    M = matrix([[IRSymbol("a")]])
    assert determinant(M) == IRSymbol("a")


def test_det_2x2_returns_compound_expr() -> None:
    """det([[a, b], [c, d]]) → Sub(Mul(a, d), Mul(b, c))."""
    a, b, c, d = (IRSymbol(s) for s in "abcd")
    M = matrix([[a, b], [c, d]])
    det = determinant(M)
    assert isinstance(det, IRApply)
    assert det.head.name == "Sub"


def test_det_3x3_structure() -> None:
    """3x3 expands to a sum of three terms (signs alternate)."""
    M = matrix([Is(1, 2, 3), Is(4, 5, 6), Is(7, 8, 9)])
    det = determinant(M)
    assert isinstance(det, IRApply)
    assert det.head.name == "Add"
    assert len(det.args) == 3


def test_det_non_square_raises() -> None:
    M = matrix([Is(1, 2, 3)])
    with pytest.raises(MatrixError):
        determinant(M)


# ---- inverse -------------------------------------------------------------


def test_inverse_2x2_shape() -> None:
    a, b, c, d = (IRSymbol(s) for s in "abcd")
    M = matrix([[a, b], [c, d]])
    inv = inverse(M)
    assert num_rows(inv) == 2
    assert num_cols(inv) == 2


def test_inverse_non_square_raises() -> None:
    M = matrix([Is(1, 2, 3)])
    with pytest.raises(MatrixError):
        inverse(M)


def test_inverse_1x1_shape() -> None:
    M = matrix([[IRSymbol("a")]])
    inv = inverse(M)
    assert num_rows(inv) == 1
    assert num_cols(inv) == 1
