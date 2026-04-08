"""
Tests for MurmurHash3 (32-bit variant).

Reference test vectors come from Austin Appleby's original C implementation
at https://github.com/aappleby/smhasher
"""

import pytest

from hash_functions import murmur3_32


class TestMurmur3KnownVectors:
    """Official test vectors from the smhasher reference implementation."""

    def test_empty_seed0(self) -> None:
        # Empty input with seed 0 must produce 0.
        # The length XOR (h ^= 0) and fmix32 of seed 0 give 0.
        assert murmur3_32(b"", seed=0) == 0

    def test_empty_seed1(self) -> None:
        # With a non-zero seed the output is non-zero even for empty input.
        assert murmur3_32(b"", seed=1) == 0x514E28B7

    def test_single_a_seed0(self) -> None:
        # 'a' = 0x61; single-byte tail path.
        # Verified against the reference C implementation: 0x3c2569b2 = 1009084850
        # Note: the spec listed 0xE40C292C which is the FNV-1a value for b"a",
        # not the MurmurHash3 value.
        assert murmur3_32(b"a", seed=0) == 0x3C2569B2

    def test_abc_seed0(self) -> None:
        # "abc" is a 3-byte tail (no full blocks)
        assert murmur3_32(b"abc", seed=0) == 0xB3DD93FA

    def test_four_byte_block(self) -> None:
        # "abcd" is exactly one full 4-byte block, no tail
        result = murmur3_32(b"abcd", seed=0)
        assert isinstance(result, int)
        assert 0 <= result < 2**32

    def test_str_input(self) -> None:
        assert murmur3_32("abc") == murmur3_32(b"abc")

    def test_returns_uint32(self) -> None:
        for data in [b"", b"x", b"hello world", bytes(range(256))]:
            result = murmur3_32(data)
            assert 0 <= result < 2**32

    def test_deterministic(self) -> None:
        assert murmur3_32(b"test") == murmur3_32(b"test")

    def test_different_inputs_differ(self) -> None:
        assert murmur3_32(b"foo") != murmur3_32(b"bar")

    def test_seed_changes_output(self) -> None:
        assert murmur3_32(b"hello", seed=0) != murmur3_32(b"hello", seed=1)

    def test_binary_safe(self) -> None:
        assert isinstance(murmur3_32(bytes(range(256))), int)

    def test_null_bytes(self) -> None:
        assert murmur3_32(b"\x00") != murmur3_32(b"")
        assert murmur3_32(b"\x00\x00") != murmur3_32(b"\x00")

    def test_unicode_str(self) -> None:
        s = "caf\u00e9"
        assert murmur3_32(s) == murmur3_32(s.encode("utf-8"))


class TestMurmur3Lengths:
    """
    Verify all four tail-byte lengths: 0, 1, 2, 3.

    MurmurHash3 has special handling for the tail bytes that do not fill
    a complete 4-byte block.  This test class ensures all four code paths
    are exercised.
    """

    def test_length_0_tail(self) -> None:
        # 4 bytes → 1 full block, 0 tail bytes
        data = b"abcd"
        result = murmur3_32(data)
        assert 0 <= result < 2**32

    def test_length_1_tail(self) -> None:
        # 5 bytes → 1 full block, 1 tail byte
        data = b"abcde"
        result = murmur3_32(data)
        assert 0 <= result < 2**32

    def test_length_2_tail(self) -> None:
        # 6 bytes → 1 full block, 2 tail bytes
        data = b"abcdef"
        result = murmur3_32(data)
        assert 0 <= result < 2**32

    def test_length_3_tail(self) -> None:
        # 7 bytes → 1 full block, 3 tail bytes
        data = b"abcdefg"
        result = murmur3_32(data)
        assert 0 <= result < 2**32

    def test_multiple_blocks(self) -> None:
        # 16 bytes → 4 full blocks, no tail
        data = b"a" * 16
        result = murmur3_32(data)
        assert 0 <= result < 2**32

    def test_different_by_one_byte(self) -> None:
        # "abcd" and "abce" differ only in the last byte
        assert murmur3_32(b"abcd") != murmur3_32(b"abce")


class TestMurmur3Collision:
    """Spot-check collision rate for MurmurHash3."""

    def test_low_collision_rate(self) -> None:
        import random

        random.seed(0)
        n = 10_000
        seen: set[int] = set()
        collisions = 0
        for _ in range(n):
            data = random.randbytes(8)
            h = murmur3_32(data)
            if h in seen:
                collisions += 1
            seen.add(h)
        assert collisions < 10
