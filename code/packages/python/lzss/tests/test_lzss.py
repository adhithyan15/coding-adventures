"""Tests for coding_adventures_lzss — LZSS compression algorithm (CMP02)."""

import struct

import pytest

from coding_adventures_lzss import (
    Literal,
    Match,
    compress,
    decode,
    decompress,
    encode,
)

# ─── Helpers ──────────────────────────────────────────────────────────────────


def rt(data: bytes) -> bytes:
    """Round-trip helper: compress then decompress."""
    return decompress(compress(data))


# ─── Spec vectors ─────────────────────────────────────────────────────────────


class TestSpecVectors:
    """Test vectors from the CMP02 LZSS spec."""

    def test_empty_input(self) -> None:
        assert encode(b"") == []

    def test_single_byte(self) -> None:
        assert encode(b"A") == [Literal(65)]

    def test_no_repetition(self) -> None:
        tokens = encode(b"ABCDE")
        assert tokens == [
            Literal(65), Literal(66), Literal(67), Literal(68), Literal(69)
        ]

    def test_all_literals_have_no_matches(self) -> None:
        tokens = encode(b"ABCDE")
        assert all(isinstance(t, Literal) for t in tokens)

    def test_aabcbbabc(self) -> None:
        # "ABC" at position 6 matches "ABC" at position 1 (5 bytes back).
        tokens = encode(b"AABCBBABC")
        assert tokens == [
            Literal(65),
            Literal(65),
            Literal(66),
            Literal(67),
            Literal(66),
            Literal(66),
            Match(offset=5, length=3),
        ]

    def test_ababab(self) -> None:
        tokens = encode(b"ABABAB")
        assert tokens == [
            Literal(65),
            Literal(66),
            Match(offset=2, length=4),
        ]

    def test_all_identical(self) -> None:
        tokens = encode(b"AAAAAAA")
        assert tokens == [
            Literal(65),
            Match(offset=1, length=6),
        ]

    def test_repeated_abc(self) -> None:
        # After "ABC" is in the window, each repetition should be a match.
        tokens = encode(b"ABCABCABC")
        matches = [t for t in tokens if isinstance(t, Match)]
        assert len(matches) >= 1
        assert all(t.length >= 3 for t in matches)


# ─── Encode properties ────────────────────────────────────────────────────────


class TestEncodeProperties:
    """Invariants that must hold for all encoded output."""

    def test_match_offset_positive(self) -> None:
        tokens = encode(b"ABABABAB")
        for t in tokens:
            if isinstance(t, Match):
                assert t.offset >= 1

    def test_match_length_ge_min_match(self) -> None:
        tokens = encode(b"ABABABABABABABAB")
        for t in tokens:
            if isinstance(t, Match):
                assert t.length >= 3

    def test_match_offset_within_window(self) -> None:
        data = bytes(range(256)) * 4  # 1024 bytes
        tokens = encode(data, window_size=512)
        for t in tokens:
            if isinstance(t, Match):
                assert t.offset <= 512

    def test_match_length_within_max(self) -> None:
        tokens = encode(b"A" * 1000, max_match=10)
        for t in tokens:
            if isinstance(t, Match):
                assert t.length <= 10

    def test_literal_byte_range(self) -> None:
        tokens = encode(bytes(range(256)))
        for t in tokens:
            if isinstance(t, Literal):
                assert 0 <= t.byte <= 255


# ─── Decode ───────────────────────────────────────────────────────────────────


class TestDecode:
    def test_decode_empty(self) -> None:
        assert decode([], original_length=0) == b""

    def test_decode_single_literal(self) -> None:
        assert decode([Literal(65)], original_length=1) == b"A"

    def test_decode_literals_only(self) -> None:
        tokens = [Literal(65), Literal(66), Literal(67)]
        assert decode(tokens, original_length=3) == b"ABC"

    def test_decode_match_no_overlap(self) -> None:
        # "ABC" then Match(offset=3, length=3) → "ABCABC"
        tokens = [Literal(65), Literal(66), Literal(67), Match(offset=3, length=3)]
        assert decode(tokens, original_length=6) == b"ABCABC"

    def test_decode_overlapping_match(self) -> None:
        # "A" then Match(offset=1, length=6) → "AAAAAAA"
        tokens = [Literal(65), Match(offset=1, length=6)]
        assert decode(tokens, original_length=7) == b"AAAAAAA"

    def test_decode_ababab(self) -> None:
        tokens = [Literal(65), Literal(66), Match(offset=2, length=4)]
        assert decode(tokens, original_length=6) == b"ABABAB"

    def test_decode_truncates_to_original_length(self) -> None:
        tokens = [Literal(65), Literal(66), Literal(67)]
        assert decode(tokens, original_length=2) == b"AB"

    def test_decode_no_length_returns_all(self) -> None:
        tokens = [Literal(65), Literal(66)]
        assert decode(tokens) == b"AB"


