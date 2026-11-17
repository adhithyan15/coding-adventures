"""Unit tests for window functions."""

from __future__ import annotations

import math

from dsp_stft import (
    WindowType,
    blackman_window,
    build_window,
    hamming_window,
    hann_window,
    rectangular_window,
)


def _assert_close(a: list[float], b: list[float], tol: float = 1e-5) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_rectangular_window() -> None:
    w = rectangular_window(8)
    assert len(w) == 8
    assert all(x == 1.0 for x in w)


def test_hann_window_properties() -> None:
    n = 65  # Odd length has exact center at (n-1)//2 = 32
    w = hann_window(n)
    assert len(w) == n
    # Hann window starts and ends at 0.0
    assert math.isclose(w[0], 0.0, abs_tol=1e-6)
    assert math.isclose(w[-1], 0.0, abs_tol=1e-6)
    # Peak at centre
    assert math.isclose(w[32], 1.0, rel_tol=1e-5)
    # Symmetric
    for i in range(n // 2):
        assert math.isclose(w[i], w[n - 1 - i], rel_tol=1e-5)


def test_hamming_window_properties() -> None:
    n = 65
    w = hamming_window(n)
    assert len(w) == n
    # Hamming window boundary is ~0.08
    assert math.isclose(w[0], 0.08, abs_tol=1e-3)
    assert math.isclose(w[-1], 0.08, abs_tol=1e-3)
    # Peak at centre
    assert math.isclose(w[32], 1.0, rel_tol=1e-3)
    # Symmetric
    for i in range(n // 2):
        assert math.isclose(w[i], w[n - 1 - i], rel_tol=1e-5)


def test_blackman_window_properties() -> None:
    n = 65
    w = blackman_window(n)
    assert len(w) == n
    # Blackman boundary is ~0.0
    assert math.isclose(w[0], 0.0, abs_tol=1e-3)
    assert math.isclose(w[-1], 0.0, abs_tol=1e-3)
    # Symmetric
    for i in range(n // 2):
        assert math.isclose(w[i], w[n - 1 - i], rel_tol=1e-5)


def test_window_len_one() -> None:
    for wt in WindowType:
        w = build_window(wt, 1)
        assert w == [1.0]
        # Also test passing WindowType enum directly
        w_enum = build_window(wt, 16)
        assert len(w_enum) == 16
