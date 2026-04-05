import math
import pytest
from matrix import Matrix


# ======================================================================
# Existing base tests
# ======================================================================

def test_zeros():
    z = Matrix.zeros(2, 3)
    assert z.rows == 2
    assert z.cols == 3
    assert z.data[1][2] == 0.0

def test_add_subtract():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    B = Matrix([[5.0, 6.0], [7.0, 8.0]])
    assert (A + B).data == [[6.0, 8.0], [10.0, 12.0]]
    assert (B - A).data == [[4.0, 4.0], [4.0, 4.0]]

    # Test scalar broadcasting
    assert (A + 2.0).data == [[3.0, 4.0], [5.0, 6.0]]
    assert (A - 1.0).data == [[0.0, 1.0], [2.0, 3.0]]

    with pytest.raises(ValueError):
        A + Matrix([[1.0]])

def test_scale():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert (A * 2.0).data == [[2.0, 4.0], [6.0, 8.0]]

def test_transpose():
    A = Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]])
    assert A.transpose().data == [[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]
    assert Matrix([]).transpose().data == []

def test_dot():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    B = Matrix([[5.0, 6.0], [7.0, 8.0]])
    assert A.dot(B).data == [[19.0, 22.0], [43.0, 50.0]]

    C = Matrix([1.0, 2.0, 3.0])
    D = Matrix([[4.0], [5.0], [6.0]])
    assert C.dot(D).data == [[32.0]]

    with pytest.raises(ValueError):
        A.dot(D)


# ======================================================================
# Element access tests
# ======================================================================

def test_get():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.get(0, 0) == 1.0
    assert M.get(0, 1) == 2.0
    assert M.get(1, 0) == 3.0
    assert M.get(1, 1) == 4.0

def test_get_out_of_bounds():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    with pytest.raises(IndexError):
        M.get(2, 0)
    with pytest.raises(IndexError):
        M.get(0, 2)
    with pytest.raises(IndexError):
        M.get(-1, 0)

def test_set():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    N = M.set(0, 0, 99.0)
    # Original unchanged (immutability)
    assert M.get(0, 0) == 1.0
    assert N.get(0, 0) == 99.0
    assert N.get(1, 1) == 4.0

def test_set_out_of_bounds():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    with pytest.raises(IndexError):
        M.set(2, 0, 5.0)


# ======================================================================
# Reduction tests
# ======================================================================

def test_sum():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.sum() == 10.0

def test_sum_rows():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    result = M.sum_rows()
    assert result.data == [[3.0], [7.0]]
    assert result.rows == 2
    assert result.cols == 1

def test_sum_cols():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    result = M.sum_cols()
    assert result.data == [[4.0, 6.0]]
    assert result.rows == 1
    assert result.cols == 2

def test_mean():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.mean() == 2.5

def test_mean_empty():
    with pytest.raises(ValueError):
        Matrix([]).mean()

def test_min_max():
    M = Matrix([[3.0, 1.0], [4.0, 2.0]])
    assert M.min() == 1.0
    assert M.max() == 4.0

def test_min_max_negative():
    M = Matrix([[-5.0, 3.0], [0.0, -1.0]])
    assert M.min() == -5.0
    assert M.max() == 3.0

def test_argmin():
    M = Matrix([[3.0, 1.0], [4.0, 2.0]])
    assert M.argmin() == (0, 1)

def test_argmax():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.argmax() == (1, 1)

def test_argmax_first_occurrence():
    """When multiple elements share the max, the first one wins."""
    M = Matrix([[4.0, 2.0], [3.0, 4.0]])
    assert M.argmax() == (0, 0)


# ======================================================================
# Element-wise math tests
# ======================================================================

def test_map():
    M = Matrix([[1.0, 4.0], [9.0, 16.0]])
    result = M.map(math.sqrt)
    assert result.data == [[1.0, 2.0], [3.0, 4.0]]

def test_sqrt():
    M = Matrix([[1.0, 4.0], [9.0, 16.0]])
    assert M.sqrt().data == [[1.0, 2.0], [3.0, 4.0]]

def test_abs():
    M = Matrix([[-1.0, 2.0], [-3.0, 4.0]])
    assert M.abs().data == [[1.0, 2.0], [3.0, 4.0]]

def test_pow():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.pow(2).data == [[1.0, 4.0], [9.0, 16.0]]

def test_sqrt_pow_roundtrip():
    """M.close(M.sqrt().pow(2.0), 1e-9) should be true."""
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.close(M.sqrt().pow(2.0), 1e-9)


# ======================================================================
# Shape operation tests
# ======================================================================

def test_flatten():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    flat = M.flatten()
    assert flat.data == [[1.0, 2.0, 3.0, 4.0]]
    assert flat.rows == 1
    assert flat.cols == 4

def test_reshape():
    M = Matrix([[1.0, 2.0, 3.0, 4.0, 5.0, 6.0]])
    result = M.reshape(2, 3)
    assert result.data == [[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]

def test_reshape_invalid():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    with pytest.raises(ValueError):
        M.reshape(3, 3)

def test_flatten_reshape_roundtrip():
    """M.flatten().reshape(M.rows, M.cols) should equal M."""
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.flatten().reshape(M.rows, M.cols).equals(M)

def test_row():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.row(0).data == [[1.0, 2.0]]
    assert M.row(1).data == [[3.0, 4.0]]

def test_row_out_of_bounds():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    with pytest.raises(IndexError):
        M.row(2)

def test_col():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.col(0).data == [[1.0], [3.0]]
    assert M.col(1).data == [[2.0], [4.0]]

def test_col_out_of_bounds():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    with pytest.raises(IndexError):
        M.col(2)

def test_slice():
    M = Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0], [7.0, 8.0, 9.0]])
    result = M.slice(0, 2, 1, 3)
    assert result.data == [[2.0, 3.0], [5.0, 6.0]]

