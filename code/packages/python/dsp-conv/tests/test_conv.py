"""Unit tests for 1-D convolution and correlation."""

from __future__ import annotations

from dsp_conv import (
    BoundaryMode,
    ConvMode,
    conv1d,
    convolve,
    correlate,
)


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_conv1d_identity() -> None:
    sig = [1.0, 2.0, 3.0, 4.0, 5.0]
    out = conv1d(sig, [1.0], mode=BoundaryMode.ZERO)
    _assert_close(sig, out)


def test_conv1d_boundary_modes() -> None:
    sig = [1.0, 2.0, 3.0]
    ker = [1.0, 2.0, 1.0]

    out_zero = conv1d(sig, ker, mode=BoundaryMode.ZERO)
    _assert_close(out_zero, [4.0, 8.0, 8.0])

    out_rep = conv1d(sig, ker, mode=BoundaryMode.REPLICATE)
    _assert_close(out_rep, [5.0, 8.0, 11.0])

    out_ref = conv1d(sig, ker, mode=BoundaryMode.REFLECT)
    _assert_close(out_ref, [6.0, 8.0, 10.0])

    out_wrap = conv1d(sig, ker, mode=BoundaryMode.WRAP)
    _assert_close(out_wrap, [7.0, 8.0, 9.0])


def test_convolve_full_mode() -> None:
    x = [1.0, 2.0, 3.0]
    h = [0.5, 1.0]
    out = convolve(x, h, mode=ConvMode.FULL)
    _assert_close(out, [0.5, 2.0, 3.5, 3.0])


def test_convolve_valid_mode() -> None:
    x = [1.0, 2.0, 3.0, 4.0, 5.0]
    h = [1.0, 2.0]
    # Valid mode length: 5 - 2 + 1 = 4
    out = convolve(x, h, mode=ConvMode.VALID)
    # n=1: x[1]*h[0] + x[0]*h[1] = 2*1 + 1*2 = 4
    # n=2: x[2]*h[0] + x[1]*h[1] = 3*1 + 2*2 = 7
    # n=3: x[3]*h[0] + x[2]*h[1] = 4*1 + 3*2 = 10
    # n=4: x[4]*h[0] + x[3]*h[1] = 5*1 + 4*2 = 13
    _assert_close(out, [4.0, 7.0, 10.0, 13.0])


def test_correlate_impulse() -> None:
    x = [1.0, 2.0, 3.0, 4.0]
    delta = [1.0]
    out = correlate(x, delta, mode=ConvMode.SAME)
    _assert_close(out, x)
