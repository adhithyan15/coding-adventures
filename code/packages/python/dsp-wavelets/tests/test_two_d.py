"""Unit tests for 2-D Discrete Wavelet Transform (dwt_2d, idwt_2d)."""

from __future__ import annotations

import math

from dsp_wavelets.boundary import Periodic, Symmetric
from dsp_wavelets.dwt import dwt_2d, idwt_2d


def test_dwt_2d_haar_4x4() -> None:
    rows, cols = 4, 4
    image = [float(r * cols + c) for r in range(rows) for c in range(cols)]

    coeffs = dwt_2d(image, rows, cols, wavelet="haar", levels=1, boundary=Periodic)
    assert len(coeffs) == rows * cols

    reconstructed = idwt_2d(
        coeffs, rows, cols, wavelet="haar", levels=1, boundary=Periodic
    )
    assert len(reconstructed) == len(image)
    for orig, rec in zip(image, reconstructed, strict=True):
        assert math.isclose(orig, rec, abs_tol=1e-5)


def test_dwt_2d_multilevel_8x8() -> None:
    rows, cols = 8, 8
    image = [
        math.sin(r * 0.5) * math.cos(c * 0.5)
        for r in range(rows)
        for c in range(cols)
    ]

    coeffs = dwt_2d(image, rows, cols, wavelet="db2", levels=2, boundary=Periodic)
    assert len(coeffs) == rows * cols

    reconstructed = idwt_2d(
        coeffs, rows, cols, wavelet="db2", levels=2, boundary=Periodic
    )
    assert len(reconstructed) == len(image)
    for orig, rec in zip(image, reconstructed, strict=True):
        assert math.isclose(orig, rec, abs_tol=1e-4)


def test_dwt_2d_nonsquare() -> None:
    rows, cols = 4, 8
    image = [float(r * 10 + c) for r in range(rows) for c in range(cols)]

    coeffs = dwt_2d(
        image, rows, cols, wavelet="haar", levels=1, boundary=Symmetric
    )
    reconstructed = idwt_2d(
        coeffs, rows, cols, wavelet="haar", levels=1, boundary=Symmetric
    )
    assert len(reconstructed) == len(image)
    for orig, rec in zip(image, reconstructed, strict=True):
        assert math.isclose(orig, rec, abs_tol=1e-5)
