"""Tests for the SHA-512 implementation.

Test vectors come from FIPS 180-4 (the official SHA-2 standard).  Any correct
SHA-512 implementation must produce exactly these digests for these inputs.

We also test the streaming API (SHA512Hasher class) to verify it produces the
same results as the one-shot sha512() function, and test edge cases like empty
input, exact block boundaries, and very long inputs.
"""

import pytest

from coding_adventures_sha512 import SHA512Hasher, sha512, sha512_hex


# ---- FIPS 180-4 Test Vectors ----


class TestFIPSVectors:
    """Official test vectors from FIPS 180-4."""

    def test_empty_string(self) -> None:
        """The empty string still hashes to a non-zero digest via padding."""
        result = sha512(b"")
        assert result.hex() == (
            "cf83e1357eefb8bdf1542850d66d8007"
            "d620e4050b5715dc83f4a921d36ce9ce"
            "47d0d13c5d85f2b0ff8318d2877eec2f"
            "63b931bd47417a81a538327af927da3e"
        )

    def test_abc(self) -> None:
        """'abc' -- the canonical single-block test case."""
        result = sha512(b"abc")
        assert result.hex() == (
            "ddaf35a193617abacc417349ae204131"
            "12e6fa4e89a97ea20a9eeee64b55d39a"
            "2192992a274fc1a836ba3c23a3feebbd"
            "454d4423643ce80e2a9ac94fa54ca49f"
        )

    def test_896_bit_message(self) -> None:
        """112-byte (896-bit) input -- the long FIPS test vector.

        This input is exactly 112 bytes, which forces the padding to overflow
        into a second 128-byte block (since 112 + 1 + 16 > 128).
        """
        msg = (
            b"abcdefghbcdefghicdefghijdefghijk"
            b"efghijklfghijklmghijklmnhijklmno"
            b"ijklmnopjklmnopqklmnopqrlmnopqrs"
            b"mnopqrstnopqrstu"
        )
        assert len(msg) == 112
        result = sha512(msg)
        assert result.hex() == (
            "8e959b75dae313da8cf4f72814fc143f"
            "8f7779c6eb9f7fa17299aeadb6889018"
            "501d289e4900f7e4331b99dec4b5433a"
            "c7d329eeb6dd26545e96e55b874be909"
        )

    def test_million_a(self) -> None:
        """One million 'a' characters -- stress test for multi-block hashing."""
        result = sha512(b"a" * 1_000_000)
        assert result.hex() == (
            "e718483d0ce769644e2e42c7bc15b463"
            "8e1f98b13b2044285632a803afa973eb"
            "de0ff244877ea60a4cb0432ce577c31b"
            "eb009c5c2c49aa2e4eadb217ad8cc09b"
        )


# ---- Return Type and Format ----


class TestReturnType:
    """sha512() returns exactly 64 bytes."""

    def test_returns_bytes(self) -> None:
        assert isinstance(sha512(b"test"), bytes)

    def test_length_is_64(self) -> None:
        assert len(sha512(b"")) == 64
        assert len(sha512(b"hello world")) == 64
        assert len(sha512(b"x" * 1000)) == 64

    def test_deterministic(self) -> None:
        """Same input always produces same output."""
        assert sha512(b"hello") == sha512(b"hello")

    def test_avalanche_effect(self) -> None:
        """Changing one byte completely changes the output."""
        h1 = sha512(b"hello")
        h2 = sha512(b"helo")  # one character different
        assert h1 != h2
        # Statistically, about half the bits should differ
        xor_bytes = bytes(a ^ b for a, b in zip(h1, h2))
        bits_different = sum(bin(b).count("1") for b in xor_bytes)
        assert bits_different > 100  # at least 100 of 512 bits should differ


# ---- Block Boundary Tests ----
#
# SHA-512 processes 128-byte blocks.  Inputs near block boundaries are the
# most common source of bugs, because padding behaves differently:
#
#   111 bytes: fits in one block (111 + 1 + 16 = 128)
#   112 bytes: overflows into a second block (112 + 1 + 15 > 112)
#   128 bytes: one data block + one full padding block


class TestBlockBoundaries:
    def test_111_bytes(self) -> None:
        """111 bytes -> exactly one block after padding."""
        result = sha512(b"x" * 111)
        assert len(result) == 64
        assert result == sha512(b"x" * 111)

    def test_112_bytes(self) -> None:
        """112 bytes -> requires a second block for padding."""
        result = sha512(b"x" * 112)
        assert len(result) == 64

    def test_111_and_112_differ(self) -> None:
        """111 bytes and 112 bytes must hash differently."""
        assert sha512(b"x" * 111) != sha512(b"x" * 112)

    def test_128_bytes(self) -> None:
        """128 bytes -> one data block + one full padding block."""
        result = sha512(b"x" * 128)
        assert len(result) == 64

    def test_255_bytes(self) -> None:
        """255 bytes -> two data blocks + one padding block."""
        result = sha512(b"x" * 255)
        assert len(result) == 64

    def test_256_bytes(self) -> None:
        """256 bytes -> exactly two data blocks + one full padding block."""
        result = sha512(b"x" * 256)
        assert len(result) == 64

    def test_exact_bytes_differ(self) -> None:
        """Each boundary length produces a unique digest."""
        digests = [sha512(b"x" * n) for n in [111, 112, 127, 128, 255, 256]]
        assert len(set(digests)) == 6  # all distinct


