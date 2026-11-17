"""Property-based testing for dsp_fft using deterministic SplitMix64 PRNG."""

from __future__ import annotations

import math

from dsp_fft import (
    Direction,
    bluestein_scalar,
    fft_scalar,
    ifft_scalar,
    irfft_scalar,
    rfft_scalar,
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


def test_pbt_radix2_round_trip() -> None:
    prng = SplitMix64(0x1337C0DE)
    for n in [2, 4, 8, 16, 32, 64, 128]:
        sig = [prng.next_f64() for _ in range(2 * n)]
        spec = fft_scalar(sig)
        rec = ifft_scalar(spec)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_bluestein_round_trip() -> None:
    prng = SplitMix64(0xBAADF00D)
    for n in [3, 5, 7, 11, 19, 37, 53]:
        sig = [prng.next_f64() for _ in range(2 * n)]
        spec = bluestein_scalar(sig, Direction.FORWARD)
        rec = bluestein_scalar(spec, Direction.INVERSE)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_rfft_irfft_round_trip() -> None:
    prng = SplitMix64(0xCAFED00D)
    for n in [1, 2, 5, 8, 13, 27, 48, 65]:
        sig = [prng.next_f64() for _ in range(n)]
        half = rfft_scalar(sig)
        rec = irfft_scalar(half, n)
        _assert_close(sig, rec, tol=1e-4)


def test_pbt_parseval_theorem() -> None:
    # Sum of |x|^2 = (1 / N) * Sum of |X|^2
    prng = SplitMix64(0x0BADCAFE)
    for n in [4, 8, 16, 32]:
        sig = [prng.next_f64() for _ in range(2 * n)]
        time_energy = sum(x * x for x in sig)

        spec = fft_scalar(sig)
        freq_energy = sum(X * X for X in spec) / n

        assert math.isclose(time_energy, freq_energy, rel_tol=1e-4)
