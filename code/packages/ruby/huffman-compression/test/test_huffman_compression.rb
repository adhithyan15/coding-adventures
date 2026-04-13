# frozen_string_literal: true

require "minitest/autorun"
require_relative "../lib/coding_adventures/huffman_compression"

# =============================================================================
# Tests for CodingAdventures::HuffmanCompression (CMP04)
# =============================================================================
#
# Test strategy:
#   1. Round-trip correctness for varied inputs (letters, binary, edge cases).
#   2. Wire-format structural checks (header fields, table layout).
#   3. Edge cases: empty, single byte, single distinct byte, all 256 byte values.
#   4. Compression effectiveness on repetitive data.
#
# The canonical-code wire format test uses "AAABBC" because it has a known,
# easy-to-verify structure:
#   Frequencies:  A=3, B=2, C=1
#   Code lengths: A→1, B→2, C→2
#   Canonical:    A→"0", B→"10", C→"11"
#   Header:
#     00 00 00 06  (original_length = 6)
#     00 00 00 03  (symbol_count = 3)
#     41 01        (sym=0x41='A'=65, len=1)
#     42 02        (sym=0x42='B'=66, len=2)
#     43 02        (sym=0x43='C'=67, len=2)
#
# =============================================================================

class TestHuffmanCompression < Minitest::Test
  include CodingAdventures

  # ── Helpers ──────────────────────────────────────────────────────────────────

  # Compress then decompress; return the round-tripped string.
  def roundtrip(data)
    compressed = HuffmanCompression.compress(data)
    HuffmanCompression.decompress(compressed)
  end

  # ── Round-trip tests ─────────────────────────────────────────────────────────

  def test_roundtrip_empty
    assert_equal "".b, roundtrip("")
  end

  def test_roundtrip_single_byte
    assert_equal "A".b, roundtrip("A")
  end

  def test_roundtrip_two_distinct
    assert_equal "AB".b, roundtrip("AB")
  end

  def test_roundtrip_aaabbc
    assert_equal "AAABBC".b, roundtrip("AAABBC")
  end

  def test_roundtrip_all_same
    assert_equal ("A" * 100).b, roundtrip("A" * 100)
  end

  def test_roundtrip_long_text
    text = "the quick brown fox jumps over the lazy dog " * 20
    assert_equal text.b, roundtrip(text)
  end

  def test_roundtrip_binary_all_bytes
    # All 256 byte values, repeated 4 times — stresses the full symbol range.
    data = (0..255).map(&:chr).join.b * 4
    assert_equal data, roundtrip(data)
  end

  def test_roundtrip_all_zeros
    data = ("\x00" * 200).b
    assert_equal data, roundtrip(data)
  end

  def test_roundtrip_all_ff
    data = ("\xFF" * 200).b
    assert_equal data, roundtrip(data)
  end

  def test_roundtrip_random_ish_binary
    # A pseudo-random but deterministic byte pattern.
    data = 300.times.map { |i| (i * 37 + 13) % 256 }.pack("C*")
    assert_equal data, roundtrip(data)
  end

  def test_roundtrip_newlines_and_spaces
    data = "hello\nworld\n  foo  \n"
    assert_equal data.b, roundtrip(data)
  end

  # ── Wire-format header tests ─────────────────────────────────────────────────

  def test_header_original_length_field
    data = "AAABBC"
    compressed = HuffmanCompression.compress(data)
    assert_equal 6, compressed[0, 4].unpack1("N")
  end

  def test_header_symbol_count_field
    # "AAABBC" has 3 distinct bytes: A, B, C
    data = "AAABBC"
    compressed = HuffmanCompression.compress(data)
    assert_equal 3, compressed[4, 4].unpack1("N")
  end

  def test_header_empty_data
    compressed = HuffmanCompression.compress("")
    assert_equal 0, compressed[0, 4].unpack1("N")
    assert_equal 0, compressed[4, 4].unpack1("N")
    assert_equal 8, compressed.bytesize
  end

  def test_wire_format_aaabbc_code_lengths_table
    # Verify the code-lengths table for "AAABBC":
    #   Entry 0: sym=0x41 (A=65), len=1  → bytes [0x41, 0x01]
    #   Entry 1: sym=0x42 (B=66), len=2  → bytes [0x42, 0x02]
    #   Entry 2: sym=0x43 (C=67), len=2  → bytes [0x43, 0x02]
    compressed = HuffmanCompression.compress("AAABBC")

    # symbol_count = 3, so table occupies bytes 8..13 (6 bytes total)
    table = compressed[8, 6]

    # Entry 0: A with length 1
    assert_equal 65, table[0, 1].unpack1("C"), "first symbol should be A (65)"
    assert_equal 1,  table[1, 1].unpack1("C"), "A code length should be 1"

    # Entry 1: B with length 2
    assert_equal 66, table[2, 1].unpack1("C"), "second symbol should be B (66)"
    assert_equal 2,  table[3, 1].unpack1("C"), "B code length should be 2"

    # Entry 2: C with length 2
    assert_equal 67, table[4, 1].unpack1("C"), "third symbol should be C (67)"
    assert_equal 2,  table[5, 1].unpack1("C"), "C code length should be 2"
  end

  def test_wire_format_table_sorted_by_len_then_sym
    # Build a string with 4 distinct bytes and known frequencies.
    # 'D'=1, 'C'=2, 'B'=3, 'A'=6 → lengths may vary but sort order must hold.
    data = ("A" * 6) + ("B" * 3) + ("C" * 2) + "D"
    compressed = HuffmanCompression.compress(data)

    symbol_count = compressed[4, 4].unpack1("N")
    entries = symbol_count.times.map do |i|
      offset = 8 + (i * 2)
      sym = compressed[offset, 1].unpack1("C")
      len = compressed[offset + 1, 1].unpack1("C")
      [sym, len]
    end

    # Verify sorted by (length, symbol)
    sorted = entries.sort_by { |sym, len| [len, sym] }
    assert_equal sorted, entries, "code-lengths table must be sorted by (length, symbol)"
  end

  def test_wire_minimum_size
    # Even a 1-byte input needs at least: 4 (orig_len) + 4 (sym_count) + 2 (table) + 1 (bits)
    compressed = HuffmanCompression.compress("X")
    assert compressed.bytesize >= 11, "expected at least 11 bytes for 1-byte input"
  end

  # ── Edge cases ───────────────────────────────────────────────────────────────

  def test_single_distinct_byte_many_repeats
    # All same byte: only one symbol in the alphabet.
    # The canonical code is "0" (1 bit) by convention.
    data = ("Z" * 50).b
    compressed = HuffmanCompression.compress(data)

    # symbol_count must be 1
    assert_equal 1, compressed[4, 4].unpack1("N")

    # Code length must be 1
    assert_equal 1, compressed[9, 1].unpack1("C")

    # Must round-trip
    assert_equal data, HuffmanCompression.decompress(compressed)
  end

  def test_two_distinct_bytes
    data = ("AB" * 30).b
    compressed = HuffmanCompression.compress(data)
    assert_equal 2, compressed[4, 4].unpack1("N")
    assert_equal data, HuffmanCompression.decompress(compressed)
  end

  def test_all_256_byte_values_single_occurrence
    # 256 distinct symbols each with frequency 1 — maximum alphabet size.
    data = (0..255).map(&:chr).join.b
    compressed = HuffmanCompression.compress(data)
    assert_equal 256, compressed[4, 4].unpack1("N")
    assert_equal data, HuffmanCompression.decompress(compressed)
  end

  def test_long_repetitive_data
    data = ("ABCD" * 250).b
    assert_equal data, roundtrip(data)
  end

  # ── Decompress edge cases ────────────────────────────────────────────────────

  def test_decompress_too_short_returns_empty
    # Fewer than 8 bytes — can't even parse the header.
    result = HuffmanCompression.decompress("\x00\x00\x00")
    assert_equal "".b, result
  end

  def test_decompress_empty_original_length
    # Header with original_length=0 should return empty string.
    header = [0, 0].pack("NN")
    result = HuffmanCompression.decompress(header)
    assert_equal "".b, result
  end

  # ── Compression effectiveness ─────────────────────────────────────────────────

  def test_compression_effectiveness_repetitive
    # Highly skewed frequencies give near-ideal compression.
    # "A"*1000 + "B"*10 → about 2 bits/byte → much smaller than 1000 bytes.
    data = ("A" * 1000) + ("B" * 10)
    compressed = HuffmanCompression.compress(data)
    # At ~1 bit per byte for A and ~2 bits per B, expect well under 200 bytes body.
    assert compressed.bytesize < data.bytesize,
           "expected Huffman to compress skewed data; " \
           "got compressed=#{compressed.bytesize} >= original=#{data.bytesize}"
  end

  def test_compression_four_symbol_text
    data = "ABCABC" * 100
    compressed = HuffmanCompression.compress(data)
    assert compressed.bytesize < data.bytesize,
           "expected compression; got #{compressed.bytesize} >= #{data.bytesize}"
  end

  # ── Idempotency / stability ───────────────────────────────────────────────────

  def test_compress_twice_same_output
    # Deterministic: same input → same output always.
    data = "hello world this is a test"
    assert_equal HuffmanCompression.compress(data), HuffmanCompression.compress(data)
  end

  def test_header_preserves_original_length
    data = "hello world"
    compressed = HuffmanCompression.compress(data)
    assert_equal data.bytesize, compressed[0, 4].unpack1("N")
  end
end
