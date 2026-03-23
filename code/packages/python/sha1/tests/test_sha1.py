"""Tests for the SHA-1 implementation.

Test vectors come from FIPS 180-4 (the official SHA-1 standard). Any correct
SHA-1 implementation must produce exactly these digests for these inputs.

We also test the streaming API (SHA1 class) to verify it produces the same
results as the one-shot sha1() function, and test edge cases like empty
input, exact block boundaries, and very long inputs.
"""

import pytest

from ca_sha1 import SHA1, sha1, sha1_hex


# ─── FIPS 180-4 Test Vectors ─────────────────────────────────────────────────


class TestFIPSVectors:
    """Official test vectors from FIPS 180-4 §B."""

    def test_empty_string(self) -> None:
        """The empty string still hashes to a non-zero digest via padding."""
        result = sha1(b"")
        assert result.hex() == "da39a3ee5e6b4b0d3255bfef95601890afd80709"

    def test_abc(self) -> None:
        """'abc' — the canonical single-block test case."""
        result = sha1(b"abc")
        assert result.hex() == "a9993e364706816aba3e25717850c26c9cd0d89d"

    def test_448_bit_message(self) -> None:
        """56-byte (448-bit) input forces two blocks (padding overflows one block)."""
        msg = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        assert len(msg) == 56  # exactly 56 bytes
        result = sha1(msg)
        assert result.hex() == "84983e441c3bd26ebaae4aa1f95129e5e54670f1"

    def test_million_a(self) -> None:
        """One million 'a' characters — stress test for multi-block hashing."""
        result = sha1(b"a" * 1_000_000)
        assert result.hex() == "34aa973cd4c4daa4f61eeb2bdbad27316534016f"


# ─── Return Type and Format ───────────────────────────────────────────────────


class TestReturnType:
    """sha1() returns exactly 20 bytes."""

    def test_returns_bytes(self) -> None:
        assert isinstance(sha1(b"test"), bytes)

    def test_length_is_20(self) -> None:
        assert len(sha1(b"")) == 20
        assert len(sha1(b"hello world")) == 20
        assert len(sha1(b"x" * 1000)) == 20

    def test_deterministic(self) -> None:
        """Same input always produces same output."""
        assert sha1(b"hello") == sha1(b"hello")

    def test_avalanche_effect(self) -> None:
        """Changing one byte completely changes the output."""
        h1 = sha1(b"hello")
        h2 = sha1(b"helo")  # one character different
        assert h1 != h2
        # Statistically, about half the bits should differ
        xor_bytes = bytes(a ^ b for a, b in zip(h1, h2))
        bits_different = sum(bin(b).count("1") for b in xor_bytes)
        assert bits_different > 20  # at least 20 of 160 bits should differ


# ─── Block Boundary Tests ─────────────────────────────────────────────────────
#
# SHA-1 processes 64-byte blocks. Inputs near block boundaries are the most
# common source of bugs, because padding behaves differently:
#
#   55 bytes: fits in one block (55 + 1 + 8 = 64)
#   56 bytes: overflows into a second block (56 + 1 + 7 > 56, so two blocks)
#   64 bytes: the block plus a full padding block


class TestBlockBoundaries:
    def test_55_bytes(self) -> None:
        """55 bytes → exactly one block after padding."""
        result = sha1(b"x" * 55)
        assert len(result) == 20
        # Verify determinism
        assert result == sha1(b"x" * 55)

    def test_56_bytes(self) -> None:
        """56 bytes → requires a second block for padding."""
        result = sha1(b"x" * 56)
        assert len(result) == 20

    def test_55_and_56_differ(self) -> None:
        """55 bytes and 56 bytes must hash differently."""
        assert sha1(b"x" * 55) != sha1(b"x" * 56)

    def test_64_bytes(self) -> None:
        """64 bytes → one data block + one full padding block."""
        result = sha1(b"x" * 64)
        assert len(result) == 20

    def test_127_bytes(self) -> None:
        """127 bytes → two data blocks + one padding block."""
        result = sha1(b"x" * 127)
        assert len(result) == 20

    def test_128_bytes(self) -> None:
        """128 bytes → exactly two data blocks + one full padding block."""
        result = sha1(b"x" * 128)
        assert len(result) == 20

    def test_exact_bytes_differ(self) -> None:
        """Each boundary length produces a unique digest."""
        digests = [sha1(b"x" * n) for n in [55, 56, 63, 64, 127, 128]]
        assert len(set(digests)) == 6  # all distinct


# ─── Edge Cases ───────────────────────────────────────────────────────────────


