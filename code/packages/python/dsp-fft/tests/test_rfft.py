"""Unit tests for real-input FFT (rfft) and inverse (irfft)."""

from __future__ import annotations

import math

from dsp_complex import ComplexTensor

from dsp_fft import irfft, irfft_scalar, rfft, rfft_scalar


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_rfft_half_spectrum_sizes() -> None:
    # Even N = 8 -> 5 bins (10 interleaved floats)
    sig_even = [float(i) for i in range(8)]
    half_even = rfft_scalar(sig_even)
    assert len(half_even) == 10

    # Odd N = 7 -> 4 bins (8 interleaved floats)
    sig_odd = [float(i) for i in range(7)]
    half_odd = rfft_scalar(sig_odd)
    assert len(half_odd) == 8


def test_rfft_irfft_round_trip() -> None:
    # Test both even and odd lengths, powers of two and non-powers of two
    for n in [1, 2, 3, 4, 7, 8, 9, 15, 16, 23, 32, 63, 64]:
        sig = [math.sin(0.4 * i) + 0.5 * math.cos(0.9 * i) for i in range(n)]
        half_spec = rfft_scalar(sig)
        recovered = irfft_scalar(half_spec, n)
        _assert_close(sig, recovered, tol=1e-4)


def test_high_level_rfft_irfft_complex_tensor() -> None:
    sig = [1.0, 2.0, 3.0, 4.0, 5.0]
    tensor_spec = rfft(sig)
    assert isinstance(tensor_spec, ComplexTensor)
    assert len(tensor_spec) == 5 // 2 + 1  # 3 bins

    recovered = irfft(tensor_spec, 5)
    _assert_close(sig, recovered, tol=1e-4)
