"""Unit tests for Stationary Wavelet Transform (swt, iswt)."""

from __future__ import annotations

import math

import pytest

from dsp_wavelets.boundary import Periodic
from dsp_wavelets.errors import InvalidCoefficientsError
from dsp_wavelets.swt import iswt, swt


def test_swt_shape() -> None:
    sig = [float(i) for i in range(16)]
    coeffs = swt(sig, wavelet="haar", level=2, boundary=Periodic)
    assert len(coeffs) == 2
    # Each level has (cA, cD) of length len(sig)
    for ca, cd in coeffs:
        assert len(ca) == 16
        assert len(cd) == 16


def test_swt_iswt_haar_roundtrip() -> None:
    sig = [1.0, 2.0, 4.0, 7.0, 11.0, 16.0, 22.0, 29.0]
    coeffs = swt(sig, wavelet="haar", level=1, boundary=Periodic)
    rec = iswt(coeffs, wavelet="haar", boundary=Periodic)
    assert len(rec) == len(sig)
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-5)


def test_swt_iswt_multilevel_db2() -> None:
    sig = [math.cos(i * 0.5) for i in range(16)]
    coeffs = swt(sig, wavelet="db2", level=2, boundary=Periodic)
    rec = iswt(coeffs, wavelet="db2", boundary=Periodic)
    assert len(rec) == len(sig)
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-4)


def test_iswt_mismatched_subbands() -> None:
    # Subband lengths mismatch
    with pytest.raises(InvalidCoefficientsError):
        iswt([([1.0, 2.0], [1.0])], wavelet="haar")
