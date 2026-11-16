"""Unit tests for Bluestein arbitrary-length FFT."""

from __future__ import annotations

import math

from dsp_fft import Direction, bluestein_scalar, fft_scalar


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def _dft_naive(signal: list[float], direction: Direction) -> list[float]:
    """Naive O(N^2) reference DFT for exact parity checking."""
    n = len(signal) // 2
    sign = -1.0 if direction == Direction.FORWARD else 1.0
    out = [0.0] * (2 * n)
    for k in range(n):
        re_sum = 0.0
        im_sum = 0.0
        for j in range(n):
            theta = sign * 2.0 * math.pi * k * j / n
            cos_t = math.cos(theta)
            sin_t = math.sin(theta)
            xr = signal[2 * j]
            xi = signal[2 * j + 1]
            re_sum += xr * cos_t - xi * sin_t
            im_sum += xr * sin_t + xi * cos_t
        if direction == Direction.INVERSE:
            re_sum /= n
            im_sum /= n
        out[2 * k] = re_sum
        out[2 * k + 1] = im_sum
    return out


def test_bluestein_n1_identity() -> None:
    sig = [3.5, -2.1]
    forward = bluestein_scalar(sig, Direction.FORWARD)
    assert forward == sig
    inverse = bluestein_scalar(sig, Direction.INVERSE)
    assert inverse == sig


def test_bluestein_dft_naive_parity() -> None:
    for n in [3, 5, 7, 11, 13]:
        sig = [math.sin(0.7 * i) for i in range(2 * n)]
        spec_blue = bluestein_scalar(sig, Direction.FORWARD)
        spec_naive = _dft_naive(sig, Direction.FORWARD)
        _assert_close(spec_blue, spec_naive, tol=1e-4)


def test_bluestein_round_trip() -> None:
    for n in [3, 5, 6, 7, 9, 10, 15, 23, 50]:
        sig = [math.sin(0.5 * i) for i in range(2 * n)]
        spec = bluestein_scalar(sig, Direction.FORWARD)
        rec = bluestein_scalar(spec, Direction.INVERSE)
        _assert_close(sig, rec, tol=1e-4)


def test_bluestein_matches_radix2_on_powers_of_two() -> None:
    for n in [2, 4, 8, 16, 32]:
        sig = [math.cos(0.4 * i) for i in range(2 * n)]
        spec_r2 = fft_scalar(sig)
        spec_blue = bluestein_scalar(sig, Direction.FORWARD)
        _assert_close(spec_blue, spec_r2, tol=1e-4)
