"""Comprehensive tests for CpuBlas — the reference implementation.

This is THE reference test suite. Every BLAS operation is tested with
known inputs and pre-computed expected results. Other backends are
tested against these same expected values.

Organization:
    - Level 1: SAXPY, DOT, NRM2, SCAL, ASUM, IAMAX, COPY, SWAP
    - Level 2: GEMV (both transposes), GER
    - Level 3: GEMM (4 transpose combos), SYMM, Batched GEMM
    - Edge cases: alpha=0, beta=0, beta=1, 1x1 matrices, identity
    - Errors: dimension mismatches
"""

from __future__ import annotations

import pytest
from conftest import approx_equal, approx_list

from blas_library import CpuBlas, Matrix, Side, Transpose, Vector


@pytest.fixture
def blas() -> CpuBlas:
    return CpuBlas()


# =========================================================================
# Properties
# =========================================================================


class TestCpuProperties:
    def test_name(self, blas: CpuBlas) -> None:
        assert blas.name == "cpu"

    def test_device_name(self, blas: CpuBlas) -> None:
        assert "CPU" in blas.device_name


# =========================================================================
# Level 1: Vector-Vector operations
# =========================================================================


class TestLevel1Saxpy:
    """SAXPY: y = alpha * x + y"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        y = Vector(data=[4.0, 5.0, 6.0], size=3)
        result = blas.saxpy(2.0, x, y)
        assert approx_list(result.data, [6.0, 9.0, 12.0])

    def test_alpha_zero(self, blas: CpuBlas) -> None:
        """alpha=0 means result = y (x contribution zeroed out)."""
        x = Vector(data=[100.0, 200.0], size=2)
        y = Vector(data=[1.0, 2.0], size=2)
        result = blas.saxpy(0.0, x, y)
        assert approx_list(result.data, [1.0, 2.0])

    def test_alpha_one(self, blas: CpuBlas) -> None:
        """alpha=1 means result = x + y."""
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0], size=2)
        result = blas.saxpy(1.0, x, y)
        assert approx_list(result.data, [4.0, 6.0])

    def test_negative_alpha(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0], size=2)
        result = blas.saxpy(-1.0, x, y)
        assert approx_list(result.data, [2.0, 2.0])

    def test_single_element(self, blas: CpuBlas) -> None:
        x = Vector(data=[5.0], size=1)
        y = Vector(data=[3.0], size=1)
        result = blas.saxpy(2.0, x, y)
        assert approx_list(result.data, [13.0])

    def test_dimension_mismatch(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[1.0, 2.0, 3.0], size=3)
        with pytest.raises(ValueError, match="dimension mismatch"):
            blas.saxpy(1.0, x, y)


class TestLevel1Dot:
    """DOT: result = sum(x[i] * y[i])"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        y = Vector(data=[4.0, 5.0, 6.0], size=3)
        assert approx_equal(blas.sdot(x, y), 32.0)  # 1*4 + 2*5 + 3*6

    def test_orthogonal(self, blas: CpuBlas) -> None:
        """Perpendicular vectors have dot product = 0."""
        x = Vector(data=[1.0, 0.0], size=2)
        y = Vector(data=[0.0, 1.0], size=2)
        assert approx_equal(blas.sdot(x, y), 0.0)

    def test_parallel(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[2.0, 4.0], size=2)
        assert approx_equal(blas.sdot(x, y), 10.0)

    def test_single_element(self, blas: CpuBlas) -> None:
        x = Vector(data=[3.0], size=1)
        y = Vector(data=[4.0], size=1)
        assert approx_equal(blas.sdot(x, y), 12.0)

    def test_dimension_mismatch(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0], size=1)
        y = Vector(data=[1.0, 2.0], size=2)
        with pytest.raises(ValueError):
            blas.sdot(x, y)


class TestLevel1Nrm2:
    """NRM2: result = sqrt(sum(x[i]^2))"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[3.0, 4.0], size=2)
        assert approx_equal(blas.snrm2(x), 5.0)  # 3-4-5 triangle

    def test_unit_vector(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 0.0, 0.0], size=3)
        assert approx_equal(blas.snrm2(x), 1.0)

    def test_zero_vector(self, blas: CpuBlas) -> None:
        x = Vector(data=[0.0, 0.0], size=2)
        assert approx_equal(blas.snrm2(x), 0.0)

    def test_negative_values(self, blas: CpuBlas) -> None:
        x = Vector(data=[-3.0, 4.0], size=2)
        assert approx_equal(blas.snrm2(x), 5.0)


class TestLevel1Scal:
    """SCAL: result = alpha * x"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        result = blas.sscal(2.0, x)
        assert approx_list(result.data, [2.0, 4.0, 6.0])

    def test_zero_alpha(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        result = blas.sscal(0.0, x)
        assert approx_list(result.data, [0.0, 0.0])

    def test_negative_alpha(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, -2.0], size=2)
        result = blas.sscal(-1.0, x)
        assert approx_list(result.data, [-1.0, 2.0])


