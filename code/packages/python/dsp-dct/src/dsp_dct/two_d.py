"""Two-dimensional Discrete Cosine Transform (2-D DCT / IDCT).

Conforms to DSP02 Phase 4 and VER15 security/weakness matrix invariants:
- Row-then-column factorization for [H, W] images (JPEG / pHash substrate).
- Strict CWE-400 bounds on height, width, and total sample count.
- CWE-20 finite float validation fail-closed.
- Zero recursion (CWE-674).
- Zero regular expressions (CWE-1333).
"""

from __future__ import annotations

import math
from collections.abc import Sequence

from dsp_dct.dct import (
    MAX_DCT_LEN,
    MAX_SAMPLES,
    DctNorm,
    DctType,
    EmptyInputError,
    InvalidInputError,
    _parse_norm,
    _parse_type,
    dct,
    idct,
)


def _validate_2d_params(
    image: Sequence[float], height: int, width: int
) -> int:
    """Validate 2-D dimensions, bounds, and finite floats."""
    if len(image) == 0:
        raise EmptyInputError("image input must not be empty")
    if height < 1:
        raise InvalidInputError("height must be at least 1")
    if width < 1:
        raise InvalidInputError("width must be at least 1")
    if height > MAX_DCT_LEN:
        raise InvalidInputError(
            f"height {height} exceeds MAX_DCT_LEN ({MAX_DCT_LEN})"
        )
    if width > MAX_DCT_LEN:
        raise InvalidInputError(
            f"width {width} exceeds MAX_DCT_LEN ({MAX_DCT_LEN})"
        )

    total_samples = height * width
    if total_samples > MAX_SAMPLES:
        raise InvalidInputError(
            f"total samples {total_samples} exceeds MAX_SAMPLES ({MAX_SAMPLES})"
        )
    if len(image) != total_samples:
        raise InvalidInputError(
            f"image length {len(image)} does not match height * width ({total_samples})"
        )

    for x in image:
        if not math.isfinite(float(x)):
            raise InvalidInputError(
                "input contains non-finite float (NaN or Inf)"
            )

    return total_samples


def dct_2d(
    image: Sequence[float],
    height: int,
    width: int,
    dct_type: DctType | int | str = DctType.II,
    norm: DctNorm | str | int = DctNorm.ORTHO,
) -> list[float]:
    """Compute 2-D Discrete Cosine Transform via row-then-column factorization."""
    _validate_2d_params(image, height, width)
    t = _parse_type(dct_type)
    nm = _parse_norm(norm)

    # 1. Transform each row of length width
    row_transformed: list[float] = [0.0] * (height * width)
    for r in range(height):
        row_slice = image[r * width : (r + 1) * width]
        t_row = dct(row_slice, t, nm)
        row_transformed[r * width : (r + 1) * width] = t_row

    # 2. Transform each column of length height
    out: list[float] = [0.0] * (height * width)
    for c in range(width):
        col_slice = [row_transformed[r * width + c] for r in range(height)]
        t_col = dct(col_slice, t, nm)
        for r in range(height):
            out[r * width + c] = t_col[r]

    return out


def idct_2d(
    image: Sequence[float],
    height: int,
    width: int,
    dct_type: DctType | int | str = DctType.III,
    norm: DctNorm | str | int = DctNorm.ORTHO,
) -> list[float]:
    """Compute inverse 2-D Discrete Cosine Transform."""
    _validate_2d_params(image, height, width)
    t = _parse_type(dct_type)
    nm = _parse_norm(norm)

    # 1. Inverse transform each column of length height
    col_inverted: list[float] = [0.0] * (height * width)
    for c in range(width):
        col_slice = [image[r * width + c] for r in range(height)]
        inv_col = idct(col_slice, t, nm)
        for r in range(height):
            col_inverted[r * width + c] = inv_col[r]

    # 2. Inverse transform each row of length width
    out: list[float] = [0.0] * (height * width)
    for r in range(height):
        row_slice = col_inverted[r * width : (r + 1) * width]
        inv_row = idct(row_slice, t, nm)
        out[r * width : (r + 1) * width] = inv_row

    return out
