"""Tests for the LZ77 compression implementation.

Test vectors come from the CMP00 specification and cover all key cases:
literals, backreferences, overlapping matches, edge cases, and round-trip
invariants. Coverage target: 95%+ to ensure all branches in encode/decode
are exercised.
"""


from coding_adventures_lz77 import Token, compress, decode, decompress, encode

# ---- Specification Test Vectors ----


class TestSpecVectors:
    """Test vectors from the CMP00 specification."""

    def test_empty_input(self) -> None:
        """Empty input produces no tokens."""
        assert encode(b"") == []
        assert decode([]) == b""

    def test_no_repetition(self) -> None:
        """No repeated substrings -> all literal tokens.

        Input: "ABCDE" (bytes 65-69).
        Expected: 5 tokens, each (0, 0, byte).
        """
        result = encode(b"ABCDE")
        expected = [
            Token(0, 0, ord("A")),
            Token(0, 0, ord("B")),
            Token(0, 0, ord("C")),
            Token(0, 0, ord("D")),
            Token(0, 0, ord("E")),
        ]
        assert result == expected

    def test_all_identical_bytes(self) -> None:
        """All identical bytes exploit the overlap mechanism.

        Input: "AAAAAAA" (7 × A).
        Expected: First A as literal, then one backreference with overlap
                  covering the remaining 5 bytes, then final A.
        Specifically: [(0,0,65), (1,5,65)]
        """
        result = encode(b"AAAAAAA")
        assert len(result) == 2
        assert result[0] == Token(0, 0, ord("A"))
        # The second token uses offset=1 (one byte back) with length=5 (overlap).
        assert result[1].offset == 1
        assert result[1].length == 5
        assert result[1].next_char == ord("A")

        # Verify decode produces the original.
        assert decode(result) == b"AAAAAAA"

    def test_repeated_pair(self) -> None:
        """Repeated pair exploits non-overlapping match.

        Input: "ABABABAB".
        Expected: Literal A, literal B, then backreference to the AB pair
                  with length=5 (matches ABABA) and next_char=B.
        """
        result = encode(b"ABABABAB")
        assert len(result) == 3
        assert result[0] == Token(0, 0, ord("A"))
        assert result[1] == Token(0, 0, ord("B"))
        # Third token references the AB pair.
        assert result[2].offset == 2
        assert result[2].length == 5
        assert result[2].next_char == ord("B")

        assert decode(result) == b"ABABABAB"

    def test_substring_reuse_no_match(self) -> None:
        """Substring reuse but with min_match=3 threshold.

        Input: "AABCBBABC".
        With default min_match=3, there are no matches of length ≥ 3.
        Expected: All literal tokens.
        """
        result = encode(b"AABCBBABC")
        assert len(result) == 9
        for token in result:
            assert token.offset == 0
            assert token.length == 0

        assert decode(result) == b"AABCBBABC"

    def test_substring_reuse_with_lower_min_match(self) -> None:
        """Same input but with min_match=2 to trigger some matches."""
        result = encode(b"AABCBBABC", min_match=2)
        # With min_match=2, we should find some backreferences.
        # (Exact structure depends on the encoding details, but tokens
        # should reconstruct correctly.)
        assert decode(result) == b"AABCBBABC"


# ---- Round-Trip Invariant Tests ----


class TestRoundTrip:
    """Verify encode/decode round-trip property: decode(encode(x)) == x."""

    def test_empty_round_trip(self) -> None:
        assert decode(encode(b"")) == b""

    def test_single_byte_round_trip(self) -> None:
        assert decode(encode(b"A")) == b"A"
        assert decode(encode(b"\x00")) == b"\x00"
        assert decode(encode(b"\xff")) == b"\xff"

    def test_ascii_round_trip(self) -> None:
        test_cases = [
            b"hello world",
            b"the quick brown fox",
            b"ababababab",
            b"aaaaaaaaaa",
        ]
        for data in test_cases:
            tokens = encode(data)
            result = decode(tokens)
            assert result == data, f"Round-trip failed for {data!r}"

    def test_binary_round_trip(self) -> None:
        # Test with various byte patterns including null bytes.
        test_cases = [
            b"\x00\x00\x00",
            b"\xff\xff\xff",
            bytes(range(256)),
            b"\x00\x01\x02\x00\x01\x02",
        ]
        for data in test_cases:
            tokens = encode(data)
            result = decode(tokens)
            assert result == data, f"Round-trip failed for {data!r}"

    def test_compress_decompress_round_trip(self) -> None:
        """Verify compress/decompress (with serialisation) round-trip."""
        test_cases = [
            b"",
            b"A",
            b"ABCDE",
            b"AAAAAAA",
            b"ABABABAB",
            b"hello world",
        ]
        for data in test_cases:
            compressed = compress(data)
            result = decompress(compressed)
            assert result == data, f"Compress/decompress failed for {data!r}"


# ---- Parameter Handling Tests ----


