"""Unit tests for Discrete Sine Transforms (DST-I..IV)."""

from __future__ import annotations

import math

from dsp_dct import DctNorm, DstType, dst, idst


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_dst_i_round_trip() -> None:
    for n in [1, 2, 4, 7, 8, 16]:
        sig = [math.sin(0.4 * (i + 1)) for i in range(n)]
        # Ortho
        coeffs_ortho = dst(sig, DstType.I, DctNorm.ORTHO)
        rec_ortho = idst(coeffs_ortho, DstType.I, DctNorm.ORTHO)
        _assert_close(sig, rec_ortho, tol=1e-4)

        # None
        coeffs_none = dst(sig, DstType.I, DctNorm.NONE)
        rec_none = idst(coeffs_none, DstType.I, DctNorm.NONE)
        _assert_close(sig, rec_none, tol=1e-4)


def test_dst_ii_iii_round_trip() -> None:
    for n in [1, 2, 4, 7, 8, 16]:
        sig = [math.sin(0.5 * (i + 1)) for i in range(n)]
        # Ortho: forward II, inverse III
        coeffs_ortho = dst(sig, DstType.II, DctNorm.ORTHO)
        rec_ortho = idst(coeffs_ortho, DstType.III, DctNorm.ORTHO)
        _assert_close(sig, rec_ortho, tol=1e-4)

        # None: forward II, inverse III
        coeffs_none = dst(sig, DstType.II, DctNorm.NONE)
        rec_none = idst(coeffs_none, DstType.III, DctNorm.NONE)
        _assert_close(sig, rec_none, tol=1e-4)

        # Forward III, inverse II
        c_iii = dst(sig, DstType.III, DctNorm.ORTHO)
        r_ii = idst(c_iii, DstType.II, DctNorm.ORTHO)
        _assert_close(sig, r_ii, tol=1e-4)

        c_iii_none = dst(sig, DstType.III, DctNorm.NONE)
        r_ii_none = idst(c_iii_none, DstType.II, DctNorm.NONE)
        _assert_close(sig, r_ii_none, tol=1e-4)


def test_dst_iv_round_trip() -> None:
    for n in [1, 2, 4, 7, 8, 16]:
        sig = [math.sin(0.3 * (i + 1)) for i in range(n)]
        # Ortho
        coeffs_ortho = dst(sig, DstType.IV, DctNorm.ORTHO)
        rec_ortho = idst(coeffs_ortho, DstType.IV, DctNorm.ORTHO)
        _assert_close(sig, rec_ortho, tol=1e-4)

        # None
        coeffs_none = dst(sig, DstType.IV, DctNorm.NONE)
        rec_none = idst(coeffs_none, DstType.IV, DctNorm.NONE)
        _assert_close(sig, rec_none, tol=1e-4)


def test_dst_parsing_and_types() -> None:
    sig = [1.0, 2.0, 3.0]
    c1 = dst(sig, dst_type="II", norm="ortho")
    c2 = dst(sig, dst_type="2", norm="none")
    c3 = dst(sig, dst_type=1, norm="unitary")
    c4 = dst(sig, dst_type=3, norm="raw")
    c5 = dst(sig, dst_type=4, norm="ortho")
    assert len(c1) == 3
    assert len(c2) == 3
    assert len(c3) == 3
    assert len(c4) == 3
    assert len(c5) == 3
