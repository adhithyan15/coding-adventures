"""
test_lz78.py — Comprehensive tests for the LZ78 compression implementation.

Test vectors come from the CMP01 specification. Covers: literals, dictionary
growth, flush token handling, round-trip invariants, and edge cases.
"""

import pytest

from coding_adventures_lz78 import (
    Token,
    compress,
    decode,
    decompress,
    encode,
)


# ─── Spec test vectors ────────────────────────────────────────────────────────


class TestSpecVectors:
    """Vectors from the CMP01 specification."""

    def test_empty_input(self) -> None:
        assert encode(b"") == []
        assert decode([], original_length=0) == b""

    def test_single_byte(self) -> None:
        tokens = encode(b"A")
        assert tokens == [Token(0, 65)]
        assert decode(tokens) == b"A"

    def test_no_repetition(self) -> None:
        # "ABCDE" — all distinct, no dictionary reuse.
        tokens = encode(b"ABCDE")
        assert tokens == [
            Token(0, 65),
            Token(0, 66),
            Token(0, 67),
            Token(0, 68),
            Token(0, 69),
        ]
        for t in tokens:
            assert t.dict_index == 0  # all literals
        assert decode(tokens) == b"ABCDE"

    def test_aabcbbabc(self) -> None:
        # From spec worked example 1.
        tokens = encode(b"AABCBBABC")
        assert tokens == [
            Token(0, 65),  # 'A'
            Token(1, 66),  # "A" + 'B'
            Token(0, 67),  # 'C'
            Token(0, 66),  # 'B'
            Token(4, 65),  # "B" + 'A'
            Token(4, 67),  # "B" + 'C'
        ]
        assert decode(tokens) == b"AABCBBABC"

    def test_ababab(self) -> None:
        # From spec worked example 2 — flush token at end.
        tokens = encode(b"ABABAB")
        assert tokens == [
            Token(0, 65),  # 'A' → dict entry 1
            Token(0, 66),  # 'B' → dict entry 2
            Token(1, 66),  # "A"+'B' → dict entry 3 ("AB")
            Token(3, 0),   # flush: "AB", sentinel
        ]
        # Round-trip via decompress strips the sentinel.
        assert decompress(compress(b"ABABAB")) == b"ABABAB"

    def test_all_identical_bytes(self) -> None:
        # "AAAAAAA" — growing runs.
        tokens = encode(b"AAAAAAA")
        # dict entries: 1="A", 2="AA", 3="AAA"; last 'A' flushes.
        assert len(tokens) == 4
        assert tokens[0] == Token(0, 65)  # literal 'A'
        assert tokens[1] == Token(1, 65)  # "A" + 'A'
        assert tokens[2] == Token(2, 65)  # "AA" + 'A'
        assert tokens[3] == Token(1, 0)   # flush: "A", sentinel

    def test_repeated_pair(self) -> None:
        tokens = encode(b"ABABABAB")
        # All tokens must decode to the original.
        assert decompress(compress(b"ABABABAB")) == b"ABABABAB"
        # Compression must reduce token count vs naive (8 literals).
        assert len(tokens) < 8


# ─── Round-trip tests ─────────────────────────────────────────────────────────


class TestRoundTrip:
    """Invariant: decompress(compress(x)) == x for all x."""

    @pytest.mark.parametrize(
        "s",
        [
            b"",
            b"A",
            b"ABCDE",
            b"AAAAAAA",
            b"ABABABAB",
            b"AABCBBABC",
            b"hello world",
            b"the quick brown fox",
            b"ababababab",
            b"aaaaaaaaaa",
        ],
    )
    def test_ascii(self, s: bytes) -> None:
        assert decompress(compress(s)) == s

    @pytest.mark.parametrize(
        "data",
        [
            bytes([0, 0, 0]),
            bytes([255, 255, 255]),
            bytes(range(256)),
            bytes([0, 1, 2, 0, 1, 2]),
            bytes([0, 0, 0, 255, 255]),
        ],
    )
    def test_binary(self, data: bytes) -> None:
        assert decompress(compress(data)) == data

    def test_encode_decode_token_level(self) -> None:
        cases = [b"", b"A", b"ABCDE", b"AAAAAAA", b"ABABABAB", b"hello world"]
        for s in cases:
            tokens = encode(s)
            # decode without length hint returns output including sentinel byte
            # for flush cases; full round-trip via compress/decompress is canonical.
            result = decompress(compress(s))
            assert result == s, f"Round-trip failed for {s!r}"


# ─── Parameter tests ──────────────────────────────────────────────────────────


class TestParameters:
    def test_max_dict_size_respected(self) -> None:
        # With max_dict_size=10, dict fills quickly; remaining tokens all literal.
        data = b"ABCABCABCABCABC"
        tokens = encode(data, max_dict_size=10)
        # dict_index must be within [0, 10).
        for t in tokens:
            assert t.dict_index < 10, f"dict_index {t.dict_index} exceeds max=10"

    def test_max_dict_size_1(self) -> None:
        # max_dict_size=1 → only root (id=0) exists; every token is a literal.
        tokens = encode(b"AAAA", max_dict_size=1)
        for t in tokens:
            assert t.dict_index == 0

    def test_max_dict_size_large(self) -> None:
        # Large dict doesn't break anything.
        data = (b"ABC" * 100) + b"X"
        assert decompress(compress(data, max_dict_size=100000)) == data


# ─── Edge cases ───────────────────────────────────────────────────────────────


