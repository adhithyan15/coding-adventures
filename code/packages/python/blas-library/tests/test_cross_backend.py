"""Cross-backend equivalence tests — same computation through all 7 backends.

=== The Capstone Test ===

This is the most important test file in the BLAS library. It runs the
same BLAS operations through ALL 7 backends and verifies they produce
identical results (within floating-point tolerance).

If any backend disagrees with the others, the bug is in that backend.
The CPU backend is the reference — all GPU backends must match it.
"""

from __future__ import annotations

import pytest
from conftest import approx_equal, approx_list

from blas_library import (
    Matrix,
    Side,
    Transpose,
    Vector,
    create_blas,
)

# All 7 backend names
ALL_BACKENDS = ["cpu", "cuda", "opencl", "metal", "vulkan", "webgpu", "opengl"]


@pytest.fixture(params=ALL_BACKENDS)
def backend_name(request: pytest.FixtureRequest) -> str:
    """Parametrize over all 7 backend names."""
    return request.param


@pytest.fixture
def blas(backend_name: str) -> object:
    """Create a BLAS instance for the given backend."""
    return create_blas(backend_name)


# =========================================================================
# Cross-backend Level 1 tests
# =========================================================================


class TestCrossBackendLevel1:
    """All backends must agree on Level 1 operations."""

    def test_saxpy(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, 2.0, 3.0, 4.0], size=4)
        y = Vector(data=[5.0, 6.0, 7.0, 8.0], size=4)
        result = blas.saxpy(2.0, x, y)  # type: ignore[union-attr]
        expected = [7.0, 10.0, 13.0, 16.0]
        assert approx_list(result.data, expected), f"{backend_name} SAXPY failed"

    def test_sdot(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        y = Vector(data=[4.0, 5.0, 6.0], size=3)
        result = blas.sdot(x, y)  # type: ignore[union-attr]
        assert approx_equal(result, 32.0), f"{backend_name} DOT failed"

    def test_snrm2(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[3.0, 4.0], size=2)
        result = blas.snrm2(x)  # type: ignore[union-attr]
        assert approx_equal(result, 5.0), f"{backend_name} NRM2 failed"

    def test_sscal(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        result = blas.sscal(3.0, x)  # type: ignore[union-attr]
        assert approx_list(result.data, [3.0, 6.0, 9.0]), f"{backend_name} SCAL failed"

    def test_sasum(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, -2.0, 3.0, -4.0], size=4)
        result = blas.sasum(x)  # type: ignore[union-attr]
        assert approx_equal(result, 10.0), f"{backend_name} ASUM failed"

    def test_isamax(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, -5.0, 3.0], size=3)
        result = blas.isamax(x)  # type: ignore[union-attr]
        assert result == 1, f"{backend_name} IAMAX failed"

    def test_scopy(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        result = blas.scopy(x)  # type: ignore[union-attr]
        assert approx_list(result.data, [1.0, 2.0, 3.0]), f"{backend_name} COPY failed"

    def test_sswap(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0], size=2)
        new_x, new_y = blas.sswap(x, y)  # type: ignore[union-attr]
        assert approx_list(new_x.data, [3.0, 4.0]), f"{backend_name} SWAP-x failed"
        assert approx_list(new_y.data, [1.0, 2.0]), f"{backend_name} SWAP-y failed"


# =========================================================================
# Cross-backend Level 2 tests
# =========================================================================


class TestCrossBackendLevel2:
    """All backends must agree on Level 2 operations."""

    def test_sgemv_no_trans(self, blas: object, backend_name: str) -> None:
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        x = Vector(data=[1.0, 1.0, 1.0], size=3)
        y = Vector(data=[0.0, 0.0], size=2)
        result = blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y)  # type: ignore[union-attr]
        assert approx_list(result.data, [6.0, 15.0]), f"{backend_name} GEMV failed"

    def test_sger(self, blas: object, backend_name: str) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0], size=2)
        a = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sger(1.0, x, y, a)  # type: ignore[union-attr]
        expected = [3.0, 4.0, 6.0, 8.0]
        assert approx_list(result.data, expected), f"{backend_name} GER failed"


# =========================================================================
# Cross-backend Level 3 tests — THE BIG ONE
# =========================================================================


class TestCrossBackendLevel3:
    """All backends must agree on Level 3 operations."""

    def test_sgemm_canonical(self, blas: object, backend_name: str) -> None:
        """The canonical GEMM test from the spec:
        A = [[1,2,3],[4,5,6]]  B = [[7,8],[9,10],[11,12]]
        C = A*B = [[58,64],[139,154]]
        """
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        b = Matrix(data=[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], rows=3, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(  # type: ignore[union-attr]
            Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c
        )
        expected = [58.0, 64.0, 139.0, 154.0]
        assert approx_list(result.data, expected), (
            f"{backend_name} GEMM failed: got {result.data}"
        )

    def test_sgemm_with_alpha_beta(self, blas: object, backend_name: str) -> None:
        """GEMM with alpha=2, beta=3."""
        a = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c = Matrix(data=[1.0, 1.0, 1.0, 1.0], rows=2, cols=2)
        result = blas.sgemm(  # type: ignore[union-attr]
            Transpose.NO_TRANS, Transpose.NO_TRANS, 2.0, a, b, 3.0, c
        )
        expected = [13.0, 15.0, 17.0, 19.0]
        assert approx_list(result.data, expected), (
            f"{backend_name} GEMM alpha/beta failed"
        )

    def test_ssymm(self, blas: object, backend_name: str) -> None:
        a = Matrix(data=[1.0, 2.0, 2.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.ssymm(Side.LEFT, 1.0, a, b, 0.0, c)  # type: ignore[union-attr]
        assert approx_list(result.data, [1.0, 2.0, 2.0, 1.0]), (
            f"{backend_name} SYMM failed"
        )

    def test_sgemm_batched(self, blas: object, backend_name: str) -> None:
        a1 = Matrix(data=[2.0, 0.0, 0.0, 2.0], rows=2, cols=2)
        b1 = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        c1 = Matrix(data=[0.0] * 4, rows=2, cols=2)
        results = blas.sgemm_batched(  # type: ignore[union-attr]
            Transpose.NO_TRANS,
            Transpose.NO_TRANS,
            1.0,
            [a1],
            [b1],
            0.0,
            [c1],
        )
        assert approx_list(results[0].data, [2.0, 4.0, 6.0, 8.0]), (
            f"{backend_name} batched GEMM failed"
        )
