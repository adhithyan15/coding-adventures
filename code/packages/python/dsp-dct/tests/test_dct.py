"""Unit tests for Discrete Cosine Transforms (DCT-I..IV)."""

from __future__ import annotations

import math

from dsp_dct import DctNorm, DctType, dct, idct


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_dct_ii_iii_ortho_round_trip() -> None:
    for n in [1, 2, 4, 7, 8, 15, 16, 32, 63, 64]:
        sig = [math.sin(0.4 * i) + 0.2 * i for i in range(n)]
        coeffs = dct(sig, DctType.II, DctNorm.ORTHO)
        rec = idct(coeffs, DctType.III, DctNorm.ORTHO)
        _assert_close(sig, rec, tol=1e-4)


def test_dct_ii_iii_unnorm_round_trip() -> None:
    for n in [1, 2, 5, 8, 16]:
        sig = [float(i + 1) for i in range(n)]
        coeffs = dct(sig, DctType.II, DctNorm.NONE)
        rec = idct(coeffs, DctType.III, DctNorm.NONE)
        _assert_close(sig, rec, tol=1e-4)


def test_dct_iii_forward_and_ii_inverse() -> None:
    # Forward DCT-III and inverse using DCT-II under both ortho and none
    for norm in [DctNorm.ORTHO, DctNorm.NONE]:
        sig = [1.0, 2.0, 3.0, 4.0]
        coeffs = dct(sig, DctType.III, norm)
        rec = idct(coeffs, DctType.II, norm)
        _assert_close(sig, rec, tol=1e-4)


def test_dct_i_round_trip() -> None:
    for n in [2, 3, 5, 8, 16]:
        sig = [math.cos(0.5 * i) for i in range(n)]
        # Ortho
        coeffs_ortho = dct(sig, DctType.I, DctNorm.ORTHO)
        rec_ortho = idct(coeffs_ortho, DctType.I, DctNorm.ORTHO)
        _assert_close(sig, rec_ortho, tol=1e-4)

        # None
        coeffs_none = dct(sig, DctType.I, DctNorm.NONE)
        rec_none = idct(coeffs_none, DctType.I, DctNorm.NONE)
        _assert_close(sig, rec_none, tol=1e-4)


def test_dct_iv_round_trip() -> None:
    for n in [1, 2, 4, 7, 8, 16]:
        sig = [math.sin(0.6 * i) for i in range(n)]
        # Ortho
        coeffs_ortho = dct(sig, DctType.IV, DctNorm.ORTHO)
        rec_ortho = idct(coeffs_ortho, DctType.IV, DctNorm.ORTHO)
        _assert_close(sig, rec_ortho, tol=1e-4)

        # None
        coeffs_none = dct(sig, DctType.IV, DctNorm.NONE)
        rec_none = idct(coeffs_none, DctType.IV, DctNorm.NONE)
        _assert_close(sig, rec_none, tol=1e-4)


def test_dct_dc_energy() -> None:
    n = 8
    sig = [2.0] * n
    coeffs = dct(sig, DctType.II, DctNorm.ORTHO)
    expected_dc = 2.0 * math.sqrt(n)
    assert math.isclose(coeffs[0], expected_dc, rel_tol=1e-5)
    for c in coeffs[1:]:
        assert abs(c) < 1e-5


def test_type_and_norm_string_and_int_parsing() -> None:
    sig = [1.0, 2.0, 3.0, 4.0]
    # String types & norms
    c1 = dct(sig, dct_type="II", norm="ortho")
    c2 = dct(sig, dct_type="2", norm="none")
    c3 = dct(sig, dct_type=2, norm=1)
    c4 = dct(sig, dct_type=2, norm=0)
    c5 = dct(sig, dct_type="I", norm="unitary")
    c6 = dct(sig, dct_type="IV", norm="raw")
    assert len(c1) == 4
    assert len(c2) == 4
    assert len(c3) == 4
    assert len(c4) == 4
    assert len(c5) == 4
    assert len(c6) == 4
