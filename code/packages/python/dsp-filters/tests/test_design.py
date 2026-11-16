"""Unit tests for filter design helpers."""

from __future__ import annotations

import math

import pytest

from dsp_filters import (
    InvalidCoefficientError,
    WindowType,
    butterworth_lowpass,
    chebyshev1_lowpass,
    design_band_pass,
    design_high_pass,
    design_low_pass,
    iir,
)


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_design_low_pass_dc_gain_is_one() -> None:
    for cutoff in [0.05, 0.1, 0.25, 0.4]:
        for win in [
            WindowType.RECTANGULAR,
            WindowType.HAMMING,
            WindowType.HANN,
            WindowType.BLACKMAN,
            "hamming",
            "hann",
            "blackman",
            "rectangular",
        ]:
            h = design_low_pass(cutoff, 33, win)
            assert len(h) == 33
            s = sum(h)
            assert math.isclose(s, 1.0, rel_tol=1e-5)


def test_design_high_pass_symmetry() -> None:
    hp = design_high_pass(0.2, 31, WindowType.HAMMING)
    assert len(hp) == 31
    for k in range(15):
        assert math.isclose(hp[k], hp[30 - k], rel_tol=1e-5)


def test_design_band_pass_all_windows() -> None:
    for win in [
        WindowType.RECTANGULAR,
        WindowType.HAMMING,
        WindowType.HANN,
        WindowType.BLACKMAN,
    ]:
        bp = design_band_pass(0.1, 0.3, 33, win)
        assert len(bp) == 33
        for k in range(16):
            assert math.isclose(bp[k], bp[32 - k], rel_tol=1e-5)


def test_butterworth_lowpass() -> None:
    for order in [1, 2]:
        b, a = butterworth_lowpass(order, 0.2)
        assert len(b) == order + 1
        assert len(a) == order + 1
        assert a[0] == 1.0

        dc_input = [1.0] * 50
        out = iir(dc_input, b, a)
        assert math.isclose(out[-1], 1.0, rel_tol=1e-3)


def test_chebyshev1_lowpass() -> None:
    for order in [1, 2]:
        b, a = chebyshev1_lowpass(order, 1.0, 0.2)
        assert len(b) == order + 1
        assert len(a) == order + 1
        assert a[0] == 1.0
        dc_input = [1.0] * 50
        out = iir(dc_input, b, a)
        assert math.isclose(out[-1], 1.0, rel_tol=0.15)


def test_unknown_window_type() -> None:
    with pytest.raises(InvalidCoefficientError, match="unknown window type"):
        design_low_pass(0.1, 15, "unknown_window")
