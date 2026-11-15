"""Unit tests for overlap-add block convolution."""

from __future__ import annotations

import math

from dsp_conv import ConvMode, convolve, overlap_add


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_overlap_add_matches_direct_full() -> None:
    x = [math.sin(0.15 * i) for i in range(128)]
    h = [math.cos(0.3 * j) for j in range(15)]
    direct = convolve(x, h, mode=ConvMode.FULL)
    ola = overlap_add(x, h, block_size=32, mode=ConvMode.FULL)
    _assert_close(direct, ola, tol=1e-4)


def test_overlap_add_same_and_valid() -> None:
    x = [float(i) for i in range(64)]
    h = [0.1, 0.2, 0.4, 0.2, 0.1]
    same_direct = convolve(x, h, mode=ConvMode.SAME)
    same_ola = overlap_add(x, h, block_size=16, mode=ConvMode.SAME)
    _assert_close(same_direct, same_ola, tol=1e-4)

    valid_direct = convolve(x, h, mode=ConvMode.VALID)
    valid_ola = overlap_add(x, h, block_size=16, mode=ConvMode.VALID)
    _assert_close(valid_direct, valid_ola, tol=1e-4)
