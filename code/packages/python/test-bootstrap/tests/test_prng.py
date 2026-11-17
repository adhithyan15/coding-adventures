"""Tests for deterministic SplitMix64 PRNG and FNV-1a hashing."""

import pytest

from coding_adventures_test_bootstrap.prng import SplitMix64, fnv1a64, shuffle


def test_splitmix64_reference_seed_zero() -> None:
    """Seed 0 must produce 0xe220a8397b1dcdaf (16294208416658607535) as first output."""
    rng = SplitMix64(0)
    first = rng.next_u64()
    assert first == 0xE220_A839_7B1D_CDAF
    assert first == 16_294_208_416_658_607_535


def test_splitmix64_determinism() -> None:
    """Same seed must produce identical outputs."""
    rng1 = SplitMix64(42)
    rng2 = SplitMix64(42)
    seq1 = [rng1.next_u64() for _ in range(50)]
    seq2 = [rng2.next_u64() for _ in range(50)]
    assert seq1 == seq2


def test_splitmix64_state_property() -> None:
    """State getter and setter."""
    rng = SplitMix64(123)
    assert rng.state == 123
    rng.state = 456
    assert rng.state == 456

    with pytest.raises(TypeError):
        SplitMix64("invalid")  # type: ignore[arg-type]

    with pytest.raises(TypeError):
        rng.state = "invalid"  # type: ignore[assignment]


def test_splitmix64_next_float() -> None:
    """Float must be in [0.0, 1.0)."""
    rng = SplitMix64(999)
    for _ in range(100):
        f = rng.next_float()
        assert 0.0 <= f < 1.0


def test_splitmix64_bounded() -> None:
    """bounded(n) must be in [0, n) and handle edge cases."""
    rng = SplitMix64(1001)

    assert rng.bounded(1) == 0

    val_max = rng.bounded(1 << 64)
    assert 0 <= val_max < (1 << 64)

    # Small bounds
    for n in (2, 3, 7, 10, 100, 1000):
        for _ in range(50):
            val = rng.bounded(n)
            assert 0 <= val < n

    # Invalid bounds
    with pytest.raises(TypeError):
        rng.bounded("10")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="n >= 1"):
        rng.bounded(0)
    with pytest.raises(ValueError, match="n >= 1"):
        rng.bounded(-5)
    with pytest.raises(ValueError, match="n <= 2\\^64"):
        rng.bounded((1 << 64) + 1)


def test_splitmix64_bounded_range() -> None:
    """bounded_range(min, max) must be in [min, max]."""
    rng = SplitMix64(777)
    assert rng.bounded_range(5, 5) == 5

    for _ in range(50):
        val = rng.bounded_range(-10, 10)
        assert -10 <= val <= 10

    with pytest.raises(TypeError):
        rng.bounded_range("a", 10)  # type: ignore[arg-type]
    with pytest.raises(TypeError):
        rng.bounded_range(10, "b")  # type: ignore[arg-type]
    with pytest.raises(ValueError, match="cannot exceed"):
        rng.bounded_range(10, 5)


def test_fnv1a64_reference_vectors() -> None:
    """FNV-1a 64-bit reference vectors."""
    assert fnv1a64(b"") == 0xCBF2_9CE4_8422_2325
    assert fnv1a64("") == 0xCBF2_9CE4_8422_2325
    assert fnv1a64(b"a") == 0xAF63_DC4C_8601_EC8C
    assert fnv1a64("a") == 0xAF63_DC4C_8601_EC8C
    assert fnv1a64(bytearray(b"a")) == 0xAF63_DC4C_8601_EC8C

    with pytest.raises(TypeError):
        fnv1a64(12345)  # type: ignore[arg-type]


def test_shuffle() -> None:
    """Fisher-Yates shuffle determinism and completeness."""
    rng1 = SplitMix64(1234)
    rng2 = SplitMix64(1234)

    items = list(range(10))
    s1 = shuffle(items, rng1)
    s2 = shuffle(items, rng2)
    assert s1 == s2
    assert sorted(s1) == items
    assert shuffle([], rng1) == []
    assert shuffle([42], rng1) == [42]
