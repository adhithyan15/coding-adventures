"""Tests for the SHA-256 implementation.

Test vectors come from FIPS 180-4 (the official SHA-2 standard). Any correct
SHA-256 implementation must produce exactly these digests for these inputs.

We also test the streaming API (SHA256Hasher class) to verify it produces the
same results as the one-shot sha256() function, and test edge cases like empty
input, exact block boundaries, and very long inputs.
"""

import pytest

from coding_adventures_sha256 import SHA256Hasher, sha256, sha256_hex


# === FIPS 180-4 Test Vectors =================================================


class TestFIPSVectors:
    """Official test vectors from FIPS 180-4."""

    def test_empty_string(self) -> None:
        """The empty string still hashes to a non-zero digest via padding."""
        result = sha256(b"")
        assert (
            result.hex()
            == "e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855"
        )

    def test_abc(self) -> None:
        """'abc' -- the canonical single-block test case."""
        result = sha256(b"abc")
        assert (
            result.hex()
            == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )

    def test_448_bit_message(self) -> None:
        """56-byte input that forces two blocks (padding overflows one block)."""
        msg = b"abcdbcdecdefdefgefghfghighijhijkijkljklmklmnlmnomnopnopq"
        assert len(msg) == 56
        result = sha256(msg)
        assert (
            result.hex()
            == "248d6a61d20638b8e5c026930c3e6039a33ce45964ff2167f6ecedd419db06c1"
        )

    def test_million_a(self) -> None:
        """One million 'a' characters -- stress test for multi-block hashing."""
        result = sha256(b"a" * 1_000_000)
        assert (
            result.hex()
            == "cdc76e5c9914fb9281a1c7e284d73e67f1809a48a497200e046d39ccc7112cd0"
        )


# === Return Type and Format ===================================================


class TestReturnType:
    """sha256() returns exactly 32 bytes."""

    def test_returns_bytes(self) -> None:
        assert isinstance(sha256(b"test"), bytes)

    def test_length_is_32(self) -> None:
        assert len(sha256(b"")) == 32
        assert len(sha256(b"hello world")) == 32
        assert len(sha256(b"x" * 1000)) == 32

    def test_deterministic(self) -> None:
        """Same input always produces same output."""
        assert sha256(b"hello") == sha256(b"hello")

    def test_avalanche_effect(self) -> None:
        """Changing one byte completely changes the output.

        The avalanche effect states that flipping a single input bit should
        cause approximately 50% of the output bits to flip. We test that
        at least 20% flip (conservative threshold to avoid flaky tests).
        """
        h1 = sha256(b"hello")
        h2 = sha256(b"helo")  # one character different
        assert h1 != h2
        # Count differing bits
        xor_bytes = bytes(a ^ b for a, b in zip(h1, h2))
        bits_different = sum(bin(b).count("1") for b in xor_bytes)
        # 256 bits total, expect ~128 different, require at least 50
        assert bits_different > 50


# === Block Boundary Tests =====================================================
#
# SHA-256 processes 64-byte blocks. Inputs near block boundaries are the most
# common source of bugs, because padding behaves differently:
#
#   55 bytes: fits in one block    (55 + 1 + 8 = 64)
#   56 bytes: overflows to 2 blocks (56 + 1 + 7 > 56, needs second block)
#   63 bytes: nearly fills a block (63 + 1 = 64, but no room for 8-byte length)
#   64 bytes: one full data block + one padding block
#   119 bytes: nearly fills 2 blocks (119 + 1 + 8 > 120...)
#   120 bytes: edge case
#   127 bytes: almost 2 full blocks
#   128 bytes: exactly 2 data blocks + 1 padding block


class TestBlockBoundaries:
    def test_55_bytes(self) -> None:
        """55 bytes -> exactly one block after padding."""
        result = sha256(b"x" * 55)
        assert len(result) == 32
        assert result == sha256(b"x" * 55)

    def test_56_bytes(self) -> None:
        """56 bytes -> requires a second block for padding."""
        result = sha256(b"x" * 56)
        assert len(result) == 32

    def test_63_bytes(self) -> None:
        """63 bytes -> nearly fills one block, padding overflows."""
        result = sha256(b"x" * 63)
        assert len(result) == 32

    def test_64_bytes(self) -> None:
        """64 bytes -> one data block + one full padding block."""
        result = sha256(b"x" * 64)
        assert len(result) == 32

    def test_119_bytes(self) -> None:
        """119 bytes -> nearly two blocks, padding edge case."""
        result = sha256(b"x" * 119)
        assert len(result) == 32

    def test_120_bytes(self) -> None:
        """120 bytes -> padding just overflows to third block."""
        result = sha256(b"x" * 120)
        assert len(result) == 32

    def test_127_bytes(self) -> None:
        """127 bytes -> almost exactly two full blocks."""
        result = sha256(b"x" * 127)
        assert len(result) == 32

    def test_128_bytes(self) -> None:
        """128 bytes -> exactly two data blocks + one full padding block."""
        result = sha256(b"x" * 128)
        assert len(result) == 32

    def test_boundary_lengths_all_distinct(self) -> None:
        """Each boundary length must produce a unique digest."""
        lengths = [55, 56, 63, 64, 119, 120, 127, 128]
        digests = [sha256(b"x" * n) for n in lengths]
        assert len(set(digests)) == len(lengths)


