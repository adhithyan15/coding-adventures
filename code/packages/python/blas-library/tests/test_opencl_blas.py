"""Tests for OpenClBlas — portable OpenCL backend."""

from __future__ import annotations

import pytest
from conftest import approx_equal, approx_list

from blas_library import Matrix, OpenClBlas, Side, Transpose, Vector


@pytest.fixture
def blas() -> OpenClBlas:
    return OpenClBlas()


class TestOpenClProperties:
    def test_name(self, blas: OpenClBlas) -> None:
        assert blas.name == "opencl"

    def test_device_name(self, blas: OpenClBlas) -> None:
        assert isinstance(blas.device_name, str)


class TestOpenClLevel1:
    def test_saxpy(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        y = Vector(data=[4.0, 5.0, 6.0], size=3)
        result = blas.saxpy(2.0, x, y)
        assert approx_list(result.data, [6.0, 9.0, 12.0])

    def test_sdot(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        y = Vector(data=[4.0, 5.0, 6.0], size=3)
        assert approx_equal(blas.sdot(x, y), 32.0)

    def test_snrm2(self, blas: OpenClBlas) -> None:
        x = Vector(data=[3.0, 4.0], size=2)
        assert approx_equal(blas.snrm2(x), 5.0)

    def test_sscal(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        result = blas.sscal(2.0, x)
        assert approx_list(result.data, [2.0, 4.0, 6.0])

    def test_sasum(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, -2.0, 3.0], size=3)
        assert approx_equal(blas.sasum(x), 6.0)

    def test_isamax(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, -5.0, 3.0], size=3)
        assert blas.isamax(x) == 1

    def test_scopy(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, 2.0, 3.0], size=3)
        result = blas.scopy(x)
        assert approx_list(result.data, [1.0, 2.0, 3.0])

    def test_sswap(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0], size=2)
        new_x, new_y = blas.sswap(x, y)
        assert approx_list(new_x.data, [3.0, 4.0])
        assert approx_list(new_y.data, [1.0, 2.0])


class TestOpenClLevel2:
    def test_sgemv(self, blas: OpenClBlas) -> None:
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        x = Vector(data=[1.0, 1.0, 1.0], size=3)
        y = Vector(data=[0.0, 0.0], size=2)
        result = blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y)
        assert approx_list(result.data, [6.0, 15.0])

    def test_sger(self, blas: OpenClBlas) -> None:
        x = Vector(data=[1.0, 2.0], size=2)
        y = Vector(data=[3.0, 4.0, 5.0], size=3)
        a = Matrix(data=[0.0] * 6, rows=2, cols=3)
        result = blas.sger(1.0, x, y, a)
        assert approx_list(result.data, [3.0, 4.0, 5.0, 6.0, 8.0, 10.0])


class TestOpenClLevel3:
    def test_sgemm(self, blas: OpenClBlas) -> None:
        a = Matrix(data=[1.0, 2.0, 3.0, 4.0, 5.0, 6.0], rows=2, cols=3)
        b = Matrix(data=[7.0, 8.0, 9.0, 10.0, 11.0, 12.0], rows=3, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [58.0, 64.0, 139.0, 154.0])

    def test_ssymm(self, blas: OpenClBlas) -> None:
        a = Matrix(data=[1.0, 2.0, 2.0, 1.0], rows=2, cols=2)
        b = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.ssymm(Side.LEFT, 1.0, a, b, 0.0, c)
        assert approx_list(result.data, [1.0, 2.0, 2.0, 1.0])

    def test_sgemm_batched(self, blas: OpenClBlas) -> None:
        a1 = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        b1 = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c1 = Matrix(data=[0.0] * 4, rows=2, cols=2)
        results = blas.sgemm_batched(
            Transpose.NO_TRANS,
            Transpose.NO_TRANS,
            1.0,
            [a1],
            [b1],
            0.0,
            [c1],
        )
        assert approx_list(results[0].data, [5.0, 6.0, 7.0, 8.0])

    def test_identity_multiply(self, blas: OpenClBlas) -> None:
        identity = Matrix(data=[1.0, 0.0, 0.0, 1.0], rows=2, cols=2)
        a = Matrix(data=[5.0, 6.0, 7.0, 8.0], rows=2, cols=2)
        c = Matrix(data=[0.0] * 4, rows=2, cols=2)
        result = blas.sgemm(
            Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, identity, a, 0.0, c
        )
        assert approx_list(result.data, [5.0, 6.0, 7.0, 8.0])

    def test_alpha_zero(self, blas: OpenClBlas) -> None:
        a = Matrix(data=[99.0] * 4, rows=2, cols=2)
        b = Matrix(data=[99.0] * 4, rows=2, cols=2)
        c = Matrix(data=[1.0, 2.0, 3.0, 4.0], rows=2, cols=2)
        result = blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 0.0, a, b, 1.0, c)
        assert approx_list(result.data, [1.0, 2.0, 3.0, 4.0])
