"""Unit tests for filter design helpers."""

from __future__ import annotations

import math
import random

import pytest

from dsp_filters import (
    MAX_TAPS,
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


def test_public_fir_design_domain_edges_and_validation_order() -> None:
    for window in ("rectangular", "hamming", "hann", "blackman"):
        assert len(design_low_pass(-0.0, 1, window)) == 1
        assert len(design_high_pass(0.5, 1, window)) == 1
        assert len(design_band_pass(-0.0, 0.5, 1, window)) == 1

    # Python's documented integer domain includes the maximum odd count.
    assert len(design_low_pass(0.25, MAX_TAPS - 1, "rectangular")) == MAX_TAPS - 1
    for design_call in (
        lambda: design_low_pass(0.25, MAX_TAPS + 1, "rectangular"),
        lambda: design_high_pass(0.25, MAX_TAPS + 1, "rectangular"),
        lambda: design_band_pass(0.1, 0.2, MAX_TAPS + 1, "rectangular"),
    ):
        with pytest.raises(InvalidCoefficientError, match="MAX_TAPS"):
            design_call()

    # Tap errors precede cutoff errors; BP ordering is checked after
    # finite cutoffs. The lane exposes its existing typed exception.
    with pytest.raises(InvalidCoefficientError, match="num_taps"):
        design_low_pass(math.nan, 0, "rectangular")
    with pytest.raises(InvalidCoefficientError, match="cutoff_norm"):
        design_low_pass(math.inf, 3, "rectangular")
    with pytest.raises(
        InvalidCoefficientError, match="cutoff frequencies must be finite"
    ):
        design_band_pass(0.1, math.inf, 3, "rectangular")
    with pytest.raises(InvalidCoefficientError, match="cutoffs must satisfy"):
        design_band_pass(0.2, 0.2, 3, "rectangular")


def test_seeded_fir_designer_valid_domain_samples() -> None:
    windows = ("rectangular", "hamming", "hann", "blackman")
    calls = 0
    for seed in (0xA0D10, 0x51BADA55):
        rng = random.Random(seed)
        for _ in range(64):
            taps = 1 + 2 * rng.randrange(16)
            cutoff = rng.random() * 0.5
            window = windows[rng.randrange(len(windows))]
            assert len(design_low_pass(cutoff, taps, window)) == taps
            assert len(design_high_pass(cutoff, taps, window)) == taps
            high = 0.01 + cutoff * 0.98
            assert len(design_band_pass(high * 0.2, high, taps, window)) == taps
            calls += 3
    assert calls == 384
