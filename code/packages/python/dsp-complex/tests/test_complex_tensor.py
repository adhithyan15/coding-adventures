"""Unit tests for dsp_complex.ComplexTensor."""

from __future__ import annotations

import math

import pytest

from dsp_complex import ComplexTensor


def test_empty_tensor() -> None:
    t = ComplexTensor()
    assert len(t) == 0
    assert t.is_empty()
    assert t.as_interleaved() == []
    assert t.into_interleaved() == []
    assert t.real() == []
    assert t.imag() == []
    assert t.magnitude() == []
    assert t.phase() == []
    assert t.shape == (0, 2)
    assert t.matrix_shape() == (0, 2)
    assert t.matrix_dtype() == "F32"
    assert t.to_complex_list() == []


def test_from_real() -> None:
    real_data = [1.0, 2.5, -3.0]
    t = ComplexTensor.from_real(real_data)
    assert len(t) == 3
    assert not t.is_empty()
    assert t.real() == [1.0, 2.5, -3.0]
    assert t.imag() == [0.0, 0.0, 0.0]
    assert t.as_interleaved() == [1.0, 0.0, 2.5, 0.0, -3.0, 0.0]
    assert t.shape == (3, 2)
    assert t[0] == (1.0, 0.0)
    assert t[1] == (2.5, 0.0)
    assert t[-1] == (-3.0, 0.0)


def test_from_real_imag() -> None:
    r = [1.0, 2.0, 3.0]
    i = [4.0, 5.0, 6.0]
    t = ComplexTensor.from_real_imag(r, i)
    assert len(t) == 3
    assert t.real() == r
    assert t.imag() == i
    assert t.as_interleaved() == [1.0, 4.0, 2.0, 5.0, 3.0, 6.0]


def test_from_complex_list() -> None:
    c_list = [1.0 + 2.0j, 3.0 - 4.0j]
    t = ComplexTensor.from_complex_list(c_list)
    assert len(t) == 2
    assert t.to_complex_list() == c_list
    assert t.real() == [1.0, 3.0]
    assert t.imag() == [2.0, -4.0]


def test_magnitude_and_phase() -> None:
    # 3 + 4j -> mag 5.0, phase atan2(4, 3)
    # 1 + 1j -> mag sqrt(2), phase pi/4
    t = ComplexTensor.from_real_imag([3.0, 1.0], [4.0, 1.0])
    mag = t.magnitude()
    phase = t.phase()
    assert math.isclose(mag[0], 5.0, rel_tol=1e-6)
    assert math.isclose(mag[1], math.sqrt(2.0), rel_tol=1e-6)
    assert math.isclose(phase[0], math.atan2(4.0, 3.0), rel_tol=1e-6)
    assert math.isclose(phase[1], math.pi / 4.0, rel_tol=1e-6)


def test_conjugate() -> None:
    t = ComplexTensor.from_real_imag([1.0, 2.0], [3.0, -4.0])
    conj = t.conjugate()
    assert conj.real() == [1.0, 2.0]
    assert conj.imag() == [-3.0, 4.0]
    # Double conjugate is identity
    assert conj.conjugate() == t


def test_indexing_and_iteration() -> None:
    t = ComplexTensor.from_real_imag([1.0, 2.0], [3.0, 4.0])
    items = list(t)
    assert items == [(1.0, 3.0), (2.0, 4.0)]
    assert t[0] == (1.0, 3.0)
    assert t[1] == (2.0, 4.0)
    with pytest.raises(IndexError):
        _ = t[2]
    with pytest.raises(IndexError):
        _ = t[-3]


def test_equality_and_repr() -> None:
    t1 = ComplexTensor.from_real([1.0, 2.0])
    t2 = ComplexTensor.from_real([1.0, 2.0])
    t3 = ComplexTensor.from_real([1.0, 3.0])
    assert t1 == t2
    assert t1 != t3
    assert t1 != "not_a_tensor"
    rep = repr(t1)
    assert "ComplexTensor(len=2" in rep
