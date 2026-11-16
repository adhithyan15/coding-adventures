"""Hostile input and security invariant tests for dsp_fft.

Covers:
- CWE-400: MAX_FFT_LEN and MAX_SAMPLES pre-flight bounds.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- CWE-674: Zero recursion.
- CWE-1333: Zero regex imports.
- Odd interleaved lengths, non-pow2 inputs, mismatched half-spectra.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest
from dsp_complex import ComplexTensor

from dsp_fft import (
    MAX_FFT_LEN,
    MAX_SAMPLES,
    Direction,
    InvalidInputError,
    NotPowerOfTwoError,
    bluestein_scalar,
    fft,
    fft_scalar,
    ifft,
    ifft_scalar,
    irfft_scalar,
    rfft_scalar,
)


def test_empty_inputs_rejected() -> None:
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        fft_scalar([])
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        ifft_scalar([])
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        bluestein_scalar([], Direction.FORWARD)
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        rfft_scalar([])
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        irfft_scalar([], 0)
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        fft([])
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        fft([], complex=True)
    with pytest.raises(InvalidInputError, match="must be at least 1"):
        ifft(ComplexTensor())


def test_odd_interleaved_lengths_rejected() -> None:
    odd_buf = [1.0, 2.0, 3.0]
    with pytest.raises(InvalidInputError, match="even length"):
        fft_scalar(odd_buf)
    with pytest.raises(InvalidInputError, match="even length"):
        ifft_scalar(odd_buf)
    with pytest.raises(InvalidInputError, match="even length"):
        bluestein_scalar(odd_buf, Direction.FORWARD)
    with pytest.raises(InvalidInputError, match="even length"):
        fft(odd_buf, complex=True)
    with pytest.raises(InvalidInputError, match="even length"):
        irfft_scalar(odd_buf, 2)


def test_not_power_of_two_rejected_in_radix2() -> None:
    # 6 floats = 3 complex bins, not power of two
    buf = [1.0, 0.0] * 3
    with pytest.raises(NotPowerOfTwoError):
        fft_scalar(buf)
    with pytest.raises(NotPowerOfTwoError):
        ifft_scalar(buf)


def test_cwe20_nan_and_inf_fail_closed() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        buf_pow2 = [1.0, 0.0, bad, 0.0]
        buf_non_pow2 = [1.0, 0.0, bad, 0.0, 2.0, 0.0]

        with pytest.raises(InvalidInputError, match="non-finite"):
            fft_scalar(buf_pow2)
        with pytest.raises(InvalidInputError, match="non-finite"):
            ifft_scalar(buf_pow2)
        with pytest.raises(InvalidInputError, match="non-finite"):
            bluestein_scalar(buf_non_pow2, Direction.FORWARD)
        with pytest.raises(InvalidInputError, match="non-finite"):
            rfft_scalar([1.0, bad, 2.0])
        with pytest.raises(InvalidInputError, match="non-finite"):
            irfft_scalar([1.0, 0.0, bad, 0.0], 2)
        with pytest.raises(InvalidInputError, match="non-finite"):
            fft([1.0, bad, 2.0])
        with pytest.raises(InvalidInputError, match="non-finite"):
            fft(buf_pow2, complex=True)


def test_mismatched_half_spectrum_bins() -> None:
    # 4 floats = 2 complex bins. For N = 8, expected floor(8/2)+1 = 5 bins.
    half = [1.0, 0.0, 2.0, 0.0]
    with pytest.raises(InvalidInputError, match="expected 5 for output_length = 8"):
        irfft_scalar(half, 8)


def test_cwe400_bounds_rejection() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    # Exceeding MAX_SAMPLES
    huge = FakeHugeSequence(MAX_SAMPLES + 2)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        fft_scalar(huge)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        ifft_scalar(huge)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        bluestein_scalar(huge, Direction.FORWARD)
    with pytest.raises(InvalidInputError, match="exceeds (MAX_FFT_LEN|MAX_SAMPLES)"):
        rfft_scalar(huge)
    with pytest.raises(InvalidInputError, match="exceeds MAX_SAMPLES"):
        irfft_scalar(huge, 100)

    # Exceeding MAX_FFT_LEN in irfft output length
    with pytest.raises(InvalidInputError, match="exceeds MAX_FFT_LEN"):
        irfft_scalar([0.0, 0.0], MAX_FFT_LEN + 1)

    with pytest.raises(InvalidInputError, match="exceeds maximum allowed"):
        fft(FakeHugeSequence(MAX_FFT_LEN + 1))
    with pytest.raises(InvalidInputError, match="exceeds maximum allowed"):
        fft(FakeHugeSequence(2 * (MAX_FFT_LEN + 1)), complex=True)


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_fft"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
