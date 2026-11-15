"""Hostile input and security invariant tests for dsp_filters.

Covers:
- CWE-400: MAX_SAMPLES, MAX_TAPS, and MAX_ORDER pre-flight bounds.
- CWE-20: Non-finite floats (NaN, +Inf, -Inf) fail-closed.
- CWE-674: Zero recursion.
- CWE-1333: Zero regular expression imports.
- Empty signals, empty kernels, zero a[0], even tap counts.
"""

from __future__ import annotations

import ast
from pathlib import Path

import pytest

from dsp_filters import (
    MAX_ORDER,
    MAX_SAMPLES,
    MAX_TAPS,
    Biquad,
    EmptyKernelError,
    EmptySignalError,
    InvalidCoefficientError,
    butterworth_lowpass,
    chebyshev1_lowpass,
    design_band_pass,
    design_low_pass,
    fir,
    fir_fft,
    iir,
    sos_cascade,
)


def test_empty_inputs_rejected() -> None:
    with pytest.raises(EmptySignalError):
        fir([], [1.0])
    with pytest.raises(EmptyKernelError):
        fir([1.0], [])
    with pytest.raises(EmptySignalError):
        fir_fft([], [1.0])
    with pytest.raises(EmptyKernelError):
        fir_fft([1.0], [])
    with pytest.raises(EmptySignalError):
        iir([], [1.0], [1.0])
    with pytest.raises(EmptyKernelError):
        iir([1.0], [], [1.0])
    with pytest.raises(EmptyKernelError):
        iir([1.0], [1.0], [])
    with pytest.raises(EmptySignalError):
        Biquad.lowpass(0.1).process([])
    with pytest.raises(EmptySignalError):
        sos_cascade([], [[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]])


def test_cwe20_nan_and_inf_fail_closed() -> None:
    for bad in [float("nan"), float("inf"), float("-inf")]:
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            fir([1.0, bad], [0.5])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            fir([1.0], [0.5, bad])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            fir_fft([1.0, bad], [0.5])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            fir_fft([1.0], [0.5, bad])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            iir([1.0, bad], [1.0], [1.0])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            iir([1.0], [1.0, bad], [1.0])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            iir([1.0], [1.0], [1.0, bad])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            Biquad.lowpass(0.1).process_sample(bad)
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            Biquad.lowpass(0.1).process([1.0, bad])
        with pytest.raises(InvalidCoefficientError, match="non-finite"):
            sos_cascade([1.0, bad], [[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]])


def test_zero_a0_rejected_in_iir_and_biquad() -> None:
    with pytest.raises(InvalidCoefficientError, match="a\\[0\\] must be non-zero"):
        iir([1.0, 2.0], [1.0], [0.0, 0.5])
    with pytest.raises(InvalidCoefficientError, match="a0 must be non-zero"):
        Biquad(1.0, 0.0, 0.0, 0.0, 0.0, 0.0)


def test_invalid_design_parameters() -> None:
    # Even tap count rejected
    with pytest.raises(InvalidCoefficientError, match="must be odd"):
        design_low_pass(0.2, 32)
    with pytest.raises(InvalidCoefficientError, match="must be odd"):
        design_band_pass(0.1, 0.2, 32)

    # Cutoff out of range
    with pytest.raises(InvalidCoefficientError, match="cutoff_norm"):
        design_low_pass(-0.1, 33)
    with pytest.raises(InvalidCoefficientError, match="cutoff_norm"):
        design_low_pass(0.6, 33)
    with pytest.raises(InvalidCoefficientError, match="cutoffs must satisfy"):
        design_band_pass(0.3, 0.2, 33)

    # Unsupported orders
    with pytest.raises(InvalidCoefficientError, match="supports orders 1 and 2"):
        butterworth_lowpass(3, 0.2)
    with pytest.raises(InvalidCoefficientError, match="supports orders 1 and 2"):
        chebyshev1_lowpass(3, 1.0, 0.2)


def test_cwe400_bounds_rejection() -> None:
    class FakeHugeSequence(list):
        def __init__(self, size: int) -> None:
            super().__init__()
            self._size = size

        def __len__(self) -> int:
            return self._size

    huge_samples = FakeHugeSequence(MAX_SAMPLES + 1)
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_SAMPLES"):
        fir(huge_samples, [1.0])
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_SAMPLES"):
        fir_fft(huge_samples, [1.0])
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_SAMPLES"):
        iir(huge_samples, [1.0], [1.0])
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_SAMPLES"):
        Biquad.lowpass(0.1).process(huge_samples)
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_SAMPLES"):
        sos_cascade(huge_samples, [[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]])

    huge_taps = FakeHugeSequence(MAX_TAPS + 1)
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_TAPS"):
        fir([1.0], huge_taps)
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_TAPS"):
        fir_fft([1.0], huge_taps)
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_TAPS"):
        iir([1.0], huge_taps, [1.0])
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_TAPS"):
        design_low_pass(0.2, MAX_TAPS + 1)

    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_ORDER"):
        iir([1.0], [1.0] * (MAX_ORDER + 2), [1.0])
    with pytest.raises(InvalidCoefficientError, match="exceeds MAX_ORDER"):
        sos_cascade([1.0], [[1.0, 0.0, 0.0, 1.0, 0.0, 0.0]] * (MAX_ORDER + 1))


def test_cwe1333_zero_regex_imports() -> None:
    pkg_src = Path(__file__).resolve().parents[1] / "src" / "dsp_filters"
    for py_file in pkg_src.rglob("*.py"):
        tree = ast.parse(py_file.read_text(encoding="utf-8"))
        for node in ast.walk(tree):
            if isinstance(node, ast.Import):
                for alias in node.names:
                    assert alias.name != "re", f"re imported in {py_file}"
            elif isinstance(node, ast.ImportFrom):
                assert node.module != "re", f"re imported in {py_file}"
