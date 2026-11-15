"""Unit tests for wavelet definitions, families, and boundary modes."""

from __future__ import annotations

import math

import pytest

from dsp_wavelets.boundary import (
    Periodic,
    Reflect,
    Replicate,
    Symmetric,
    WaveletBoundary,
    Zero,
    parse_wavelet_boundary,
    sample_with_boundary,
)
from dsp_wavelets.errors import InvalidParamError
from dsp_wavelets.wavelet import (
    Wavelet,
    WaveletFamily,
    parse_wavelet,
)


def test_wavelet_haar() -> None:
    w = Wavelet.haar()
    assert w.name == "haar"
    assert w.family == WaveletFamily.HAAR
    assert w.filter_length == 2
    assert math.isclose(sum(tap * tap for tap in w.dec_lo), 1.0, rel_tol=1e-6)
    assert math.isclose(sum(tap * tap for tap in w.dec_hi), 1.0, rel_tol=1e-6)


def test_wavelet_daubechies() -> None:
    for order, expected_len in [(1, 2), (2, 4), (4, 8), (6, 12), (8, 16)]:
        w = Wavelet.db(order)
        assert w.filter_length == expected_len
        # Check energy normalization: sum(h^2) == 1.0
        energy_lo = sum(x * x for x in w.dec_lo)
        energy_hi = sum(x * x for x in w.dec_hi)
        assert math.isclose(energy_lo, 1.0, rel_tol=1e-4)
        assert math.isclose(energy_hi, 1.0, rel_tol=1e-4)
        # Check orthogonality: dot(h, g) == 0.0
        dot = sum(h * g for h, g in zip(w.dec_lo, w.dec_hi, strict=True))
        assert abs(dot) < 1e-6

    with pytest.raises(InvalidParamError):
        Wavelet.db(3)


def test_wavelet_symlet() -> None:
    w2 = Wavelet.sym(2)
    assert w2.filter_length == 4
    w4 = Wavelet.sym(4)
    assert w4.name == "sym4"
    assert w4.family == WaveletFamily.SYMLETS
    assert w4.filter_length == 8

    with pytest.raises(InvalidParamError):
        Wavelet.sym(5)


def test_wavelet_coiflet() -> None:
    w = Wavelet.coif(1)
    assert w.name == "coif1"
    assert w.family == WaveletFamily.COIFLETS
    assert w.filter_length == 6

    with pytest.raises(InvalidParamError):
        Wavelet.coif(2)


def test_wavelet_biorthogonal() -> None:
    w = Wavelet.bior(2, 2)
    assert w.name == "bior2.2"
    assert w.family == WaveletFamily.BIORTHOGONAL
    assert len(w.dec_lo) == 5
    assert len(w.rec_lo) == 3

    w_alias = Wavelet.bior(5, 3)
    assert w_alias.name == "bior2.2"

    with pytest.raises(InvalidParamError):
        Wavelet.bior(9, 7)


def test_continuous_wavelets() -> None:
    wm = Wavelet.morlet()
    assert wm.family == WaveletFamily.MORLET
    wr = Wavelet.ricker()
    assert wr.family == WaveletFamily.RICKER
    wmh = Wavelet.mexican_hat()
    assert wmh.family == WaveletFamily.RICKER


def test_parse_wavelet() -> None:
    assert parse_wavelet("haar").family == WaveletFamily.HAAR
    assert parse_wavelet("HAAR").family == WaveletFamily.HAAR
    assert parse_wavelet("db4").filter_length == 8
    assert parse_wavelet("DB2").filter_length == 4
    assert parse_wavelet("sym4").name == "sym4"
    assert parse_wavelet("coif1").name == "coif1"
    assert parse_wavelet("bior2.2").name == "bior2.2"
    assert parse_wavelet("bior5.3").name == "bior2.2"
    assert parse_wavelet("morlet").name == "morlet"
    assert parse_wavelet("ricker").name == "ricker"
    assert parse_wavelet("mexican_hat").name == "ricker"

    w = Wavelet.haar()
    assert parse_wavelet(w) is w

    with pytest.raises(InvalidParamError):
        parse_wavelet("unknown_wavelet")


def test_boundary_sampling() -> None:
    sig = [10.0, 20.0, 30.0]

    # Interior
    assert sample_with_boundary(sig, 0, Zero) == 10.0
    assert sample_with_boundary(sig, 1, Zero) == 20.0
    assert sample_with_boundary(sig, 2, Zero) == 30.0

    # Zero
    assert sample_with_boundary(sig, -1, Zero) == 0.0
    assert sample_with_boundary(sig, 3, Zero) == 0.0

    # Replicate
    assert sample_with_boundary(sig, -2, Replicate) == 10.0
    assert sample_with_boundary(sig, 5, Replicate) == 30.0

    # Periodic
    assert sample_with_boundary(sig, 3, Periodic) == 10.0
    assert sample_with_boundary(sig, 4, Periodic) == 20.0
    assert sample_with_boundary(sig, -1, Periodic) == 30.0

    # Reflect (period = 2*(n-1) = 4)
    # indices: 0->10, 1->20, 2->30, 3->20, 4->10
    assert sample_with_boundary(sig, 3, Reflect) == 20.0
    assert sample_with_boundary(sig, 4, Reflect) == 10.0
    assert sample_with_boundary(sig, -1, Reflect) == 20.0

    # Symmetric (period = 2*n = 6)
    # 0->10, 1->20, 2->30, 3->30, 4->20, 5->10
    assert sample_with_boundary(sig, 3, Symmetric) == 30.0
    assert sample_with_boundary(sig, 4, Symmetric) == 20.0
    assert sample_with_boundary(sig, 5, Symmetric) == 10.0

    # Empty signal
    assert sample_with_boundary([], 0, Zero) == 0.0

    # String boundary
    assert parse_wavelet_boundary("periodic") == WaveletBoundary.PERIODIC
    assert parse_wavelet_boundary(WaveletBoundary.ZERO) == WaveletBoundary.ZERO
    with pytest.raises(InvalidParamError):
        parse_wavelet_boundary("invalid_mode")
