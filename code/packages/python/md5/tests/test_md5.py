"""Tests for the MD5 implementation.

Test vectors come from RFC 1321 Appendix A. Any correct MD5 implementation must
produce exactly these digests.

We also test the streaming API and edge cases like block boundaries, the little-
endian output format, and inputs across all 256 byte values.
"""

import pytest

from md5 import MD5, md5, md5_hex


# ─── RFC 1321 Test Vectors ────────────────────────────────────────────────────


class TestRFC1321Vectors:
    """Official test vectors from RFC 1321 Appendix A."""

    def test_empty_string(self) -> None:
        assert md5_hex(b"") == "d41d8cd98f00b204e9800998ecf8427e"

    def test_a(self) -> None:
        assert md5_hex(b"a") == "0cc175b9c0f1b6a831c399e269772661"

    def test_abc(self) -> None:
        assert md5_hex(b"abc") == "900150983cd24fb0d6963f7d28e17f72"

    def test_message_digest(self) -> None:
        assert md5_hex(b"message digest") == "f96b697d7cb7938d525a2f31aaf161d0"

    def test_lowercase_alphabet(self) -> None:
        assert md5_hex(b"abcdefghijklmnopqrstuvwxyz") == "c3fcd3d76192e4007dfb496cca67e13b"

    def test_alphanumeric(self) -> None:
        msg = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789"
        assert md5_hex(msg) == "d174ab98d277d9f5a5611c2c9f419d9f"

    def test_digits(self) -> None:
        msg = b"12345678901234567890123456789012345678901234567890123456789012345678901234567890"
        assert md5_hex(msg) == "57edf4a22be3c955ac49da2e2107b67a"


# ─── Output Format ────────────────────────────────────────────────────────────


class TestReturnType:
    """md5() returns exactly 16 bytes."""

    def test_returns_bytes(self) -> None:
        assert isinstance(md5(b"test"), bytes)

    def test_length_is_16(self) -> None:
        assert len(md5(b"")) == 16
        assert len(md5(b"hello world")) == 16
        assert len(md5(b"x" * 1000)) == 16

    def test_deterministic(self) -> None:
        assert md5(b"hello") == md5(b"hello")

    def test_avalanche_effect(self) -> None:
        h1 = md5(b"hello")
        h2 = md5(b"helo")
        assert h1 != h2
        xor_bytes = bytes(a ^ b for a, b in zip(h1, h2))
        bits_different = sum(bin(b).count("1") for b in xor_bytes)
        assert bits_different > 20

    def test_hex_length_is_32(self) -> None:
        assert len(md5_hex(b"")) == 32
        assert len(md5_hex(b"hello")) == 32

    def test_hex_is_lowercase(self) -> None:
        result = md5_hex(b"abc")
        assert result == result.lower()
        assert not any(c.isupper() for c in result)

    def test_hex_matches_digest_hex(self) -> None:
        for msg in [b"", b"abc", b"hello world"]:
            assert md5_hex(msg) == md5(msg).hex()


# ─── Little-Endian Output ─────────────────────────────────────────────────────


class TestLittleEndian:
    """MD5 outputs words in little-endian order — the #1 implementation gotcha."""

    def test_empty_known_le_structure(self) -> None:
        # MD5("") = d41d8cd98f00b204e9800998ecf8427e
        # This is 4 words: 0xD98CD81D, 0x04B2008F, 0x9809800E, 0x7E42F8EC
        # stored little-endian. The hex string reads left-to-right as bytes.
        result = md5(b"")
        assert result.hex() == "d41d8cd98f00b204e9800998ecf8427e"

    def test_output_is_16_bytes(self) -> None:
        result = md5(b"abc")
        assert len(result) == 16

    def test_not_big_endian(self) -> None:
        # If someone accidentally used big-endian packing, the result would
        # differ from the RFC vector. Verify we're not doing that.
        result = md5_hex(b"abc")
        assert result == "900150983cd24fb0d6963f7d28e17f72"
        # The big-endian (wrong) result would start with something different.
        assert not result.startswith("83099015")


# ─── Block Boundary Tests ─────────────────────────────────────────────────────
#
# MD5 processes 64-byte blocks. Same boundary conditions as SHA-1:
#   55 bytes: fits in one block
#   56 bytes: requires second block for padding
#   64 bytes: one data block + one full padding block