def test_slice_single_column():
    """slice(0,2,0,1) on [[1,2],[3,4]] -> [[1],[3]]"""
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    result = M.slice(0, 2, 0, 1)
    assert result.data == [[1.0], [3.0]]

def test_slice_out_of_bounds():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    with pytest.raises(IndexError):
        M.slice(0, 3, 0, 1)


# ======================================================================
# Equality tests
# ======================================================================

def test_equals():
    A = Matrix([[1.0, 2.0], [3.0, 4.0]])
    B = Matrix([[1.0, 2.0], [3.0, 4.0]])
    C = Matrix([[1.0, 2.0], [3.0, 5.0]])
    assert A.equals(B)
    assert not A.equals(C)

def test_equals_shape_mismatch():
    A = Matrix([[1.0, 2.0]])
    B = Matrix([[1.0], [2.0]])
    assert not A.equals(B)

def test_close():
    A = Matrix([[1.0, 2.0]])
    B = Matrix([[1.0 + 1e-10, 2.0 - 1e-10]])
    assert A.close(B, 1e-9)

def test_close_fails():
    A = Matrix([[1.0, 2.0]])
    B = Matrix([[1.1, 2.0]])
    assert not A.close(B, 1e-9)


# ======================================================================
# Factory method tests
# ======================================================================

def test_identity():
    I = Matrix.identity(3)
    assert I.rows == 3
    assert I.cols == 3
    assert I.data == [[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]]

def test_identity_dot():
    """identity(3).dot(M) == M for any 3xn M."""
    M = Matrix([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
    I = Matrix.identity(3)
    result = I.dot(M)
    assert result.equals(M)

def test_from_diagonal():
    D = Matrix.from_diagonal([2.0, 3.0])
    assert D.data == [[2.0, 0.0], [0.0, 3.0]]

def test_from_diagonal_single():
    D = Matrix.from_diagonal([5.0])
    assert D.data == [[5.0]]


# ======================================================================
# Parity test vectors (cross-language consistency)
# ======================================================================

def test_parity_sum_mean():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.sum() == 10.0
    assert M.mean() == 2.5

def test_parity_sum_rows_cols():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.sum_rows().data == [[3.0], [7.0]]
    assert M.sum_cols().data == [[4.0, 6.0]]

def test_parity_identity_dot():
    M = Matrix([[1.0, 2.0], [3.0, 4.0], [5.0, 6.0]])
    assert Matrix.identity(3).dot(M).equals(M)

def test_parity_flatten_reshape():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.flatten().reshape(M.rows, M.cols).equals(M)

def test_parity_close_sqrt_pow():
    M = Matrix([[1.0, 2.0], [3.0, 4.0]])
    assert M.close(M.sqrt().pow(2.0), 1e-9)
