"""Unit tests for 1-D Discrete Wavelet Transform (dwt, idwt, dwt_1d, idwt_1d)."""

from __future__ import annotations

import math

import pytest

from dsp_wavelets.boundary import Periodic, Reflect, Zero
from dsp_wavelets.dwt import (
    dwt,
    dwt_1d,
    forward_level_lengths,
    idwt,
    idwt_1d,
)
from dsp_wavelets.errors import InvalidParamError
from dsp_wavelets.wavelet import Wavelet


def test_forward_level_lengths() -> None:
    lengths = forward_level_lengths(16, 3)
    assert lengths == [16, 8, 4, 2]

    odd_lens = forward_level_lengths(15, 3)
    assert odd_lens == [15, 8, 4, 2]


def test_dwt_haar_roundtrip() -> None:
    sig = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0]
    ca, cd = dwt(sig, wavelet="haar", boundary=Periodic)
    assert len(ca) == 4
    assert len(cd) == 4

    rec = idwt(ca, cd, wavelet="haar", boundary=Periodic, target_len=len(sig))
    assert len(rec) == len(sig)
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-5)


def test_dwt_db4_roundtrip() -> None:
    sig = [math.sin(2.0 * math.pi * i / 32.0) for i in range(32)]
    ca, cd = dwt(sig, wavelet="db4", boundary=Periodic)
    rec = idwt(ca, cd, wavelet="db4", boundary=Periodic, target_len=32)
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-4)


def test_dwt_odd_length() -> None:
    sig = [1.0, 3.0, -2.0, 5.0, 4.0]
    ca, cd = dwt(sig, wavelet="haar", boundary=Reflect)
    assert len(ca) == 3
    assert len(cd) == 3
    rec = idwt(ca, cd, wavelet="haar", boundary=Reflect, target_len=5)
    assert len(rec) == 5
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-5)


def test_dwt_1d_multilevel() -> None:
    sig = [float(i) for i in range(16)]
    # Level 2 decomposition: [cA_2 (4) | cD_2 (4) | cD_1 (8)] -> total 16
    coeffs = dwt_1d(sig, wavelet="haar", levels=2, boundary=Periodic)
    assert len(coeffs) == 16

    rec = idwt_1d(
        coeffs,
        wavelet="haar",
        levels=2,
        boundary=Periodic,
        output_length=16,
    )
    assert len(rec) == 16
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-5)


def test_dwt_1d_sym4() -> None:
    sig = [math.cos(i * 0.3) for i in range(32)]
    coeffs = dwt_1d(sig, wavelet="sym4", levels=2, boundary=Periodic)
    rec = idwt_1d(
        coeffs,
        wavelet="sym4",
        levels=2,
        boundary=Periodic,
        output_length=32,
    )
    assert len(rec) == 32
    for orig, r in zip(sig, rec, strict=True):
        assert math.isclose(orig, r, abs_tol=1e-4)


def test_dwt_biorthogonal() -> None:
    w = Wavelet.bior(2, 2)
    sig = [1.0, 4.0, 2.0, 8.0, 5.0, 7.0, 3.0, 6.0]
    with pytest.raises(InvalidParamError, match="orthogonal"):
        dwt(sig, wavelet=w, boundary=Zero)
    with pytest.raises(InvalidParamError, match="orthogonal"):
        idwt([1.0, 2.0], [3.0, 4.0], wavelet=w, boundary=Zero)