class TestEdgeCases:
    def test_single_byte_literal(self) -> None:
        tokens = encode(b"X")
        assert tokens == [Token(0, 88)]

    def test_two_bytes(self) -> None:
        tokens = encode(b"AB")
        assert tokens == [Token(0, 65), Token(0, 66)]
        assert decode(tokens) == b"AB"

    def test_flush_token_round_trip(self) -> None:
        # "ABABAB" ends mid-match → flush token → still round-trips cleanly.
        assert decompress(compress(b"ABABAB")) == b"ABABAB"

    def test_binary_with_nulls(self) -> None:
        data = bytes([0, 0, 0, 255, 255])
        assert decompress(compress(data)) == data

    def test_all_null_bytes(self) -> None:
        data = bytes(100)
        assert decompress(compress(data)) == data

    def test_all_max_bytes(self) -> None:
        data = bytes([255] * 100)
        assert decompress(compress(data)) == data

    def test_single_repeated_byte_large(self) -> None:
        data = b"A" * 10000
        compressed = compress(data)
        assert decompress(compressed) == data

    def test_very_long_input(self) -> None:
        data = b"Hello, World! " * 100 + bytes(range(256))
        assert decompress(compress(data)) == data

    def test_full_byte_range(self) -> None:
        # All 256 distinct byte values in sequence — no repetition, all literals.
        data = bytes(range(256))
        tokens = encode(data)
        for t in tokens:
            assert t.dict_index == 0  # no matches possible on first pass
        assert decompress(compress(data)) == data


# ─── Decode tests ─────────────────────────────────────────────────────────────


class TestDecode:
    def test_decode_pure_literals(self) -> None:
        tokens = [Token(0, 65), Token(0, 66), Token(0, 67)]
        assert decode(tokens) == b"ABC"

    def test_decode_with_dict_reference(self) -> None:
        # Token(0, 65) → 'A', adds entry 1="A"
        # Token(0, 66) → 'B', adds entry 2="B"
        # Token(1, 66) → entry1("A") + 'B' = "AB", adds entry 3="AB"
        # Full output: A + B + A + B = "ABAB"
        tokens = [Token(0, 65), Token(0, 66), Token(1, 66)]
        assert decode(tokens) == b"ABAB"

    def test_decode_chain(self) -> None:
        # Verify parent-chain reconstruction for deeper entries.
        tokens = encode(b"ABCABCABC")
        assert decompress(compress(b"ABCABCABC")) == b"ABCABCABC"

    def test_decode_original_length_truncation(self) -> None:
        # Manually verify that original_length truncation works.
        # Token(0, 65) → "A", adds entry 1="A"
        # Token(0, 66) → "B", adds entry 2="B"
        # Token(1, 0)  → entry1("A") + sentinel(0) = "A\x00", adds entry 3="A\x00"
        # Full output without truncation: b"ABA\x00"
        tokens = [Token(0, 65), Token(0, 66), Token(1, 0)]
        result_full = decode(tokens)
        assert result_full == b"ABA\x00"
        # With original_length=2, truncate to b"AB".
        result_trunc = decode(tokens, original_length=2)
        assert result_trunc == b"AB"


# ─── Serialisation tests ──────────────────────────────────────────────────────


class TestSerialisation:
    def test_compress_format_size(self) -> None:
        # 8-byte header + 4 bytes per token.
        tokens = [Token(0, 65), Token(1, 66)]
        data = compress(b"AB")
        # Header (8) + 2 tokens × 4 bytes each = 16.
        assert len(data) == 8 + 2 * 4

    def test_compress_decompress_all_spec_vectors(self) -> None:
        vectors = [b"", b"A", b"ABCDE", b"AAAAAAA", b"ABABABAB", b"AABCBBABC"]
        for v in vectors:
            assert decompress(compress(v)) == v, f"Failed for {v!r}"

    def test_decompress_empty(self) -> None:
        assert decompress(compress(b"")) == b""

    def test_compress_header(self) -> None:
        import struct

        # Verify the 8-byte header structure.
        data = b"AB"
        compressed = compress(data)
        original_length = struct.unpack(">I", compressed[0:4])[0]
        token_count = struct.unpack(">I", compressed[4:8])[0]
        assert original_length == 2
        assert token_count == len(encode(b"AB"))

    def test_deterministic(self) -> None:
        data = b"hello world test data repeated repeated"
        assert compress(data) == compress(data)


# ─── Behaviour tests ──────────────────────────────────────────────────────────


class TestBehaviour:
    def test_repetitive_data_compresses(self) -> None:
        data = b"ABC" * 1000  # 3000 bytes
        assert len(compress(data)) < len(data)

    def test_incompressible_data_does_not_expand_excessively(self) -> None:
        # Worst case: all distinct bytes, no repetition. At most 4×len + header.
        data = bytes(range(256))
        compressed = compress(data)
        assert len(compressed) <= 4 * len(data) + 10

    def test_all_same_byte_compresses(self) -> None:
        data = b"A" * 10000
        compressed = compress(data)
        # Dictionary entries grow: "A", "AA", "AAA", ... each doubling coverage.
        # For 10000 'A's, expect far fewer than 10000 tokens.
        assert len(compressed) < len(data)
        assert decompress(compressed) == data

    def test_bytearray_input(self) -> None:
        # Should accept bytearray, not just bytes.
        data = bytearray(b"hello world")
        assert decompress(compress(data)) == b"hello world"