class TestLevel1Asum:
    """ASUM: result = sum(|x[i]|)"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, -2.0, 3.0, -4.0], size=4)
        assert approx_equal(blas.sasum(x), 10.0)

    def test_all_positive(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        assert approx_equal(blas.sasum(x), 6.0)

    def test_zero_vector(self, blas: CpuBlas) -> None:
        x = Vector(data=[0.0, 0.0], size=2)
        assert approx_equal(blas.sasum(x), 0.0)


class TestLevel1Isamax:
    """ISAMAX: argmax(|x[i]|)"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, -5.0, 3.0], size=3)
        assert blas.isamax(x) == 1  # |-5| = 5 is largest

    def test_first_element(self, blas: CpuBlas) -> None:
        x = Vector(data=[10.0, 1.0, 2.0], size=3)
        assert blas.isamax(x) == 0

    def test_last_element(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0, 10.0], size=3)
        assert blas.isamax(x) == 2

    def test_single_element(self, blas: CpuBlas) -> None:
        x = Vector(data=[42.0], size=1)
        assert blas.isamax(x) == 0

    def test_empty_vector(self, blas: CpuBlas) -> None:
        x = Vector(data=[], size=0)
        assert blas.isamax(x) == 0


class TestLevel1Copy:
    """COPY: result = x (deep copy)"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        result = blas.scopy(x)
        assert result.data == x.data
        assert result.size == x.size

    def test_independent_copy(self, blas: CpuBlas) -> None:
        """Modifying the copy should not affect the original."""
        x = Vector(data=[1.0, 2.0], size=2)
        result = blas.scopy(x)
        result.data[0] = 999.0
        assert x.data[0] == 1.0


class TestLevel1Swap:
    """SWAP: x <-> y"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0], size=2)
        new_x, new_y = blas.sswap(x, y)
        assert new_x.data == [3.0, 4.0]
        assert new_y.data == [1.0, 2.0]

    def test_dimension_mismatch(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0], size=1)
        y = Vector(data=[1.0, 2.0], size=2)
        with pytest.raises(ValueError):
            blas.sswap(x, y)


# =========================================================================
# Level 2: Matrix-Vector operations
# =========================================================================


class TestLevel2Gemv:
    """GEMV: y = alpha * op(A) * x + beta * y"""

    def test_no_trans(self, blas: CpuBlas) -> None:
        """A * x with no transpose: (2x3) * (3,) = (2,)"""
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        x = Vector(data=[1.0, 1.0, 1.0], size=3)
        y = Vector(data=[0.0, 0.0], size=2)
        result = blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y)
        assert approx_list(result.data, [6.0, 15.0])

    def test_trans(self, blas: CpuBlas) -> None:
        """A^T * x with transpose: (3x2) after transpose * (2,) = (3,)"""
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        x = Vector(data=[1.0, 1.0], size=2)
        y = Vector(data=[0.0, 0.0, 0.0], size=3)
        result = blas.sgemv(Transpose.TRANS, 1.0, a, x, 0.0, y)
        assert approx_list(result.data, [5.0, 7.0, 9.0])

    def test_alpha_beta(self, blas: CpuBlas) -> None:
        """Test with non-trivial alpha and beta."""
        a = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        x = Vector(data=[3.0, 4.0], size=2)
        y = Vector(data=[1.0, 2.0], size=2)
        result = blas.sgemv(Transpose.NO_TRANS, 2.0, a, x, 3.0, y)
        # 2.0 * I * [3,4] + 3.0 * [1,2] = [6,8] + [3,6] = [9,14]
        assert approx_list(result.data, [9.0, 14.0])

    def test_dimension_mismatch_x(self, blas: CpuBlas) -> None:
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        y = Vector(data=[0.0, 0.0], size=2)
        with pytest.raises(ValueError, match="dimension mismatch"):
            blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y)

    def test_dimension_mismatch_y(self, blas: CpuBlas) -> None:
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[0.0, 0.0, 0.0], size=3)
        with pytest.raises(ValueError, match="dimension mismatch"):
            blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y)


