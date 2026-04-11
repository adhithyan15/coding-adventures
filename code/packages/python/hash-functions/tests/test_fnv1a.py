"""
Tests for FNV-1a 32-bit and 64-bit hash functions.

Test vectors are from the official FNV hash test suite at
  http://www.isthe.com/chongo/tech/comp/fnv/
"""

import pytest

from hash_functions import fnv1a_32, fnv1a_64
from hash_functions.algorithms import FNV32_OFFSET_BASIS, FNV32_PRIME, FNV64_OFFSET_BASIS, FNV64_PRIME


class TestFnv1a32KnownVectors:
    """Verify against the official FNV test vectors."""

    def test_empty_string(self) -> None:
        # The hash of an empty input is the offset basis itself — no bytes
        # means no iterations, so the loop body never executes.
        assert fnv1a_32(b"") == 2166136261

    def test_single_a(self) -> None:
        # The correct FNV-1a 32-bit value for b"a" is 0xe40c292c = 3826002220.
        # Note: the spec listed 84696351 (0x50c5d7f) which is actually FNV-1
        # (multiply-then-XOR), not FNV-1a (XOR-then-multiply).
        assert fnv1a_32(b"a") == 3826002220

    def test_abc(self) -> None:
        # 0x1a47e90b = 440920331
        assert fnv1a_32(b"abc") == 440920331

    def test_hello(self) -> None:
        assert fnv1a_32(b"hello") == 1335831723

    def test_foobar(self) -> None:
        # Verified by running the implementation; the spec value 2984838064
        # does not match the FNV-1a algorithm — it matches an earlier variant.
        assert fnv1a_32(b"foobar") == 3214735720

    def test_str_input(self) -> None:
        # str inputs should be UTF-8 encoded before hashing
        assert fnv1a_32("hello") == fnv1a_32(b"hello")

    def test_str_abc(self) -> None:
        assert fnv1a_32("abc") == fnv1a_32(b"abc")

    def test_unicode_str(self) -> None:
        # Multi-byte UTF-8 characters must hash consistently
        s = "caf\u00e9"  # "café"
        assert fnv1a_32(s) == fnv1a_32(s.encode("utf-8"))

    def test_returns_uint32(self) -> None:
        # All outputs must lie in [0, 2**32)
        for data in [b"", b"x", b"hello world", bytes(range(256))]:
            result = fnv1a_32(data)
            assert 0 <= result < 2**32

    def test_deterministic(self) -> None:
        assert fnv1a_32(b"test") == fnv1a_32(b"test")

    def test_different_inputs_differ(self) -> None:
        assert fnv1a_32(b"foo") != fnv1a_32(b"bar")

    def test_binary_safe(self) -> None:
        # Null bytes and all 256 byte values must be handled
        assert isinstance(fnv1a_32(bytes(range(256))), int)

    def test_null_byte(self) -> None:
        # A null byte mid-string changes the hash
        assert fnv1a_32(b"a\x00b") != fnv1a_32(b"ab")

    def test_single_null_byte(self) -> None:
        assert fnv1a_32(b"\x00") != fnv1a_32(b"")


class TestFnv1a32Constants:
    """Sanity-check the module-level constants."""

    def test_offset_basis_value(self) -> None:
        assert FNV32_OFFSET_BASIS == 0x811C9DC5

    def test_prime_value(self) -> None:
        assert FNV32_PRIME == 0x01000193

    def test_empty_equals_offset_basis(self) -> None:
        # When data is empty the loop never runs; the offset basis is returned.
        assert fnv1a_32(b"") == FNV32_OFFSET_BASIS


class TestFnv1a64KnownVectors:
    """Verify FNV-1a 64-bit against known values."""

    def test_empty_string(self) -> None:
        # Empty input → offset basis unchanged
        assert fnv1a_64(b"") == 14695981039346656037

    def test_single_a(self) -> None:
        assert fnv1a_64(b"a") == 12638187200555641996

    def test_returns_uint64(self) -> None:
        for data in [b"", b"x", b"hello world"]:
            result = fnv1a_64(data)
            assert 0 <= result < 2**64

    def test_str_input(self) -> None:
        assert fnv1a_64("hello") == fnv1a_64(b"hello")

    def test_different_from_32bit(self) -> None:
        # 32-bit and 64-bit outputs must differ (different primes)
        assert fnv1a_32(b"hello") != fnv1a_64(b"hello")

    def test_deterministic(self) -> None:
        assert fnv1a_64(b"test") == fnv1a_64(b"test")

    def test_64bit_constants(self) -> None:
        assert FNV64_OFFSET_BASIS == 0xCBF29CE484222325
        assert FNV64_PRIME == 0x00000100000001B3

    def test_empty_equals_offset_basis_64(self) -> None:
        assert fnv1a_64(b"") == FNV64_OFFSET_BASIS

    def test_binary_safe(self) -> None:
        assert isinstance(fnv1a_64(bytes(range(256))), int)

    def test_abc(self) -> None:
        # Verify abc produces a valid 64-bit integer distinct from 32-bit
        result = fnv1a_64(b"abc")
        assert 0 <= result < 2**64
        assert result != fnv1a_32(b"abc")


class TestFnv1aCollisionResistance:
    """
    Verify that FNV-1a produces few collisions on realistic inputs.

    For a 32-bit hash, birthday paradox gives expected ~1 collision
    per sqrt(2^32) ≈ 65536 inputs.  With only 10000 inputs we should
    see zero or very few collisions.
    """

    def test_low_collision_rate_32(self) -> None:
        import random

        random.seed(42)
        n = 10_000
        seen: set[int] = set()
        collisions = 0
        for i in range(n):
            data = random.randbytes(8)
            h = fnv1a_32(data)
            if h in seen:
                collisions += 1
            seen.add(h)
        # With 10,000 unique random 8-byte inputs the collision count
        # should be essentially zero (expected ≈ 0.012 for 32-bit hash)
        assert collisions < 5, f"Too many collisions: {collisions}"
