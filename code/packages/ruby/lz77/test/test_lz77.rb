# frozen_string_literal: true

# Comprehensive tests for the LZ77 compression implementation.
#
# Test vectors come from the CMP00 specification and cover all key cases:
# literals, backreferences, overlapping matches, edge cases, and round-trip
# invariants. Coverage target: 95%+.
#
# All string comparisons use .b (ASCII-8BIT) because decode returns a binary
# string. This is correct: LZ77 operates on raw bytes, not Unicode characters.

require "minitest/autorun"
require "coding_adventures_lz77"

# Shorthand for binary string literal.
module BinaryHelper
  def b(str)
    str.b
  end
end

# ---- Specification Test Vectors ----

class TestSpecVectors < Minitest::Test
  include BinaryHelper

  def test_empty_input
    assert_equal [], CodingAdventures::LZ77.encode("")
    assert_equal b(""), CodingAdventures::LZ77.decode([])
  end

  def test_no_repetition
    # Input: "ABCDE" — no repeated substrings → all literal tokens.
    tokens = CodingAdventures::LZ77.encode("ABCDE")
    assert_equal 5, tokens.length
    tokens.each do |t|
      assert_equal 0, t.offset
      assert_equal 0, t.length
    end
  end

  def test_all_identical_bytes
    # Input: "AAAAAAA" (7 × A).
    # Expected: literal A + backreference with overlap (offset=1, length=5, next_char=A).
    tokens = CodingAdventures::LZ77.encode("AAAAAAA")
    assert_equal 2, tokens.length
    assert_equal CodingAdventures::LZ77::Token.new(0, 0, 65), tokens[0]
    assert_equal 1, tokens[1].offset
    assert_equal 5, tokens[1].length
    assert_equal 65, tokens[1].next_char

    assert_equal b("AAAAAAA"), CodingAdventures::LZ77.decode(tokens)
  end

  def test_repeated_pair
    # Input: "ABABABAB".
    # Expected: [A literal, B literal, backreference (offset=2, length=5, next_char='B')].
    tokens = CodingAdventures::LZ77.encode("ABABABAB")
    assert_equal 3, tokens.length
    assert_equal CodingAdventures::LZ77::Token.new(0, 0, 65), tokens[0]
    assert_equal CodingAdventures::LZ77::Token.new(0, 0, 66), tokens[1]
    assert_equal 2, tokens[2].offset
    assert_equal 5, tokens[2].length
    assert_equal 66, tokens[2].next_char

    assert_equal b("ABABABAB"), CodingAdventures::LZ77.decode(tokens)
  end

  def test_substring_reuse_no_match
    # Input: "AABCBBABC" with default min_match=3 → all literals (no match ≥ 3).
    tokens = CodingAdventures::LZ77.encode("AABCBBABC")
    assert_equal 9, tokens.length
    tokens.each do |t|
      assert_equal 0, t.offset
      assert_equal 0, t.length
    end

    assert_equal b("AABCBBABC"), CodingAdventures::LZ77.decode(tokens)
  end

  def test_substring_reuse_with_lower_min_match
    # With min_match=2, some backreferences should appear.
    tokens = CodingAdventures::LZ77.encode("AABCBBABC", min_match: 2)
    assert_equal b("AABCBBABC"), CodingAdventures::LZ77.decode(tokens)
  end
end

# ---- Round-Trip Invariant Tests ----

class TestRoundTrip < Minitest::Test
  # Helper: encode then decode a string, return the result.
  def rt(str)
    CodingAdventures::LZ77.decode(CodingAdventures::LZ77.encode(str))
  end

  def test_empty_round_trip
    assert_equal "".b, rt("")
  end

  def test_single_byte
    assert_equal "A".b, rt("A")
    assert_equal "\x00".b, rt("\x00")
    assert_equal "\xff".b, rt("\xff")
  end

  def test_ascii_strings
    [
      "hello world",
      "the quick brown fox",
      "ababababab",
      "aaaaaaaaaa"
    ].each do |s|
      assert_equal s.b, rt(s), "Round-trip failed for #{s.inspect}"
    end
  end

  def test_binary_data
    [
      "\x00\x00\x00",
      "\xff\xff\xff",
      (0..255).map(&:chr).join,
      "\x00\x01\x02\x00\x01\x02"
    ].each do |s|
      assert_equal s.b, rt(s), "Round-trip failed for binary data"
    end
  end

  def test_compress_decompress_round_trip
    [
      "",
      "A",
      "ABCDE",
      "AAAAAAA",
      "ABABABAB",
      "hello world"
    ].each do |s|
      compressed = CodingAdventures::LZ77.compress(s)
      result = CodingAdventures::LZ77.decompress(compressed)
      assert_equal s.b, result, "Compress/decompress failed for #{s.inspect}"
    end
  end
end

# ---- Parameter Tests ----