class TestLevel2Ger:
    """GER: A = alpha * x * y^T + A"""

    def test_basic(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0, 5.0], size=3)
        a = Matrix(data=[0.0] * 6, rows=2, cols=3)
        result = blas.sger(1.0, x, y, a)
        # x*y^T = [[3,4,5],[6,8,10]]
        assert approx_list(result.data, [3.0, 4.0, 5.0, 6.0, 8.0, 10.0])

    def test_with_existing_a(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 1.0], size=2)
        y = Vector(data=[1.0, 1.0], size=2)
        a = Matrix(data=[10.0, 20.0, 30.0, 40.0], rows=2, cols=2)
        result = blas.sger(1.0, x, y, a)
        assert approx_list(result.data, [11.0, 21.0, 31.0, 41.0])

    def test_alpha_scaling(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0], size=1)
        y = Vector(data=[1.0], size=1)
        a = Matrix(data=[0.0], rows=1, cols=1)
        result = blas.sger(5.0, x, y, a)
        assert approx_list(result.data, [5.0])

    def test_dimension_mismatch(self, blas: CpuBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0], size=1)
        a = Matrix(data=[0.0] * 4, rows=2, cols=2)
        with pytest.raises(ValueError):
            blas.sger(1.0, x, y, a)


# =========================================================================
# Level 3: Matrix-Matrix operations
# =========================================================================


class TestLevel3Gemm:
    """GEMM: C = alpha * op(A) * op(B) + beta * C"""

    def test_basic_no_trans(self, blas: CpuBlas) -> None:
        """Standard matrix multiply: (2x3) * (3x2) = (2x2)"""
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        b = Matrix(data=[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], rows=3, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [58.0, 64.0, 139.0, 154.0])

    def test_trans_a(self, blas: CpuBlas) -> None:
        """A^T * B: A is (3x2), A^T is (2x3), B is (2x2)... wait.
        Let A be (3x2), so A^T is (2x3). B is (3x2). Result is (2x2)."""
        a = Matrix(data=[1.0, 4.0, 2.0, 5.0, 3.0, 6.0], rows=3, cols=2)
        b = Matrix(data=[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], rows=3, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(Transpose.TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)
        # A^T = [[1,2,3],[4,5,6]], B = [[7,8],[9,10],[11,12]]
        # A^T * B = [[58,64],[139,154]]
        assert approx_list(result.data, [58.0, 64.0, 139.0, 154.0])

    def test_trans_b(self, blas: CpuBlas) -> None:
        """A * B^T: A is (2x3), B is (2x3), B^T is (3x2). Result is (2x2)."""
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        b = Matrix(data=[7.0, 9.0, 11.0, 8.0, 10.0, 12.0], rows=2, cols=3)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.TRANS, 1.0, a, b, 0.0, c)
        # B^T = [[7,8],[9,10],[11,12]]. A*B^T = [[58,64],[139,154]]
        assert approx_list(result.data, [58.0, 64.0, 139.0, 154.0])

    def test_trans_both(self, blas: CpuBlas) -> None:
        """A^T * B^T"""
        a = Matrix(data=[1.0, 4.0, 2.0, 5.0, 3.0, 6.0], rows=3, cols=2)
        b = Matrix(data=[7.0, 9.0, 11.0, 8.0, 10.0, 12.0], rows=2, cols=3)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(Transpose.TRANS, Transpose.TRANS, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [58.0, 64.0, 139.0, 154.0])

    def test_alpha_beta(self, blas: CpuBlas) -> None:
        """C = 2*A*B + 3*C"""
        a = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c = Matrix(data=[1.0, 1.0, 1.0, 1.0], rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 2.0, a, b, 3.0, c)
        # 2*I*B + 3*C = 2*B + 3*[1,1,1,1] = [10,12,14,16] + [3,3,3,3] = [13,15,17,19]
        assert approx_list(result.data, [13.0, 15.0, 17.0, 19.0])

    def test_alpha_zero(self, blas: CpuBlas) -> None:
        """alpha=0 means result = beta * C."""
        a = Matrix(data=[99.0] * 4, rows=2, cols=2)
        b = Matrix(data=[99.0] * 4, rows=2, cols=2)
        c = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 0.0, a, b, 1.0, c)
        assert approx_list(result.data, [1.0, 2.0, 3.0, 4.0])

    def test_beta_zero(self, blas: CpuBlas) -> None:
        """beta=0 means C is ignored (zeroed out)."""
        a = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c = Matrix(data=[999.0] * 4, rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [5.0, 6.0, 7.0, 8.0])

    def test_1x1_matrices(self, blas: CpuBlas) -> None:
        """Scalar-like 1x1 GEMM."""
        a = Matrix(data=[3.0], rows=1, cols=1)
        b = Matrix(data=[4.0], rows=1, cols=1)
        c = Matrix(data=[0.0], rows=1, cols=1)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [12.0])

    def test_identity_multiply(self, blas: CpuBlas) -> None:
        """I * A = A."""
        identity = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        a = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(
            Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, identity, a, 0.0, c
        )
        assert approx_list(result.data, [5.0, 6.0, 7.0, 8.0])

    def test_dimension_mismatch_inner(self, blas: CpuBlas) -> None:
        """Inner dimensions must match."""
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=3, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        with pytest.raises(ValueError, match="dimension mismatch"):
            blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)

    def test_dimension_mismatch_c(self, blas: CpuBlas) -> None:
        """C shape must match result shape."""
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 6, rows=2, cols=3)
        with pytest.raises(ValueError, match="dimension mismatch"):
            blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)