class TestParameters:
    """Verify parameter constraints are respected."""

    def test_window_size_limit(self) -> None:
        """Offsets never exceed window_size."""
        # Create a pattern that repeats with large spacing.
        data = b"X" + b"Y" * 5000 + b"X"
        result = encode(data, window_size=100)
        for token in result:
            assert token.offset <= 100, f"Offset {token.offset} exceeds window_size 100"

    def test_max_match_limit(self) -> None:
        """Match lengths never exceed max_match."""
        data = b"A" * 1000
        result = encode(data, max_match=50)
        for token in result:
            assert token.length <= 50, f"Length {token.length} exceeds max_match 50"

    def test_min_match_threshold(self) -> None:
        """Matches shorter than min_match are not emitted as backreferences."""
        # "AABAA" has an "A" match of length 1 at position 4.
        result = encode(b"AABAA", min_match=2)
        # Should not have any backreferences.
        for token in result:
            assert token.length >= 2 or token.length == 0


# ---- Edge Cases ----


class TestEdgeCases:
    """Edge cases and boundary conditions."""

    def test_single_byte_literal(self) -> None:
        """Single byte encodes as a literal token."""
        result = encode(b"X")
        assert result == [Token(0, 0, ord("X"))]

    def test_exact_window_boundary(self) -> None:
        """Match at exactly window_size offset."""
        # Create a pattern that repeats at exactly window_size distance.
        window = 10
        data = b"X" * window + b"X"
        result = encode(data, window_size=window)
        # The second X should match the first.
        found_match = any(t.offset > 0 for t in result)
        assert found_match, "Expected to find a match at window boundary"
        assert decode(result) == data

    def test_overlapping_match_decode(self) -> None:
        """Verify byte-by-byte copy handles overlapping matches."""
        # Manually construct a token with offset < length (overlap).
        # Start with [A, B] and apply (offset=2, length=5, next_char='Z').
        tokens = [
            Token(0, 0, ord("A")),
            Token(0, 0, ord("B")),
            Token(2, 5, ord("Z")),
        ]
        result = decode(tokens)
        # The overlapping match should produce ABABAB (5 bytes copied byte-by-byte),
        # then append Z. Total: ABABABAZ (8 bytes).
        expected = b"ABABABAZ"
        assert result == expected, f"Got {result!r}, expected {expected!r}"

    def test_binary_with_nulls(self) -> None:
        """Null bytes are handled correctly."""
        data = b"\x00\x00\x00\xff\xff"
        result = encode(data)
        assert decode(result) == data

    def test_very_long_input(self) -> None:
        """Large files compress correctly."""
        # Create a file with repeating patterns.
        data = (b"Hello, World! " * 100) + (b"X" * 500)
        result = encode(data)
        assert decode(result) == data

    def test_all_same_byte_long_run(self) -> None:
        """Very long run of identical byte."""
        data = b"A" * 10000
        result = encode(data)
        # Should compress: first A as literal, then matches of length=max_match.
        # With 10000 bytes: 1 literal + ~39 matches of 255 + 1 partial → ~41
        # tokens.
        assert len(result) < 50, "Expected compression for repeated byte"
        assert decode(result) == data


# ---- Serialisation Tests ----


class TestSerialisation:
    """Verify compress/decompress serialisation format."""

    def test_serialisation_format_structure(self) -> None:
        """Verify serialised format has correct structure."""
        tokens = [
            Token(0, 0, 65),
            Token(2, 5, 66),
        ]
        from coding_adventures_lz77 import _serialise_tokens

        serialised = _serialise_tokens(tokens)
        # 4 bytes for count + 2 tokens × 4 bytes = 12 bytes total.
        assert len(serialised) == 4 + 2 * 4

    def test_round_trip_with_zero_tokens(self) -> None:
        """Empty token list serialises and deserialises correctly."""
        compressed = compress(b"")
        result = decompress(compressed)
        assert result == b""

    def test_compress_decompress_all_vectors(self) -> None:
        """Compress/decompress all spec vectors."""
        vectors = [
            b"",
            b"ABCDE",
            b"AAAAAAA",
            b"ABABABAB",
            b"AABCBBABC",
        ]
        for vector in vectors:
            compressed = compress(vector)
            result = decompress(compressed)
            assert result == vector, f"Failed for {vector!r}"


# ---- Behaviour Verification ----


class TestBehaviour:
    """Verify expected compression behaviour."""

    def test_no_expansion_on_incompressible_data(self) -> None:
        """Random/incompressible data does not expand too much."""
        # Worst case: N bytes of unique data → N tokens of (0, 0, byte).
        # Serialisation: 4 bytes header + N × 4 bytes = 4N + 4 bytes.
        data = bytes(range(256))  # 256 unique bytes.
        compressed = compress(data)
        # Serialised size should be ≈4 * 256 + 4 = 1028 bytes.
        assert len(compressed) <= 4 * len(data) + 10

    def test_compression_of_repetitive_data(self) -> None:
        """Repetitive data compresses significantly."""
        repetitive = b"ABC" * 100
        compressed = compress(repetitive)
        # Should compress to much less than original size.
        assert len(compressed) < len(repetitive)

    def test_deterministic_compression(self) -> None:
        """Compress always produces the same output for the same input."""
        data = b"hello world test"
        result1 = compress(data)
        result2 = compress(data)
        assert result1 == result2
