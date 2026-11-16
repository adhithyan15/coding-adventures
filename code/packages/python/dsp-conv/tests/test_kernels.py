"""Unit tests for image filter design helpers."""

from __future__ import annotations

import math

from dsp_conv import (
    BoundaryMode,
    box_blur_kernel,
    conv2d,
    gaussian_blur_kernel,
    laplacian_kernel,
    sharpen_kernel,
    sobel_x_kernel,
    sobel_y_kernel,
)


def test_gaussian_blur_properties() -> None:
    k = gaussian_blur_kernel(sigma=1.0, size=5)
    assert len(k) == 5
    assert math.isclose(sum(k), 1.0, rel_tol=1e-5)
    # Symmetric
    assert math.isclose(k[0], k[4], rel_tol=1e-5)
    assert math.isclose(k[1], k[3], rel_tol=1e-5)
    # Peak at centre
    assert k[2] > k[1] > k[0]


def test_box_blur_properties() -> None:
    k = box_blur_kernel(7)
    assert len(k) == 7
    assert math.isclose(sum(k), 1.0, rel_tol=1e-5)
    for v in k:
        assert math.isclose(v, 1.0 / 7, rel_tol=1e-5)


def test_sobel_edge_detection() -> None:
    # Vertical edge image: left columns 0.0, right columns 10.0
    h, w = 5, 5
    img = [0.0 if col < 2 else 10.0 for _ in range(h) for col in range(w)]
    sx = sobel_x_kernel()
    resp_x = conv2d(img, sx, h, w, 3, 3, mode=BoundaryMode.REPLICATE)
    # Response magnitude should be strong at column 2
    mid_resp = abs(resp_x[2 * w + 2])
    assert mid_resp > 10.0

    sy = sobel_y_kernel()
    resp_y = conv2d(img, sy, h, w, 3, 3, mode=BoundaryMode.REPLICATE)
    # Since edge is vertical, vertical gradient should be zero
    assert math.isclose(resp_y[2 * w + 2], 0.0, abs_tol=1e-5)


def test_laplacian_and_sharpen() -> None:
    lap = laplacian_kernel()
    assert len(lap) == 9
    assert sum(lap) == 0.0

    sh = sharpen_kernel(amount=1.5)
    assert len(sh) == 9
    assert math.isclose(sum(sh), 1.0, rel_tol=1e-5)
