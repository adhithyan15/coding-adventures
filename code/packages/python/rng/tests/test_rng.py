"""Tests for rng — LCG, Xorshift64, and PCG32.

All reference values are cross-checked against the Go reference implementation.
"""

from __future__ import annotations

import pytest

from rng import LCG, PCG32, Xorshift64, __version__

# ── Sanity ────────────────────────────────────────────────────────────────────


class TestVersion:
    """Verify the package is importable and has a version."""

    def test_version_exists(self) -> None:
        assert __version__ == "0.1.0"


# ── Reference values ──────────────────────────────────────────────────────────
#
# First three next_u32() outputs for seed=1 — must match Go exactly.


class TestKnownValues:
    """Cross-language reference output for seed=1."""

    def test_lcg_seed1(self) -> None:
        g = LCG(1)
        assert g.next_u32() == 1817669548
        assert g.next_u32() == 2187888307
        assert g.next_u32() == 2784682393

    def test_xorshift64_seed1(self) -> None:
        g = Xorshift64(1)
        assert g.next_u32() == 1082269761
        assert g.next_u32() == 201397313
        assert g.next_u32() == 1854285353

    def test_pcg32_seed1(self) -> None:
        g = PCG32(1)
        assert g.next_u32() == 1412771199
        assert g.next_u32() == 1791099446
        assert g.next_u32() == 124312908


# ── Determinism ───────────────────────────────────────────────────────────────


class TestDeterminism:
    """Same seed → same sequence."""

    def test_lcg_deterministic(self) -> None:
        a = [LCG(42).next_u32() for _ in range(10)]  # noqa: F821 — comprehension trick
        g1 = LCG(42)
        g2 = LCG(42)
        seq1 = [g1.next_u32() for _ in range(10)]
        seq2 = [g2.next_u32() for _ in range(10)]
        assert seq1 == seq2
        _ = a  # keep linter happy

    def test_xorshift64_deterministic(self) -> None:
        g1 = Xorshift64(42)
        g2 = Xorshift64(42)
        assert [g1.next_u32() for _ in range(10)] == [g2.next_u32() for _ in range(10)]

    def test_pcg32_deterministic(self) -> None:
        g1 = PCG32(42)
        g2 = PCG32(42)
        assert [g1.next_u32() for _ in range(10)] == [g2.next_u32() for _ in range(10)]


# ── Different seeds diverge ────────────────────────────────────────────────────


class TestDifferentSeeds:
    """Different seeds must produce different sequences."""

    def test_lcg_diverges(self) -> None:
        g1, g2 = LCG(1), LCG(2)
        seq1 = [g1.next_u32() for _ in range(5)]
        seq2 = [g2.next_u32() for _ in range(5)]
        assert seq1 != seq2

    def test_xorshift64_diverges(self) -> None:
        g1, g2 = Xorshift64(1), Xorshift64(2)
        seq1 = [g1.next_u32() for _ in range(5)]
        seq2 = [g2.next_u32() for _ in range(5)]
        assert seq1 != seq2

    def test_pcg32_diverges(self) -> None:
        g1, g2 = PCG32(1), PCG32(2)
        seq1 = [g1.next_u32() for _ in range(5)]
        seq2 = [g2.next_u32() for _ in range(5)]
        assert seq1 != seq2


# ── Seed-0 Xorshift64 ─────────────────────────────────────────────────────────


class TestXorshift64Seed0:
    """Seed 0 is replaced with 1; generator must never get stuck at zero."""

    def test_seed0_not_stuck(self) -> None:
        g = Xorshift64(0)
        for i in range(100):
            v = g.next_u32()
            assert v != 0, f"Xorshift64 output zero at step {i}"

    def test_seed0_first_value_matches_seed1(self) -> None:
        # Seed 0 is silently replaced with 1, so first output should equal seed=1.
        g0 = Xorshift64(0)
        g1 = Xorshift64(1)
        assert g0.next_u32() == g1.next_u32()


# ── Float range ───────────────────────────────────────────────────────────────


