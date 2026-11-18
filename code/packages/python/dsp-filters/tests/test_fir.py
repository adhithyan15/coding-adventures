"""Unit tests for FIR filtering (direct and FFT-based)."""

from __future__ import annotations

import math

from dsp_filters import fir, fir_fft


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_fir_impulse() -> None:
    signal = [1.0, 0.0, 0.0, 0.0]
    kernel = [0.2, 0.5, 0.3]
    out = fir(signal, kernel)
    # Output length should be 4 + 3 - 1 = 6
    assert len(out) == 6
    _assert_close(out, [0.2, 0.5, 0.3, 0.0, 0.0, 0.0])


def test_fir_single_sample_convolution_is_not_a_round_trip() -> None:
    assert fir([2.0], [2.0]) == [4.0]
    assert fir([1.0, -2.0, 3.0], [1.0]) == [1.0, -2.0, 3.0]


def test_fir_fft_matches_direct_fir() -> None:
    for n, k in [(4, 3), (8, 5), (16, 7), (32, 15), (64, 33)]:
        sig = [math.sin(0.3 * i) for i in range(n)]
        ker = [math.cos(0.5 * j) for j in range(k)]
        direct_out = fir(sig, ker)
        fft_out = fir_fft(sig, ker)
        _assert_close(direct_out, fft_out, tol=1e-4)