class TestEdgeCases:
    def test_single_zero_byte(self) -> None:
        result = sha1(b"\x00")
        assert len(result) == 20
        assert result != sha1(b"")  # null byte ≠ empty string

    def test_single_ff_byte(self) -> None:
        result = sha1(b"\xff")
        assert len(result) == 20

    def test_all_byte_values(self) -> None:
        """Can hash all 256 possible byte values."""
        result = sha1(bytes(range(256)))
        assert len(result) == 20

    def test_utf8_text(self) -> None:
        """Works correctly on UTF-8 encoded text."""
        text = "Hello, 世界!".encode("utf-8")
        result = sha1(text)
        assert len(result) == 20

    def test_binary_zeros(self) -> None:
        """1000 zero bytes."""
        result = sha1(b"\x00" * 1000)
        assert len(result) == 20

    def test_different_single_bytes(self) -> None:
        """Every single-byte input should produce a unique digest."""
        digests = {sha1(bytes([i])) for i in range(256)}
        assert len(digests) == 256  # all distinct


# ─── sha1_hex ─────────────────────────────────────────────────────────────────


class TestSha1Hex:
    def test_returns_string(self) -> None:
        assert isinstance(sha1_hex(b""), str)

    def test_length_is_40(self) -> None:
        assert len(sha1_hex(b"")) == 40
        assert len(sha1_hex(b"hello")) == 40

    def test_lowercase(self) -> None:
        result = sha1_hex(b"abc")
        assert result == result.lower()
        # No uppercase letters
        assert not any(c.isupper() for c in result)

    def test_matches_digest_hex(self) -> None:
        """sha1_hex(data) == sha1(data).hex()"""
        for msg in [b"", b"abc", b"hello world"]:
            assert sha1_hex(msg) == sha1(msg).hex()

    def test_fips_vector(self) -> None:
        assert sha1_hex(b"abc") == "a9993e364706816aba3e25717850c26c9cd0d89d"


# ─── Streaming API (SHA1 class) ───────────────────────────────────────────────


class TestStreaming:
    """The SHA1 streaming hasher must produce the same results as sha1()."""

    def test_single_update_equals_oneshot(self) -> None:
        h = SHA1()
        h.update(b"abc")
        assert h.digest() == sha1(b"abc")

    def test_split_at_byte_boundary(self) -> None:
        """Splitting input should not affect the result."""
        h = SHA1()
        h.update(b"ab")
        h.update(b"c")
        assert h.digest() == sha1(b"abc")

    def test_split_at_block_boundary(self) -> None:
        """Splitting exactly at 64-byte block boundary."""
        data = b"x" * 128
        h = SHA1()
        h.update(data[:64])
        h.update(data[64:])
        assert h.digest() == sha1(data)

    def test_many_tiny_updates(self) -> None:
        """One byte at a time for 100 bytes."""
        data = bytes(range(100))
        h = SHA1()
        for byte in data:
            h.update(bytes([byte]))
        assert h.digest() == sha1(data)

    def test_empty_input(self) -> None:
        h = SHA1()
        assert h.digest() == sha1(b"")

    def test_digest_is_nondestructive(self) -> None:
        """Calling digest() twice returns the same bytes."""
        h = SHA1()
        h.update(b"abc")
        d1 = h.digest()
        d2 = h.digest()
        assert d1 == d2

    def test_update_after_digest(self) -> None:
        """Can continue updating after calling digest()."""
        h = SHA1()
        h.update(b"ab")
        _ = h.digest()  # snapshot
        h.update(b"c")
        assert h.digest() == sha1(b"abc")

    def test_hexdigest(self) -> None:
        h = SHA1()
        h.update(b"abc")
        assert h.hexdigest() == "a9993e364706816aba3e25717850c26c9cd0d89d"

    def test_copy_is_independent(self) -> None:
        """Modifying a copy does not affect the original."""
        h = SHA1()
        h.update(b"ab")
        h2 = h.copy()
        h2.update(b"c")
        h.update(b"x")  # different suffix on original
        assert h2.digest() == sha1(b"abc")
        assert h.digest() == sha1(b"abx")

    def test_copy_same_result(self) -> None:
        """A copy of a hasher produces the same digest as the original."""
        h = SHA1()
        h.update(b"abc")
        h2 = h.copy()
        assert h.digest() == h2.digest()

    def test_fips_vector_streaming(self) -> None:
        """FIPS vectors work correctly via the streaming interface."""
        h = SHA1()
        h.update(b"a" * 500_000)
        h.update(b"a" * 500_000)
        assert h.digest() == sha1(b"a" * 1_000_000)
