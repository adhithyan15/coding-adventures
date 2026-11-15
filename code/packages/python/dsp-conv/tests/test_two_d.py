"""Unit tests for 2-D and separable convolution."""

from __future__ import annotations

import math

import pytest

from dsp_conv import (
    BoundaryMode,
    ConvMode,
    ImageSizeMismatchError,
    conv2d,
    convolve2d,
    sep_conv2d,
)


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_conv2d_identity() -> None:
    img = [float(i) for i in range(16)]  # 4x4
    kernel = [1.0]  # 1x1
    out = conv2d(img, kernel, 4, 4, 1, 1, mode=BoundaryMode.ZERO)
    _assert_close(img, out)


def test_conv2d_delta_centred() -> None:
    img = [float(i) for i in range(9)]  # 3x3
    # 3x3 delta kernel
    kernel = [
        0.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 0.0,
    ]
    out = conv2d(img, kernel, 3, 3, 3, 3, mode=BoundaryMode.ZERO)
    _assert_close(img, out)


def test_conv2d_boundary_modes() -> None:
    img = [1.0, 2.0, 3.0, 4.0]  # 2x2
    ker = [0.0, 1.0, 1.0, 0.0]  # 2x2
    modes = [
        BoundaryMode.ZERO,
        BoundaryMode.REPLICATE,
        BoundaryMode.REFLECT,
        BoundaryMode.WRAP,
    ]
    for mode in modes:
        out = conv2d(img, ker, 2, 2, 2, 2, mode=mode)
        assert len(out) == 4


def test_sep_conv2d_matches_conv2d_outer_product() -> None:
    h = 5
    w = 6
    img = [math.sin(0.4 * i + 0.1 * j) for i in range(h) for j in range(w)]
    h_ker = [0.25, 0.5, 0.25]
    v_ker = [0.2, 0.6, 0.2]

    # Outer product kernel: 3x3
    outer_ker = [v * u for v in v_ker for u in h_ker]

    out_direct = conv2d(img, outer_ker, h, w, 3, 3, mode=BoundaryMode.REFLECT)
    out_sep = sep_conv2d(img, h_ker, v_ker, h, w, mode=BoundaryMode.REFLECT)
    _assert_close(out_direct, out_sep, tol=1e-5)


def test_convolve2d_nested_and_flat() -> None:
    img_2d = [
        [1.0, 2.0, 3.0],
        [4.0, 5.0, 6.0],
        [7.0, 8.0, 9.0],
    ]
    ker_2d = [
        [1.0, 0.0],
        [0.0, 1.0],
    ]
    res_valid = convolve2d(img_2d, ker_2d, mode=ConvMode.VALID)
    assert len(res_valid) == 2
    assert len(res_valid[0]) == 2
    assert res_valid == [[6.0, 8.0], [12.0, 14.0]]

    # SAME mode nested
    res_same = convolve2d(img_2d, ker_2d, mode=ConvMode.SAME)
    assert len(res_same) == 3
    assert len(res_same[0]) == 3

    # FULL mode nested
    res_full = convolve2d(img_2d, ker_2d, mode=ConvMode.FULL)
    assert len(res_full) == 4
    assert len(res_full[0]) == 4

    # Flat image and flat kernel with explicit dimensions
    flat_img = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0, 9.0]
    flat_ker = [1.0, 0.0, 0.0, 1.0]
    flat_full = convolve2d(
        flat_img, flat_ker,
        mode=ConvMode.FULL,
        image_height=3, image_width=3,
        kernel_height=2, kernel_width=2,
    )
    assert len(flat_full) == 16

    flat_valid = convolve2d(
        flat_img, flat_ker,
        mode=ConvMode.VALID,
        image_height=3, image_width=3,
        kernel_height=2, kernel_width=2,
    )
    assert len(flat_valid) == 4

    flat_same = convolve2d(
        flat_img, flat_ker,
        mode=ConvMode.SAME,
        image_height=3, image_width=3,
        kernel_height=2, kernel_width=2,
    )
    assert len(flat_same) == 9

    with pytest.raises(ImageSizeMismatchError):
        convolve2d(img_2d, flat_ker)  # Missing kernel dimensions for flat kernel
