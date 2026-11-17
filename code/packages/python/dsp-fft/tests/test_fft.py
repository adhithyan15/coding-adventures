"""Unit tests for radix-2 Cooley-Tukey FFT and high-level ComplexTensor dispatch."""

from __future__ import annotations

import math

from dsp_complex import ComplexTensor

from dsp_fft import fft, fft_scalar, ifft, ifft_scalar


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_impulse_spectrum() -> None:
    signal = [1.0, 0.0] + [0.0, 0.0] * 7
    spec = fft_scalar(signal)
    expected = [1.0, 0.0] * 8
    _assert_close(spec, expected)


def test_dc_spectrum() -> None:
    signal = [1.0, 0.0] * 8
    spec = fft_scalar(signal)
    expected = [8.0, 0.0] + [0.0, 0.0] * 7
    _assert_close(spec, expected)


def test_radix2_round_trip() -> None:
    for n in [1, 2, 4, 8, 16, 32, 64, 128]:
        signal = [math.sin(0.3 * i) for i in range(2 * n)]
        spec = fft_scalar(signal)
        recovered = ifft_scalar(spec)
        _assert_close(signal, recovered, tol=1e-5)


def test_high_level_complex_tensor_fft_ifft() -> None:
    real_sig = [1.0, 2.0, 3.0, 4.0]
    tensor_spec = fft(real_sig, complex=False)
    assert isinstance(tensor_spec, ComplexTensor)
    assert len(tensor_spec) == 4

    recovered = ifft(tensor_spec)
    assert isinstance(recovered, ComplexTensor)
    _assert_close(recovered.real(), real_sig, tol=1e-5)
    _assert_close(recovered.imag(), [0.0] * 4, tol=1e-5)


def test_high_level_complex_input_fft() -> None:
    complex_data = [1.0, 0.5, 2.0, -0.5, 3.0, 1.0, 4.0, -1.0]
    tensor_spec = fft(complex_data, complex=True)
    recovered = ifft(tensor_spec)
    _assert_close(recovered.as_interleaved(), complex_data, tol=1e-5)


def test_high_level_non_pow2_fft_ifft() -> None:
    # Non-power-of-two real signal
    real_sig = [1.0, 2.0, 3.0, 4.0, 5.0]
    tensor_spec = fft(real_sig, complex=False)
    assert len(tensor_spec) == 5
    recovered = ifft(tensor_spec)
    _assert_close(recovered.real(), real_sig, tol=1e-4)

    # Non-power-of-two complex signal
    complex_sig = [1.0, 0.5, 2.0, -0.2, 3.0, 0.1]  # 3 complex elements
    c_spec = fft(complex_sig, complex=True)
    assert len(c_spec) == 3
    c_rec = ifft(c_spec)
    _assert_close(c_rec.as_interleaved(), complex_sig, tol=1e-4)


def test_single_element_fft() -> None:
    single = [4.2]
    spec = fft(single)
    assert len(spec) == 1
    assert math.isclose(spec.real()[0], 4.2)
    rec = ifft(spec)
    assert math.isclose(rec.real()[0], 4.2)
