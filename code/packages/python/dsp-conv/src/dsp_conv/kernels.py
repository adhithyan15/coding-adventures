"""Image filter design helpers (Gaussian, Sobel, Box, Laplacian, Sharpen)."""

from __future__ import annotations

import math

from dsp_conv.boundary import (
    MAX_KERNEL_LEN,
    InvalidInputError,
)


def gaussian_blur_kernel(sigma: float, size: int) -> list[float]:
    """1-D Gaussian blur kernel with standard deviation sigma and length size.

    Normalised so the sum is 1.0. Size must be odd and positive.
    """
    if size <= 0 or size % 2 == 0:
        raise InvalidInputError(f"size must be odd and > 0; got {size}")
    if size > MAX_KERNEL_LEN:
        raise InvalidInputError(
            f"size {size} exceeds MAX_KERNEL_LEN {MAX_KERNEL_LEN}"
        )
    if not math.isfinite(sigma) or sigma <= 0.0:
        raise InvalidInputError(
            f"sigma must be positive and finite; got {sigma}"
        )

    centre = (size - 1) / 2.0
    two_sigma_sq = 2.0 * sigma * sigma
    norm_pdf = 1.0 / (sigma * math.sqrt(2.0 * math.pi))

    kernel = [0.0] * size
    for i in range(size):
        x = i - centre
        kernel[i] = norm_pdf * math.exp(-(x * x) / two_sigma_sq)

    total = sum(kernel)
    if total > 0.0:
        kernel = [val / total for val in kernel]
    return kernel


def box_blur_kernel(size: int) -> list[float]:
    """1-D box blur (uniform) kernel of length size summing to 1.0."""
    if size <= 0:
        raise InvalidInputError(f"size must be > 0; got {size}")
    if size > MAX_KERNEL_LEN:
        raise InvalidInputError(
            f"size {size} exceeds MAX_KERNEL_LEN {MAX_KERNEL_LEN}"
        )
    val = 1.0 / size
    return [val] * size


def sobel_x_kernel() -> list[float]:
    """3x3 Sobel kernel for horizontal edge detection (gradient in X)."""
    return [
        -1.0, 0.0, 1.0,
        -2.0, 0.0, 2.0,
        -1.0, 0.0, 1.0,
    ]


def sobel_y_kernel() -> list[float]:
    """3x3 Sobel kernel for vertical edge detection (gradient in Y)."""
    return [
        -1.0, -2.0, -1.0,
         0.0,  0.0,  0.0,
         1.0,  2.0,  1.0,
    ]


def laplacian_kernel() -> list[float]:
    """3x3 discrete Laplacian kernel (second derivative)."""
    return [
        0.0,  1.0, 0.0,
        1.0, -4.0, 1.0,
        0.0,  1.0, 0.0,
    ]


def sharpen_kernel(amount: float = 1.0) -> list[float]:
    """3x3 sharpen kernel: identity + scaled negative Laplacian."""
    if not math.isfinite(amount):
        raise InvalidInputError(f"amount must be finite; got {amount}")
    return [
        0.0, -amount, 0.0,
        -amount, 1.0 + 4.0 * amount, -amount,
        0.0, -amount, 0.0,
    ]