# ─── Round-trip ───────────────────────────────────────────────────────────────


class TestRoundTrip:
    def test_empty(self) -> None:
        assert rt(b"") == b""

    def test_single_byte(self) -> None:
        assert rt(b"A") == b"A"

    def test_no_repetition(self) -> None:
        assert rt(b"ABCDE") == b"ABCDE"

    def test_all_identical(self) -> None:
        assert rt(b"AAAAAAA") == b"AAAAAAA"

    def test_ababab(self) -> None:
        assert rt(b"ABABAB") == b"ABABAB"

    def test_aabcbbabc(self) -> None:
        assert rt(b"AABCBBABC") == b"AABCBBABC"

    def test_hello_world(self) -> None:
        assert rt(b"hello world") == b"hello world"

    def test_repeated_abc(self) -> None:
        assert rt(b"ABC" * 100) == b"ABC" * 100

    def test_binary_nulls(self) -> None:
        data = bytes([0, 0, 0, 255, 255])
        assert rt(data) == data

    def test_full_byte_range(self) -> None:
        data = bytes(range(256))
        assert rt(data) == data

    def test_repeated_pattern(self) -> None:
        data = bytes([0, 1, 2] * 100)
        assert rt(data) == data

    def test_long_repetitive(self) -> None:
        data = b"ABCDEF" * 500
        assert rt(data) == data

    def test_all_zeros(self) -> None:
        data = b"\x00" * 1000
        assert rt(data) == data

    def test_all_same_byte(self) -> None:
        data = b"\xff" * 10000
        assert rt(data) == data

    @pytest.mark.parametrize("n", [1, 10, 100, 1000])
    def test_various_lengths(self, n: int) -> None:
        data = bytes(i % 256 for i in range(n))
        assert rt(data) == data


# ─── Parameters ───────────────────────────────────────────────────────────────


class TestParameters:
    def test_window_size_limits_offset(self) -> None:
        data = b"ABCABC" * 100
        tokens = encode(data, window_size=4)
        for t in tokens:
            if isinstance(t, Match):
                assert t.offset <= 4

    def test_max_match_limits_length(self) -> None:
        tokens = encode(b"A" * 100, max_match=5)
        for t in tokens:
            if isinstance(t, Match):
                assert t.length <= 5

    def test_min_match_large_forces_all_literals(self) -> None:
        tokens = encode(b"ABABAB", min_match=100)
        assert all(isinstance(t, Literal) for t in tokens)


# ─── Wire format ──────────────────────────────────────────────────────────────


class TestWireFormat:
    def test_compress_header_size(self) -> None:
        result = compress(b"")
        assert len(result) >= 8
        assert result[:4] == b"\x00\x00\x00\x00"  # original_length = 0

    def test_compress_stores_original_length(self) -> None:
        data = b"hello"
        compressed = compress(data)
        stored_len = struct.unpack(">I", compressed[:4])[0]
        assert stored_len == 5

    def test_compress_deterministic(self) -> None:
        data = b"hello world test"
        assert compress(data) == compress(data)

    def test_compress_empty_produces_header_only(self) -> None:
        compressed = compress(b"")
        orig_len, block_count = struct.unpack(">II", compressed[:8])
        assert orig_len == 0
        assert block_count == 0
        assert len(compressed) == 8

    def test_blocks_size_correct(self) -> None:
        # 8 distinct bytes → 8 literals, 1 block = 1 (flag) + 8 (bytes) = 9 bytes
        data = b"ABCDEFGH"
        compressed = compress(data)
        _, block_count = struct.unpack(">II", compressed[:8])
        assert block_count == 1
        assert len(compressed) == 8 + 9  # header + 1 block

    def test_crafted_large_block_count_is_safe(self) -> None:
        # Craft a header claiming 2^30 blocks over only a few bytes of data.
        bad_header = struct.pack(">II", 4, 2**30)
        payload = bad_header + b"\x00ABCD"  # 1 real block, flag=0 (4 literals)
        result = decompress(payload)
        assert isinstance(result, bytes)


# ─── Compression effectiveness ────────────────────────────────────────────────


class TestCompressionEffectiveness:
    def test_repetitive_data_compresses(self) -> None:
        data = b"ABC" * 1000
        assert len(compress(data)) < len(data)

    def test_all_same_byte_compresses(self) -> None:
        data = b"\x42" * 10000
        compressed = compress(data)
        assert len(compressed) < len(data)
        assert decompress(compressed) == data

    def test_lzss_much_smaller_than_raw_on_repetitive(self) -> None:
        # LZSS should compress "ABCDEF" * 500 to well under half its size.
        data = b"ABCDEF" * 500  # 3000 bytes
        compressed = compress(data)
        assert len(compressed) < len(data) // 2
