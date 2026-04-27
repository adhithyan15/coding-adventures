"""transpose, identity, zero, add, sub, scalar_multiply, dot, trace."""

from __future__ import annotations

import pytest
from symbolic_ir import IRInteger, IRSymbol

from cas_matrix import (
    MatrixError,
    add_matrices,
    dot,
    identity_matrix,
    matrix,
    num_cols,
    num_rows,
    scalar_multiply,
    sub_matrices,
    trace,
    transpose,
    zero_matrix,
)


def Is(*vals: int) -> list[IRInteger]:
    """Build a list of IRInteger nodes (short helper for test expressions)."""
    return [IRInteger(v) for v in vals]


# ---- identity / zero -----------------------------------------------------


def test_identity_3x3() -> None:
    Id = identity_matrix(3)
    expected = matrix([Is(1, 0, 0), Is(0, 1, 0), Is(0, 0, 1)])
    assert Id == expected


def test_zero_2x4() -> None:
    Z = zero_matrix(2, 4)
    assert num_rows(Z) == 2
    assert num_cols(Z) == 4


# ---- transpose -----------------------------------------------------------


def test_transpose_square() -> None:
    M = matrix([Is(1, 2), Is(3, 4)])
    T = transpose(M)
    assert matrix([Is(1, 3), Is(2, 4)]) == T


def test_transpose_rectangular() -> None:
    M = matrix([Is(1, 2, 3), Is(4, 5, 6)])
    T = transpose(M)
    assert matrix([Is(1, 4), Is(2, 5), Is(3, 6)]) == T


def test_transpose_double_inverse() -> None:
    """transpose(transpose(M)) == M."""
    M = matrix([[IRSymbol("a"), IRSymbol("b")], [IRSymbol("c"), IRSymbol("d")]])
    assert transpose(transpose(M)) == M


# ---- elementwise ---------------------------------------------------------


def test_add_matrices_shape() -> None:
    A = matrix([Is(1, 2), Is(3, 4)])
    B = matrix([Is(5, 6), Is(7, 8)])
    out = add_matrices(A, B)
    assert num_rows(out) == 2
    assert num_cols(out) == 2


def test_add_shape_mismatch() -> None:
    A = matrix([Is(1, 2)])
    B = matrix([Is(1)])
    with pytest.raises(MatrixError):
        add_matrices(A, B)


def test_sub_matrices() -> None:
    A = matrix([Is(1, 2)])
    B = matrix([Is(3, 4)])
    out = sub_matrices(A, B)
    assert num_rows(out) == 1


def test_scalar_multiply() -> None:
    M = matrix([Is(1, 2)])
    out = scalar_multiply(IRInteger(3), M)
    assert num_rows(out) == 1
    assert num_cols(out) == 2


# ---- dot -----------------------------------------------------------------


def test_dot_shape_check() -> None:
    A = matrix([Is(1, 2)])  # 1x2
    B = matrix([Is(3), Is(4)])  # 2x1
    out = dot(A, B)
    # 1x1 result
    assert num_rows(out) == 1
    assert num_cols(out) == 1


def test_dot_mismatch_raises() -> None:
    A = matrix([Is(1, 2)])  # 1x2
    B = matrix([Is(3, 4)])  # 1x2 — incompatible
    with pytest.raises(MatrixError):
        dot(A, B)


def test_dot_3x3_with_identity() -> None:
    """A . I == A."""
    A = matrix([Is(1, 2, 3), Is(4, 5, 6), Is(7, 8, 9)])
    Id = identity_matrix(3)
    out = dot(A, Id)
    assert num_rows(out) == 3
    assert num_cols(out) == 3


# ---- trace ---------------------------------------------------------------


def test_trace_square() -> None:
    M = matrix([Is(1, 2), Is(3, 4)])
    out = trace(M)
    assert out is not None  # symbolic Add(1, 4)


def test_trace_non_square_raises() -> None:
    M = matrix([Is(1, 2, 3)])
    with pytest.raises(MatrixError):
        trace(M)


def test_trace_singleton_returns_entry() -> None:
    """1x1 trace returns the single entry."""
    M = matrix([[IRSymbol("a")]])
    assert trace(M) == IRSymbol("a")
