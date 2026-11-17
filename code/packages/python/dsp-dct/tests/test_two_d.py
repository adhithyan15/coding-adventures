"""Unit tests for 2-D Discrete Cosine Transforms."""

from __future__ import annotations

import math

from dsp_dct import DctNorm, DctType, dct_2d, idct_2d


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_dct_2d_round_trip() -> None:
    # Test symmetric and asymmetric sizes, including 8x8 (JPEG block)
    test_dims = [(2, 2), (4, 4), (8, 8), (3, 7), (5, 4)]
    for h, w in test_dims:
        image = [math.sin(0.3 * r + 0.5 * c) for r in range(h) for c in range(w)]
        coeffs = dct_2d(image, h, w, DctType.II, DctNorm.ORTHO)
        rec = idct_2d(coeffs, h, w, DctType.III, DctNorm.ORTHO)
        _assert_close(image, rec, tol=1e-4)
