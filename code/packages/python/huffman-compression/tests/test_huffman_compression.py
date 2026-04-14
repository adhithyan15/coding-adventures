"""Tests for coding_adventures_huffman_compression (CMP04).

Test organisation
-----------------
1. Round-trip tests (compress → decompress = original)
2. Wire format verification (exact byte layout)
3. Edge cases (empty, single byte, single symbol repeated)
4. Compression effectiveness (compressible data shrinks)
5. Error handling (malformed input)
"""

from __future__ import annotations

import struct

import pytest

from coding_adventures_huffman_compression import compress, decompress


# ---------------------------------------------------------------------------
# 1. Round-trip tests
# ---------------------------------------------------------------------------

class TestRoundTrip:
    """compress(data) → decompress → original for a variety of inputs."""

    def test_simple_aaabbc(self) -> None:
        data = b"AAABBC"
        assert decompress(compress(data)) == data

    def test_hello_world(self) -> None:
        data = b"hello world"
        assert decompress(compress(data)) == data

    def test_all_256_byte_values(self) -> None:
        data = bytes(range(256))
        assert decompress(compress(data)) == data

    def test_all_256_bytes_repeated(self) -> None:
        data = bytes(range(256)) * 10
        assert decompress(compress(data)) == data

    def test_single_byte(self) -> None:
        data = b"X"
        assert decompress(compress(data)) == data

    def test_two_bytes(self) -> None:
        data = b"AB"
        assert decompress(compress(data)) == data

    def test_lorem_ipsum(self) -> None:
        text = (
            b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. "
            b"Sed do eiusmod tempor incididunt ut labore et dolore magna aliqua."
        )
        assert decompress(compress(text)) == text

    def test_binary_data(self) -> None:
        data = bytes([0, 1, 2, 3, 255, 254, 253, 128, 64, 32])
        assert decompress(compress(data)) == data

    def test_repeated_pattern(self) -> None:
        data = b"ABCABC" * 100
        assert decompress(compress(data)) == data

    def test_single_repeated_byte(self) -> None:
        data = b"A" * 100
        assert decompress(compress(data)) == data

    def test_two_symbol_input(self) -> None:
        data = b"AB" * 50
        assert decompress(compress(data)) == data

    def test_newline_heavy_text(self) -> None:
        data = b"line\n" * 200
        assert decompress(compress(data)) == data

    def test_long_input(self) -> None:
        data = b"the quick brown fox jumps over the lazy dog " * 500
        assert decompress(compress(data)) == data


# ---------------------------------------------------------------------------
# 2. Wire format verification
# ---------------------------------------------------------------------------

class TestWireFormat:
    """Verify the exact byte layout of the CMP04 wire format."""

    def test_empty_input_wire_format(self) -> None:
        """compress(b'') must produce exactly an 8-byte header."""
        result = compress(b"")
        assert len(result) == 8
        original_length, symbol_count = struct.unpack(">II", result[:8])
        assert original_length == 0
        assert symbol_count == 0

    def test_aaabbc_wire_format(self) -> None:
        """Verify the exact wire-format bytes for b'AAABBC'.

        According to the spec:
          Frequencies: A=3, B=2, C=1
          DT27 canonical table: A→"0" (len=1), B→"10" (len=2), C→"11" (len=2)
          Lengths sorted by (length, symbol): [(65,1), (66,2), (67,2)]

          Header:
            00 00 00 06   original_length = 6
            00 00 00 03   symbol_count = 3
          Code-lengths table:
            41 01         symbol='A'(65), length=1
            42 02         symbol='B'(66), length=2
            43 02         symbol='C'(67), length=2
          Bit stream:
            Encoding: A→"0", A→"0", A→"0", B→"10", B→"10", C→"11"
            Concatenated: "000101011" (9 bits)
            Packed LSB-first:
              Byte 0: bits 0..7 → 0b10101000 = 0xA8
              Byte 1: bit  8    → 0b00000001 = 0x01
        """
        result = compress(b"AAABBC")

        # Header
        original_length, symbol_count = struct.unpack(">II", result[:8])
        assert original_length == 6
        assert symbol_count == 3

        # Code-lengths table
        assert result[8] == 65   # 'A'
        assert result[9] == 1    # length 1
        assert result[10] == 66  # 'B'
        assert result[11] == 2   # length 2
        assert result[12] == 67  # 'C'
        assert result[13] == 2   # length 2

        # Bit stream
        assert result[14] == 0xA8
        assert result[15] == 0x01

        # Total: 4+4+6+2 = 16 bytes
        assert len(result) == 16

    def test_header_original_length_field(self) -> None:
        for length in [1, 5, 100, 1000]:
            data = b"A" * length
            compressed = compress(data)
            stored_length = struct.unpack(">I", compressed[:4])[0]
            assert stored_length == length

    def test_symbol_count_field(self) -> None:
        """symbol_count must equal the number of distinct bytes."""
        assert struct.unpack(">I", compress(b"A")[4:8])[0] == 1
        assert struct.unpack(">I", compress(b"AB")[4:8])[0] == 2
        assert struct.unpack(">I", compress(b"ABC")[4:8])[0] == 3
        # All 256 distinct bytes
        data = bytes(range(256))
        assert struct.unpack(">I", compress(data)[4:8])[0] == 256

    def test_code_lengths_table_is_sorted(self) -> None:
        """Wire format entries must be sorted by (code_length, symbol)."""
        result = compress(b"AAABBC")
        # Parse the code-lengths table
        _, symbol_count = struct.unpack(">II", result[:8])
        lengths = []
        for i in range(symbol_count):
            off = 8 + 2 * i
            sym = result[off]
            length = result[off + 1]
            lengths.append((length, sym))
        # Verify sorted
        assert lengths == sorted(lengths)

    def test_bit_stream_starts_after_table(self) -> None:
        """Bit stream must begin at offset 8 + 2*symbol_count."""
        data = b"AAABBC"
        result = compress(data)
        _, symbol_count = struct.unpack(">II", result[:8])
        bits_offset = 8 + 2 * symbol_count
        # Verify the bit stream has some content (at least 1 byte)
        assert len(result) > bits_offset

    def test_single_byte_input_wire_format(self) -> None:
        """Single byte input: symbol='A'(65), length=1, bit stream = [0x00]."""
        result = compress(b"A")
        original_length, symbol_count = struct.unpack(">II", result[:8])
        assert original_length == 1
        assert symbol_count == 1
        # Code-lengths table: (65, 1)
        assert result[8] == 65
        assert result[9] == 1
        # Bit stream: "0" packed → 0x00
        assert result[10] == 0x00
        assert len(result) == 11


