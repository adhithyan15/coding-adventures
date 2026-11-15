"""Property-based testing for dsp_dct using deterministic SplitMix64 PRNG."""

from __future__ import annotations

import math

from dsp_dct import (
    DctNorm,
    DctType,
    DstType,
    dct,
    dct_2d,
    dst,
    idct,
    idct_2d,
    idst,
)


class SplitMix64:
    """Pure-Python deterministic 64-bit PRNG."""

    def __init__(self, seed: int = 0x123456789ABCDEF0) -> None:
        self.state: int = seed & 0xFFFFFFFFFFFFFFFF

    def next_u64(self) -> int:
        self.state = (self.state + 0x9E3779B97F4A7C15) & 0xFFFFFFFFFFFFFFFF
        z = self.state
        z = ((z ^ (z >> 30)) * 0xBF58476D1CE4E5B9) & 0xFFFFFFFFFFFFFFFF
        z = ((z ^ (z >> 27)) * 0x94D049BB133111EB) & 0xFFFFFFFFFFFFFFFF
        return (z ^ (z >> 31)) & 0xFFFFFFFFFFFFFFFF

    def next_f64(self) -> float:
        u = (self.next_u64() >> 11) / (1 << 53)
        return (u * 20.0) - 10.0


def _assert_close(a: list[float], b: list[float], tol: float = 1e-4) -> None:
    assert len(a) == len(b), f"length mismatch: {len(a)} vs {len(b)}"
    for idx, (x, y) in enumerate(zip(a, b, strict=True)):
        scale = max(abs(x), abs(y), 1.0)
        assert abs(x - y) <= scale * tol, f"mismatch at {idx}: {x} vs {y} (tol {tol})"


def test_pbt_dct_round_trip() -> None:
    prng = SplitMix64(0xABCD1234)
    for n in [1, 2, 4, 7, 8, 15, 16, 31, 32]:
        sig = [prng.next_f64() for _ in range(n)]
        coeffs = dct(sig, DctType.II, DctNorm.ORTHO)
        rec = idct(coeffs, DctType.III, DctNorm.ORTHO)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_dst_round_trip() -> None:
    prng = SplitMix64(0x4321DCBA)
    for n in [1, 2, 4, 7, 8, 16]:
        sig = [prng.next_f64() for _ in range(n)]
        coeffs = dst(sig, DstType.II, DctNorm.ORTHO)
        rec = idst(coeffs, DstType.III, DctNorm.ORTHO)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_2d_round_trip() -> None:
    prng = SplitMix64(0xFEED1337)
    for h, w in [(2, 2), (4, 4), (3, 5), (6, 4)]:
        img = [prng.next_f64() for _ in range(h * w)]
        coeffs = dct_2d(img, h, w, DctType.II, DctNorm.ORTHO)
        rec = idct_2d(coeffs, h, w, DctType.III, DctNorm.ORTHO)
        _assert_close(img, rec, tol=1e-4)


def test_pbt_ortho_energy_conservation() -> None:
    # Under Ortho, Parseval holds: sum(x^2) == sum(X^2)
    prng = SplitMix64(0xCAFEDEAD)
    for n in [4, 8, 16, 32]:
        sig = [prng.next_f64() for _ in range(n)]
        time_energy = sum(x * x for x in sig)

        coeffs = dct(sig, DctType.II, DctNorm.ORTHO)
        freq_energy = sum(c * c for c in coeffs)

        assert math.isclose(time_energy, freq_energy, rel_tol=1e-4)
