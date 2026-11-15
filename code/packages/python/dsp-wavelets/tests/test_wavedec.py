"""Unit tests for multi-level 1-D wavedec and waverec."""

from __future__ import annotations

import math

import pytest

from dsp_wavelets.boundary import Periodic
from dsp_wavelets.errors import InvalidCoefficientsError
from dsp_wavelets.wavedec import wavedec, waverec


def test_wavedec_structure() -> None:
    sig = [float(i) for i in range(16)]
    # 2 levels: returns [cA_2, cD_2, cD_1]
    decomp = wavedec(sig, wavelet="haar", boundary=Periodic, level=2)
    assert len(decomp) == 3
    assert len(decomp[0]) == 4  # cA_2
    assert len(decomp[1]) == 4  # cD_2
    assert len(decomp[2]) == 8  # cD_1


def test_wavedec_default_level() -> None:
    sig = [float(i) for i in range(32)]
    decomp = wavedec(sig, wavelet="haar", boundary=Periodic)
    # Default level should be log2(32) = 5 -> [cA_5, cD_5, cD_4, cD_3, cD_2, cD_1]
    assert len(decomp) == 6
    assert len(decomp[0]) == 1


def test_waverec_roundtrip() -> None:
    sig = [1.0, 3.0, 5.0, 7.0, 2.0, 4.0, 6.0, 8.0]
    decomp = wavedec(sig, wavelet="haar", boundary=Periodic, level=2)
    rec = waverec(decomp, wavelet="haar", boundary=Periodic)
    assert len(rec) == len(sig)
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-5)


def test_waverec_sym4() -> None:
    sig = [math.sin(i * 0.4) for i in range(16)]
    decomp = wavedec(sig, wavelet="sym4", boundary=Periodic, level=2)
    rec = waverec(decomp, wavelet="sym4", boundary=Periodic)
    assert len(rec) == len(sig)
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-4)


def test_waverec_invalid_coeffs() -> None:
    with pytest.raises(InvalidCoefficientsError):
        waverec([[1.0, 2.0]])  # Only 1 band, needs at least 2