# ---- Edge Cases ----


class TestEdgeCases:
    def test_single_zero_byte(self) -> None:
        result = sha512(b"\x00")
        assert len(result) == 64
        assert result != sha512(b"")  # null byte != empty string

    def test_single_ff_byte(self) -> None:
        result = sha512(b"\xff")
        assert len(result) == 64

    def test_all_byte_values(self) -> None:
        """Can hash all 256 possible byte values."""
        result = sha512(bytes(range(256)))
        assert len(result) == 64

    def test_utf8_text(self) -> None:
        """Works correctly on UTF-8 encoded text."""
        text = "Hello, 世界!".encode("utf-8")
        result = sha512(text)
        assert len(result) == 64

    def test_binary_zeros(self) -> None:
        """1000 zero bytes."""
        result = sha512(b"\x00" * 1000)
        assert len(result) == 64

    def test_different_single_bytes(self) -> None:
        """Every single-byte input should produce a unique digest."""
        digests = {sha512(bytes([i])) for i in range(256)}
        assert len(digests) == 256  # all distinct


# ---- sha512_hex ----


class TestSha512Hex:
    def test_returns_string(self) -> None:
        assert isinstance(sha512_hex(b""), str)

    def test_length_is_128(self) -> None:
        assert len(sha512_hex(b"")) == 128
        assert len(sha512_hex(b"hello")) == 128

    def test_lowercase(self) -> None:
        result = sha512_hex(b"abc")
        assert result == result.lower()
        assert not any(c.isupper() for c in result)

    def test_matches_digest_hex(self) -> None:
        """sha512_hex(data) == sha512(data).hex()"""
        for msg in [b"", b"abc", b"hello world"]:
            assert sha512_hex(msg) == sha512(msg).hex()

    def test_fips_vector(self) -> None:
        assert sha512_hex(b"abc") == (
            "ddaf35a193617abacc417349ae204131"
            "12e6fa4e89a97ea20a9eeee64b55d39a"
            "2192992a274fc1a836ba3c23a3feebbd"
            "454d4423643ce80e2a9ac94fa54ca49f"
        )


# ---- Streaming API (SHA512Hasher class) ----


class TestStreaming:
    """The SHA512Hasher streaming hasher must produce the same results as sha512()."""

    def test_single_update_equals_oneshot(self) -> None:
        h = SHA512Hasher()
        h.update(b"abc")
        assert h.digest() == sha512(b"abc")

    def test_split_at_byte_boundary(self) -> None:
        """Splitting input should not affect the result."""
        h = SHA512Hasher()
        h.update(b"ab")
        h.update(b"c")
        assert h.digest() == sha512(b"abc")

    def test_split_at_block_boundary(self) -> None:
        """Splitting exactly at 128-byte block boundary."""
        data = b"x" * 256
        h = SHA512Hasher()
        h.update(data[:128])
        h.update(data[128:])
        assert h.digest() == sha512(data)

    def test_many_tiny_updates(self) -> None:
        """One byte at a time for 200 bytes."""
        data = bytes(range(200))
        h = SHA512Hasher()
        for byte in data:
            h.update(bytes([byte]))
        assert h.digest() == sha512(data)

    def test_empty_input(self) -> None:
        h = SHA512Hasher()
        assert h.digest() == sha512(b"")

    def test_digest_is_nondestructive(self) -> None:
        """Calling digest() twice returns the same bytes."""
        h = SHA512Hasher()
        h.update(b"abc")
        d1 = h.digest()
        d2 = h.digest()
        assert d1 == d2

    def test_update_after_digest(self) -> None:
        """Can continue updating after calling digest()."""
        h = SHA512Hasher()
        h.update(b"ab")
        _ = h.digest()  # snapshot
        h.update(b"c")
        assert h.digest() == sha512(b"abc")

    def test_hex_digest(self) -> None:
        h = SHA512Hasher()
        h.update(b"abc")
        assert h.hex_digest() == sha512_hex(b"abc")

    def test_copy_is_independent(self) -> None:
        """Modifying a copy does not affect the original."""
        h = SHA512Hasher()
        h.update(b"ab")
        h2 = h.copy()
        h2.update(b"c")
        h.update(b"x")  # different suffix on original
        assert h2.digest() == sha512(b"abc")
        assert h.digest() == sha512(b"abx")

    def test_copy_same_result(self) -> None:
        """A copy of a hasher produces the same digest as the original."""
        h = SHA512Hasher()
        h.update(b"abc")
        h2 = h.copy()
        assert h.digest() == h2.digest()

    def test_fips_vector_streaming(self) -> None:
        """FIPS vectors work correctly via the streaming interface."""
        h = SHA512Hasher()
        h.update(b"a" * 500_000)
        h.update(b"a" * 500_000)
        assert h.digest() == sha512(b"a" * 1_000_000)

    def test_chaining(self) -> None:
        """update() returns self for method chaining."""
        h = SHA512Hasher()
        result = h.update(b"ab").update(b"c").hex_digest()
        assert result == sha512_hex(b"abc")
