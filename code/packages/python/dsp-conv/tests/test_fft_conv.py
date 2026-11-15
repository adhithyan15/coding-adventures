"""Unit tests for FFT-based convolution."""

from __future__ import annotations

import math

from dsp_conv import ConvMode, convolve, fft_convolve


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_fft_convolve_full() -> None:
    x = [math.sin(0.2 * i) for i in range(17)]
    h = [math.cos(0.5 * j) for j in range(7)]
    direct = convolve(x, h, mode=ConvMode.FULL)
    fft_res = fft_convolve(x, h, mode=ConvMode.FULL)
    _assert_close(direct, fft_res, tol=1e-4)


def test_fft_convolve_same() -> None:
    x = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0]
    h = [0.2, 0.5, 0.3]
    direct = convolve(x, h, mode=ConvMode.SAME)
    fft_res = fft_convolve(x, h, mode=ConvMode.SAME)
    _assert_close(direct, fft_res, tol=1e-4)


def test_fft_convolve_valid() -> None:
    x = [float(i) for i in range(12)]
    h = [1.0, -1.0, 0.5]
    direct = convolve(x, h, mode=ConvMode.VALID)
    fft_res = fft_convolve(x, h, mode=ConvMode.VALID)
    _assert_close(direct, fft_res, tol=1e-4)