# === Edge Cases ===============================================================


class TestEdgeCases:
    def test_single_zero_byte(self) -> None:
        result = sha256(b"\x00")
        assert len(result) == 32
        assert result != sha256(b"")  # null byte != empty string

    def test_single_ff_byte(self) -> None:
        result = sha256(b"\xff")
        assert len(result) == 32

    def test_all_byte_values(self) -> None:
        """Can hash all 256 possible byte values in sequence."""
        result = sha256(bytes(range(256)))
        assert len(result) == 32

    def test_utf8_text(self) -> None:
        """Works correctly on UTF-8 encoded text."""
        text = "Hello, \u4e16\u754c!".encode("utf-8")
        result = sha256(text)
        assert len(result) == 32

    def test_binary_zeros(self) -> None:
        """1000 zero bytes."""
        result = sha256(b"\x00" * 1000)
        assert len(result) == 32

    def test_different_single_bytes(self) -> None:
        """Every single-byte input should produce a unique digest."""
        digests = {sha256(bytes([i])) for i in range(256)}
        assert len(digests) == 256


# === sha256_hex ===============================================================


class TestSha256Hex:
    def test_returns_string(self) -> None:
        assert isinstance(sha256_hex(b""), str)

    def test_length_is_64(self) -> None:
        assert len(sha256_hex(b"")) == 64
        assert len(sha256_hex(b"hello")) == 64

    def test_lowercase(self) -> None:
        result = sha256_hex(b"abc")
        assert result == result.lower()
        assert not any(c.isupper() for c in result)

    def test_matches_digest_hex(self) -> None:
        """sha256_hex(data) == sha256(data).hex()"""
        for msg in [b"", b"abc", b"hello world"]:
            assert sha256_hex(msg) == sha256(msg).hex()

    def test_fips_vector(self) -> None:
        assert (
            sha256_hex(b"abc")
            == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )


# === Streaming API (SHA256Hasher class) =======================================


class TestStreaming:
    """The SHA256Hasher streaming hasher must match sha256() one-shot results."""

    def test_single_update_equals_oneshot(self) -> None:
        h = SHA256Hasher()
        h.update(b"abc")
        assert h.digest() == sha256(b"abc")

    def test_split_at_byte_boundary(self) -> None:
        """Splitting input should not affect the result."""
        h = SHA256Hasher()
        h.update(b"ab")
        h.update(b"c")
        assert h.digest() == sha256(b"abc")

    def test_split_at_block_boundary(self) -> None:
        """Splitting exactly at 64-byte block boundary."""
        data = b"x" * 128
        h = SHA256Hasher()
        h.update(data[:64])
        h.update(data[64:])
        assert h.digest() == sha256(data)

    def test_many_tiny_updates(self) -> None:
        """One byte at a time for 100 bytes."""
        data = bytes(range(100))
        h = SHA256Hasher()
        for byte_val in data:
            h.update(bytes([byte_val]))
        assert h.digest() == sha256(data)

    def test_empty_input(self) -> None:
        h = SHA256Hasher()
        assert h.digest() == sha256(b"")

    def test_digest_is_nondestructive(self) -> None:
        """Calling digest() twice returns the same bytes."""
        h = SHA256Hasher()
        h.update(b"abc")
        d1 = h.digest()
        d2 = h.digest()
        assert d1 == d2

    def test_update_after_digest(self) -> None:
        """Can continue updating after calling digest()."""
        h = SHA256Hasher()
        h.update(b"ab")
        _ = h.digest()  # snapshot
        h.update(b"c")
        assert h.digest() == sha256(b"abc")

    def test_hex_digest(self) -> None:
        h = SHA256Hasher()
        h.update(b"abc")
        assert (
            h.hex_digest()
            == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )

    def test_chaining(self) -> None:
        """update() returns self for method chaining."""
        h = SHA256Hasher()
        result = h.update(b"a").update(b"b").update(b"c").hex_digest()
        assert (
            result
            == "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        )

    def test_copy_is_independent(self) -> None:
        """Modifying a copy does not affect the original."""
        h = SHA256Hasher()
        h.update(b"ab")
        h2 = h.copy()
        h2.update(b"c")
        h.update(b"x")  # different suffix on original
        assert h2.digest() == sha256(b"abc")
        assert h.digest() == sha256(b"abx")

    def test_copy_same_result(self) -> None:
        """A copy of a hasher produces the same digest as the original."""
        h = SHA256Hasher()
        h.update(b"abc")
        h2 = h.copy()
        assert h.digest() == h2.digest()

    def test_fips_vector_streaming(self) -> None:
        """FIPS vectors work correctly via the streaming interface."""
        h = SHA256Hasher()
        h.update(b"a" * 500_000)
        h.update(b"a" * 500_000)
        assert h.digest() == sha256(b"a" * 1_000_000)

    def test_streaming_various_chunk_sizes(self) -> None:
        """Varying chunk sizes all produce the same hash."""
        data = b"a" * 200
        expected = sha256(data)
        for chunk_size in [1, 7, 13, 32, 63, 64, 65, 100, 200]:
            h = SHA256Hasher()
            for i in range(0, len(data), chunk_size):
                h.update(data[i : i + chunk_size])
            assert h.digest() == expected, f"Failed with chunk_size={chunk_size}"
