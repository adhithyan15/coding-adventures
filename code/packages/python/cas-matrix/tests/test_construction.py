"""matrix(), dimensions(), get_entry()."""

from __future__ import annotations

import pytest
from symbolic_ir import IRApply, IRInteger, IRSymbol

from cas_matrix import (
    LIST,
    MATRIX,
    MatrixError,
    dimensions,
    get_entry,
    is_matrix,
    matrix,
    num_cols,
    num_rows,
)


def test_matrix_2x2() -> None:
    M = matrix([[IRInteger(1), IRInteger(2)], [IRInteger(3), IRInteger(4)]])
    assert is_matrix(M)
    assert M.head == MATRIX
    assert len(M.args) == 2  # two rows


def test_matrix_rejects_jagged() -> None:
    with pytest.raises(MatrixError):
        matrix([[IRInteger(1), IRInteger(2)], [IRInteger(3)]])


def test_matrix_rejects_empty() -> None:
    with pytest.raises(MatrixError):
        matrix([])


def test_dimensions() -> None:
    M = matrix([[IRInteger(1), IRInteger(2), IRInteger(3)]])
    dims = dimensions(M)
    assert dims == IRApply(LIST, (IRInteger(1), IRInteger(3)))


def test_num_rows_cols() -> None:
    M = matrix(
        [
            [IRInteger(1), IRInteger(2), IRInteger(3)],
            [IRInteger(4), IRInteger(5), IRInteger(6)],
        ]
    )
    assert num_rows(M) == 2
    assert num_cols(M) == 3


def test_get_entry_one_based() -> None:
    M = matrix([[IRSymbol("a"), IRSymbol("b")], [IRSymbol("c"), IRSymbol("d")]])
    assert get_entry(M, 1, 1) == IRSymbol("a")
    assert get_entry(M, 2, 2) == IRSymbol("d")


def test_get_entry_out_of_range() -> None:
    M = matrix([[IRInteger(1)]])
    with pytest.raises(MatrixError):
        get_entry(M, 2, 1)
