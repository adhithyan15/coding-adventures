"""Tests for row_reduce and rank — Group E (cas-matrix 0.2.0)."""

from __future__ import annotations

import pytest
from symbolic_ir import IRInteger, IRRational

from cas_matrix import (
    MatrixError,
    identity_matrix,
    matrix,
    rank,
    row_reduce,
    zero_matrix,
)

# ---------------------------------------------------------------------------
# Helpers
# ---------------------------------------------------------------------------

def _i(n: int):
    return IRInteger(n)


def _r(p: int, q: int):
    return IRRational(p, q)


def _mat(*rows):
    """Convenience: _mat([1,2],[3,4]) builds a Matrix from ints."""
    return matrix([[_i(v) for v in row] for row in rows])


def _entries(M):
    """Extract (numer, denom) pairs from a Matrix for easy comparison."""
    from cas_matrix.matrix import _rows_of
    result = []
    for row in _rows_of(M):
        r = []
        for e in row:
            if isinstance(e, IRInteger):
                r.append((e.value, 1))
            elif isinstance(e, IRRational):
                r.append((e.numer, e.denom))
            else:
                raise ValueError(f"Unexpected entry type: {type(e)}")
        result.append(r)
    return result


# ---------------------------------------------------------------------------
# rank — basic cases
# ---------------------------------------------------------------------------

class TestRank:
    def test_identity_2x2(self):
        assert rank(identity_matrix(2)) == _i(2)

    def test_identity_3x3(self):
        assert rank(identity_matrix(3)) == _i(3)

    def test_zero_matrix_rank_0(self):
        assert rank(zero_matrix(3, 3)) == _i(0)

    def test_zero_matrix_1x1(self):
        assert rank(zero_matrix(1, 1)) == _i(0)

    def test_rank_2_singular_3x3(self):
        # rows [1,2,3],[4,5,6],[7,8,9] — rows are linearly dependent
        M = _mat([1, 2, 3], [4, 5, 6], [7, 8, 9])
        assert rank(M) == _i(2)

    def test_full_rank_2x2(self):
        M = _mat([1, 2], [3, 4])
        assert rank(M) == _i(2)

    def test_rank_1_repeated_row(self):
        M = _mat([1, 2, 3], [2, 4, 6], [3, 6, 9])
        assert rank(M) == _i(1)

    def test_rank_wide_matrix(self):
        # 2×4 matrix, rank 2
        M = _mat([1, 0, 2, 1], [0, 1, 3, -1])
        assert rank(M) == _i(2)

    def test_rank_tall_matrix(self):
        # 4×2, rank 2
        M = _mat([1, 0], [0, 1], [1, 1], [2, 3])
        assert rank(M) == _i(2)

    def test_rank_1x1_nonzero(self):
        M = _mat([5])
        assert rank(M) == _i(1)

    def test_rank_1x1_zero(self):
        M = _mat([0])
        assert rank(M) == _i(0)

    def test_rank_with_rational_entries(self):
        # Row 1 = 2 × row 0 (scaled by 1/2), so rank 1
        M = matrix([[_i(1), _r(1, 2)], [_i(2), _i(1)]])
        assert rank(M) == _i(1)

    def test_rank_symbolic_raises(self):
        from symbolic_ir import IRSymbol
        M = matrix([[IRSymbol("a"), _i(1)], [_i(0), _i(1)]])
        with pytest.raises(MatrixError):
            rank(M)


# ---------------------------------------------------------------------------
# row_reduce — basic cases
# ---------------------------------------------------------------------------

class TestRowReduce:
    def test_identity_2x2_unchanged(self):
        Id = identity_matrix(2)
        R = row_reduce(Id)
        assert _entries(R) == [[(1, 1), (0, 1)], [(0, 1), (1, 1)]]

    def test_2x2_full_rank(self):
        M = _mat([2, 4], [1, 3])
        R = row_reduce(M)
        # RREF of [[2,4],[1,3]] = [[1,0],[0,1]]
        assert _entries(R) == [[(1, 1), (0, 1)], [(0, 1), (1, 1)]]

    def test_3x3_singular(self):
        # [[1,2,3],[4,5,6],[7,8,9]] — rank 2
        M = _mat([1, 2, 3], [4, 5, 6], [7, 8, 9])
        R = row_reduce(M)
        entries = _entries(R)
        # First pivot at col 0: row 0 has (1, ..., -1)
        # Second pivot at col 1: row 1 has (0, 1, 2)
        # Third row is all zeros
        assert entries[0][0] == (1, 1)
        assert entries[1][1] == (1, 1)
        assert all(e == (0, 1) for e in entries[2])

    def test_3x4_wide_matrix(self):
        # Full-rank 3×4
        M = _mat([1, 0, 2, -1], [0, 1, 3, 4], [0, 0, 1, -2])
        R = row_reduce(M)
        entries = _entries(R)
        # pivot columns should be 0, 1, 2
        assert entries[0][0] == (1, 1)
        assert entries[1][1] == (1, 1)
        assert entries[2][2] == (1, 1)
        # All entries in pivot columns above/below pivot should be 0
        assert entries[1][0] == (0, 1)  # col 0, row 1
        assert entries[2][0] == (0, 1)  # col 0, row 2
        assert entries[0][1] == (0, 1)  # col 1, row 0
        assert entries[2][1] == (0, 1)  # col 1, row 2

    def test_zero_matrix_rref(self):
        Z = zero_matrix(3, 3)
        R = row_reduce(Z)
        entries = _entries(R)
        assert all(e == (0, 1) for row in entries for e in row)

    def test_1x1_nonzero(self):
        M = _mat([7])
        R = row_reduce(M)
        assert _entries(R) == [[(1, 1)]]

    def test_1x1_zero(self):
        M = _mat([0])
        R = row_reduce(M)
        assert _entries(R) == [[(0, 1)]]

    def test_rational_entries(self):
        # [[1/2, 1], [1, 2]] — row 1 = 2 × row 0, so rank 1
        M = matrix([[_r(1, 2), _i(1)], [_i(1), _i(2)]])
        R = row_reduce(M)
        entries = _entries(R)
        assert entries[0][0] == (1, 1)
        assert entries[1] == [(0, 1), (0, 1)]

    def test_symbolic_raises(self):
        from symbolic_ir import IRSymbol
        M = matrix([[IRSymbol("a"), _i(1)], [_i(0), _i(1)]])
        with pytest.raises(MatrixError):
            row_reduce(M)

    def test_consistency_with_rank(self):
        """RREF of M should have exactly rank(M) non-zero rows."""
        M = _mat([1, 2, 3], [4, 5, 6], [7, 8, 9])
        R = row_reduce(M)
        r = rank(M)
        from cas_matrix.matrix import _rows_of
        rows = _rows_of(R)
        non_zero_rows = sum(
            1 for row in rows
            if any(isinstance(e, IRInteger) and e.value != 0 for e in row)
        )
        assert non_zero_rows == r.value