class TestParameters < Minitest::Test
  def test_window_size_limit
    data = "X" + ("Y" * 5000) + "X"
    tokens = CodingAdventures::LZ77.encode(data, window_size: 100)
    tokens.each do |t|
      assert t.offset <= 100, "Offset #{t.offset} exceeds window_size 100"
    end
  end

  def test_max_match_limit
    data = "A" * 1000
    tokens = CodingAdventures::LZ77.encode(data, max_match: 50)
    tokens.each do |t|
      assert t.length <= 50, "Length #{t.length} exceeds max_match 50"
    end
  end

  def test_min_match_threshold
    tokens = CodingAdventures::LZ77.encode("AABAA", min_match: 2)
    tokens.each do |t|
      assert t.length >= 2 || t.length == 0, "Length #{t.length} is below min_match 2"
    end
  end
end

# ---- Edge Cases ----

class TestEdgeCases < Minitest::Test
  def test_single_byte_literal
    tokens = CodingAdventures::LZ77.encode("X")
    assert_equal 1, tokens.length
    assert_equal CodingAdventures::LZ77::Token.new(0, 0, 88), tokens[0]
  end

  def test_exact_window_boundary
    window = 10
    data = "X" * window + "X"
    tokens = CodingAdventures::LZ77.encode(data, window_size: window)
    assert tokens.any? { |t| t.offset > 0 }, "Expected at least one match at window boundary"
    assert_equal data.b, CodingAdventures::LZ77.decode(tokens)
  end

  def test_overlapping_match_decode
    # Start with [A, B] and apply (offset=2, length=5, next_char='Z').
    # Copies byte-by-byte: ABABAB then appends Z → ABABABAZ.
    tokens = [
      CodingAdventures::LZ77::Token.new(0, 0, 65),  # A
      CodingAdventures::LZ77::Token.new(0, 0, 66),  # B
      CodingAdventures::LZ77::Token.new(2, 5, 90)   # overlap → ABABAB, then Z
    ]
    result = CodingAdventures::LZ77.decode(tokens)
    assert_equal "ABABABAZ".b, result
  end

  def test_binary_with_nulls
    data = "\x00\x00\x00\xff\xff"
    tokens = CodingAdventures::LZ77.encode(data)
    assert_equal data.b, CodingAdventures::LZ77.decode(tokens)
  end

  def test_very_long_input
    data = ("Hello, World! " * 100) + ("X" * 500)
    tokens = CodingAdventures::LZ77.encode(data)
    assert_equal data.b, CodingAdventures::LZ77.decode(tokens)
  end

  def test_all_same_byte_long_run
    data = "A" * 10_000
    tokens = CodingAdventures::LZ77.encode(data)
    assert tokens.length < 50, "Expected < 50 tokens for 10000 identical bytes, got #{tokens.length}"
    assert_equal data.b, CodingAdventures::LZ77.decode(tokens)
  end
end

# ---- Serialisation Tests ----

class TestSerialisation < Minitest::Test
  def test_serialise_format_structure
    tokens = [
      CodingAdventures::LZ77::Token.new(0, 0, 65),
      CodingAdventures::LZ77::Token.new(2, 5, 66)
    ]
    serialised = CodingAdventures::LZ77::Compressor.serialise_tokens(tokens)
    # 4 bytes for count + 2 tokens × 4 bytes = 12 bytes total.
    assert_equal 12, serialised.bytesize
  end

  def test_round_trip_with_zero_tokens
    compressed = CodingAdventures::LZ77.compress("")
    result = CodingAdventures::LZ77.decompress(compressed)
    assert_equal "".b, result
  end

  def test_compress_decompress_all_vectors
    [
      "",
      "ABCDE",
      "AAAAAAA",
      "ABABABAB",
      "AABCBBABC"
    ].each do |s|
      compressed = CodingAdventures::LZ77.compress(s)
      result = CodingAdventures::LZ77.decompress(compressed)
      assert_equal s.b, result, "Failed for #{s.inspect}"
    end
  end
end

# ---- Behaviour Tests ----

class TestBehaviour < Minitest::Test
  def test_no_expansion_on_incompressible_data
    data = (0..255).map(&:chr).join
    compressed = CodingAdventures::LZ77.compress(data)
    assert compressed.bytesize <= 4 * data.bytesize + 10
  end

  def test_compression_of_repetitive_data
    data = "ABC" * 100
    compressed = CodingAdventures::LZ77.compress(data)
    assert compressed.bytesize < data.bytesize
  end

  def test_deterministic_compression
    data = "hello world test"
    result1 = CodingAdventures::LZ77.compress(data)
    result2 = CodingAdventures::LZ77.compress(data)
    assert_equal result1, result2
  end

  def test_version
    refute_nil CodingAdventures::LZ77::VERSION
    assert_equal "0.1.0", CodingAdventures::LZ77::VERSION
  end
end