# ---------------------------------------------------------------------------
# 3. Edge cases
# ---------------------------------------------------------------------------

class TestEdgeCases:
    """Edge cases: empty input, single byte, single repeated symbol."""

    def test_empty_compress(self) -> None:
        assert compress(b"") == struct.pack(">II", 0, 0)

    def test_empty_decompress(self) -> None:
        assert decompress(compress(b"")) == b""

    def test_decompress_empty_bytes(self) -> None:
        """Decompressing raw b'' should return b'' gracefully."""
        assert decompress(b"") == b""

    def test_decompress_short_header(self) -> None:
        """Headers shorter than 8 bytes return b''."""
        assert decompress(b"\x00\x00\x00\x00") == b""

    def test_single_symbol_round_trip(self) -> None:
        """A stream of all identical bytes round-trips correctly."""
        for sym in [0, 65, 127, 255]:
            data = bytes([sym]) * 50
            assert decompress(compress(data)) == data

    def test_single_symbol_encodes_to_one_bit(self) -> None:
        """With one distinct symbol, each occurrence uses exactly 1 bit."""
        data = b"A" * 8
        result = compress(data)
        _, symbol_count = struct.unpack(">II", result[:8])
        assert symbol_count == 1
        # Bit stream: 8 bits → 1 byte
        bits_offset = 8 + 2 * symbol_count
        assert len(result) == bits_offset + 1

    def test_bytearray_input(self) -> None:
        data = bytearray(b"hello world")
        assert decompress(compress(data)) == b"hello world"

    def test_null_bytes(self) -> None:
        data = b"\x00" * 100
        assert decompress(compress(data)) == data

    def test_two_symbols_equal_frequency(self) -> None:
        data = b"AB" * 100
        result = decompress(compress(data))
        assert result == data


# ---------------------------------------------------------------------------
# 4. Compression effectiveness
# ---------------------------------------------------------------------------

class TestCompressionEffectiveness:
    """Highly skewed distributions should compress well."""

    def test_compressible_input_shrinks(self) -> None:
        """'A' × 900 + 'B' × 100 should compress to fewer bytes."""
        data = b"A" * 900 + b"B" * 100
        compressed = compress(data)
        # 'A' gets code "0" (1 bit each) → 900 bits; 'B' gets "1" → 100 bits
        # Total: 1000 bits = 125 bytes + 8-byte header + 4-byte table = 137 bytes
        # vs. 1000 raw bytes  →  should be much smaller.
        assert len(compressed) < len(data)

    def test_repeated_byte_compresses_well(self) -> None:
        data = b"X" * 1000
        compressed = compress(data)
        # 1000 bits = 125 bytes of bit stream + 11 bytes header+table = 136 bytes
        assert len(compressed) < len(data)

    def test_uniform_distribution_larger_than_original(self) -> None:
        """All 256 symbols once — Huffman can't win against raw bytes here.

        With 256 distinct symbols, every code is 8 bits on average, so the
        bit stream is the same size as the original. Plus headers and code
        table overhead → compressed is larger.
        """
        data = bytes(range(256))
        compressed = compress(data)
        assert len(compressed) > len(data)  # overhead dominates small inputs


# ---------------------------------------------------------------------------
# 5. Idempotency and stability
# ---------------------------------------------------------------------------

class TestIdempotency:
    """Deterministic encoding: same input always produces same output."""

    def test_deterministic_compression(self) -> None:
        data = b"the quick brown fox jumps over the lazy dog"
        assert compress(data) == compress(data)

    def test_deterministic_decompression(self) -> None:
        data = b"hello world"
        compressed = compress(data)
        assert decompress(compressed) == decompress(compressed)

    def test_compress_twice_same_result(self) -> None:
        data = b"ABCABCABC" * 10
        assert compress(data) == compress(data)