class TestFloatRange:
    """next_float() must return values in [0.0, 1.0)."""

    @pytest.mark.parametrize("cls,seed", [(LCG, 7), (Xorshift64, 7), (PCG32, 7)])
    def test_float_in_range(self, cls: type, seed: int) -> None:
        g = cls(seed)
        for _ in range(1000):
            f = g.next_float()
            assert 0.0 <= f < 1.0, f"{cls.__name__} produced float {f}"


# ── Integer range bounds ───────────────────────────────────────────────────────


class TestIntInRangeBounds:
    """next_int_in_range(min, max) must always satisfy min <= result <= max."""

    @pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
    def test_die_roll_bounds(self, cls: type) -> None:
        g = cls(999)
        for _ in range(1000):
            v = g.next_int_in_range(1, 6)
            assert 1 <= v <= 6, f"{cls.__name__} returned {v}"

    @pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
    def test_single_value_range(self, cls: type) -> None:
        g = cls(5)
        for _ in range(20):
            assert g.next_int_in_range(42, 42) == 42

    @pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
    def test_negative_range(self, cls: type) -> None:
        g = cls(11)
        for _ in range(500):
            v = g.next_int_in_range(-10, -1)
            assert -10 <= v <= -1, f"{cls.__name__} returned {v}"


# ── Distribution ──────────────────────────────────────────────────────────────
#
# Roll a 6-sided die 12 000 times. Each face should appear ~2000 ± 30% times.


def _check_distribution(counts: list[int], label: str) -> None:
    for face_idx, count in enumerate(counts):
        assert 1400 <= count <= 2600, (
            f"{label}: face {face_idx + 1} appeared {count} times (expected ~2000 ±30%)"
        )


class TestDistribution:
    """Coarse uniformity test — 12 000 die rolls, each face within ±30%."""

    def test_lcg_distribution(self) -> None:
        g = LCG(123)
        counts = [0] * 6
        for _ in range(12_000):
            counts[g.next_int_in_range(1, 6) - 1] += 1
        _check_distribution(counts, "LCG")

    def test_xorshift64_distribution(self) -> None:
        g = Xorshift64(123)
        counts = [0] * 6
        for _ in range(12_000):
            counts[g.next_int_in_range(1, 6) - 1] += 1
        _check_distribution(counts, "Xorshift64")

    def test_pcg32_distribution(self) -> None:
        g = PCG32(123)
        counts = [0] * 6
        for _ in range(12_000):
            counts[g.next_int_in_range(1, 6) - 1] += 1
        _check_distribution(counts, "PCG32")


# ── next_u64 composition ──────────────────────────────────────────────────────
#
# next_u64 must equal (hi << 32) | lo where hi and lo are successive next_u32
# calls on an identically-seeded generator.


class TestU64Composition:
    """next_u64() == (next_u32() << 32) | next_u32()."""

    @pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
    def test_u64_composition(self, cls: type) -> None:
        g_u64 = cls(55)
        g_u32 = cls(55)
        for _ in range(50):
            u64_val = g_u64.next_u64()
            hi = g_u32.next_u32()
            lo = g_u32.next_u32()
            expected = (hi << 32) | lo
            assert u64_val == expected, f"{cls.__name__}: u64={u64_val} vs {expected}"


# ── Output range sanity ───────────────────────────────────────────────────────


class TestOutputRange:
    """next_u32() fits in 32 bits; next_u64() fits in 64 bits."""

    @pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
    def test_u32_range(self, cls: type) -> None:
        g = cls(77)
        for _ in range(200):
            v = g.next_u32()
            assert 0 <= v < 2**32, f"{cls.__name__} u32 out of range: {v}"

    @pytest.mark.parametrize("cls", [LCG, Xorshift64, PCG32])
    def test_u64_range(self, cls: type) -> None:
        g = cls(77)
        for _ in range(100):
            v = g.next_u64()
            assert 0 <= v < 2**64, f"{cls.__name__} u64 out of range: {v}"