class TestLevel3Symm:
    """SYMM: C = alpha * A * B + beta * C (A symmetric)"""

    def test_left_side(self, blas: CpuBlas) -> None:
        """LEFT: C = alpha * A * B + beta * C"""
        a = Matrix(data=[1.0, 2.0, 2.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.ssymm(Side.LEFT, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [1.0, 2.0, 2.0, 1.0])

    def test_right_side(self, blas: CpuBlas) -> None:
        """RIGHT: C = alpha * B * A + beta * C"""
        a = Matrix(data=[1.0, 2.0, 2.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.ssymm(Side.RIGHT, 1.0, a, b, 0.0, c)
        # B * A = I * A = A
        assert approx_list(result.data, [1.0, 2.0, 2.0, 1.0])

    def test_non_square_a_raises(self, blas: CpuBlas) -> None:
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        b = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        c = Matrix(data=[0.0] * 6, rows=2, cols=3)
        with pytest.raises(ValueError, match="square"):
            blas.ssymm(Side.LEFT, 1.0, a, b, 0.0, c)

    def test_with_beta(self, blas: CpuBlas) -> None:
        a = Matrix(data=[2.0, 0.0, 0.0, 2.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 1.0, 1.0, 1.0], rows=2, cols=2)
        c = Matrix(data=[1.0, 1.0, 1.0, 1.0], rows=2, cols=2)
        result = blas.ssymm(Side.LEFT, 1.0, a, b, 1.0, c)
        # A*B = [[2,2],[2,2]] + C = [[3,3],[3,3]]
        assert approx_list(result.data, [3.0, 3.0, 3.0, 3.0])


class TestLevel3BatchedGemm:
    """Batched GEMM: multiple independent GEMMs."""

    def test_basic(self, blas: CpuBlas) -> None:
        a1 = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        b1 = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c1 = Matrix(data=[0.0] * 4, rows=2, cols=2)
        a2 = Matrix(data=[2.0, 0.0, 0.0, 2.0], rows=2, cols=2)
        b2 = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        c2 = Matrix(data=[0.0] * 4, rows=2, cols=2)

        results = blas.sgemm_batched(
            Transpose.NO_TRANS,
            Transpose.NO_TRANS,
            1.0,
            [a1, a2],
            [b1, b2],
            0.0,
            [c1, c2],
        )
        assert len(results) == 2
        assert approx_list(results[0].data, [5.0, 6.0, 7.0, 8.0])
        assert approx_list(results[1].data, [2.0, 4.0, 6.0, 8.0])

    def test_batch_size_mismatch(self, blas: CpuBlas) -> None:
        a = [Matrix(data=[1.0], rows=1, cols=1)]
        b = [Matrix(data=[1.0], rows=1, cols=1), Matrix(data=[1.0], rows=1, cols=1)]
        c = [Matrix(data=[0.0], rows=1, cols=1)]
        with pytest.raises(ValueError, match="batch sizes"):
            blas.sgemm_batched(
                Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c
            )

    def test_empty_batch(self, blas: CpuBlas) -> None:
        results = blas.sgemm_batched(
            Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, [], [], 0.0, []
        )
        assert results == []