class TestBlockBoundaries:
    def test_55_bytes(self) -> None:
        result = md5(b"x" * 55)
        assert len(result) == 16
        assert result == md5(b"x" * 55)

    def test_56_bytes(self) -> None:
        assert len(md5(b"x" * 56)) == 16

    def test_55_and_56_differ(self) -> None:
        assert md5(b"x" * 55) != md5(b"x" * 56)

    def test_64_bytes(self) -> None:
        assert len(md5(b"x" * 64)) == 16

    def test_127_bytes(self) -> None:
        assert len(md5(b"x" * 127)) == 16

    def test_128_bytes(self) -> None:
        assert len(md5(b"x" * 128)) == 16

    def test_all_boundary_sizes_distinct(self) -> None:
        digests = [md5(b"x" * n) for n in [55, 56, 63, 64, 127, 128]]
        assert len(set(digests)) == 6


# ─── Edge Cases ───────────────────────────────────────────────────────────────


class TestEdgeCases:
    def test_single_null_byte(self) -> None:
        result = md5(b"\x00")
        assert len(result) == 16
        assert result != md5(b"")

    def test_single_ff_byte(self) -> None:
        assert len(md5(b"\xff")) == 16

    def test_all_byte_values(self) -> None:
        result = md5(bytes(range(256)))
        assert len(result) == 16

    def test_every_single_byte_unique(self) -> None:
        digests = {md5(bytes([i])) for i in range(256)}
        assert len(digests) == 256

    def test_utf8_text(self) -> None:
        text = "Hello, 世界!".encode("utf-8")
        assert len(md5(text)) == 16

    def test_binary_zeros(self) -> None:
        assert len(md5(b"\x00" * 1000)) == 16


# ─── Streaming API ────────────────────────────────────────────────────────────


class TestStreaming:
    def test_single_update_equals_oneshot(self) -> None:
        h = MD5()
        h.update(b"abc")
        assert h.digest() == md5(b"abc")

    def test_split_at_byte_boundary(self) -> None:
        h = MD5()
        h.update(b"ab")
        h.update(b"c")
        assert h.digest() == md5(b"abc")

    def test_split_at_block_boundary(self) -> None:
        data = b"x" * 128
        h = MD5()
        h.update(data[:64])
        h.update(data[64:])
        assert h.digest() == md5(data)

    def test_many_tiny_updates(self) -> None:
        data = bytes(range(100))
        h = MD5()
        for byte in data:
            h.update(bytes([byte]))
        assert h.digest() == md5(data)

    def test_empty_input(self) -> None:
        h = MD5()
        assert h.digest() == md5(b"")

    def test_digest_is_nondestructive(self) -> None:
        h = MD5()
        h.update(b"abc")
        d1 = h.digest()
        d2 = h.digest()
        assert d1 == d2

    def test_update_after_digest(self) -> None:
        h = MD5()
        h.update(b"ab")
        h.digest()  # snapshot
        h.update(b"c")
        assert h.digest() == md5(b"abc")

    def test_hexdigest(self) -> None:
        h = MD5()
        h.update(b"abc")
        assert h.hexdigest() == "900150983cd24fb0d6963f7d28e17f72"

    def test_chained_updates(self) -> None:
        result = MD5().update(b"a").update(b"b").update(b"c").hexdigest()
        assert result == md5_hex(b"abc")

    def test_copy_is_independent(self) -> None:
        h = MD5()
        h.update(b"ab")
        h2 = h.copy()
        h2.update(b"c")
        h.update(b"x")
        assert h2.digest() == md5(b"abc")
        assert h.digest() == md5(b"abx")

    def test_copy_same_result(self) -> None:
        h = MD5()
        h.update(b"abc")
        h2 = h.copy()
        assert h.digest() == h2.digest()

    def test_streaming_rfc_vectors(self) -> None:
        for data, expected in [
            (b"", "d41d8cd98f00b204e9800998ecf8427e"),
            (b"a", "0cc175b9c0f1b6a831c399e269772661"),
            (b"abc", "900150983cd24fb0d6963f7d28e17f72"),
        ]:
            h = MD5()
            h.update(data)
            assert h.hexdigest() == expected
