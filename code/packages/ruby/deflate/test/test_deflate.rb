# frozen_string_literal: true

require "minitest/autorun"
require "coding_adventures_deflate"

class TestDeflate < Minitest::Test
  def roundtrip(data, label = "data")
    compressed = CodingAdventures::Deflate.compress(data)
    result = CodingAdventures::Deflate.decompress(compressed)
    assert_equal data.b, result, "roundtrip mismatch for #{label}"
  end

  # -------------------------------------------------------------------------
  # Edge cases
  # -------------------------------------------------------------------------

  def test_empty
    compressed = CodingAdventures::Deflate.compress("")
    result = CodingAdventures::Deflate.decompress(compressed)
    assert_equal "".b, result
  end

  def test_single_byte_null
    roundtrip("\x00", "NUL")
  end

  def test_single_byte_0xff
    roundtrip("\xFF", "0xFF")
  end

  def test_single_byte_a
    roundtrip("A", "A")
  end

  def test_single_byte_repeated
    roundtrip("A" * 20, "A×20")
    roundtrip("\x00" * 100, "NUL×100")
  end

  # -------------------------------------------------------------------------
  # Spec examples
  # -------------------------------------------------------------------------

  def test_aaabbc_all_literals
    data = "AAABBC"
    roundtrip(data, "AAABBC")
    compressed = CodingAdventures::Deflate.compress(data)
    _orig_len, _ll_count, dist_count = compressed.unpack("Nnn")
    assert_equal 0, dist_count, "expected dist_entry_count=0 for all-literals input"
  end

  def test_aabcbbabc_one_match
    data = "AABCBBABC"
    roundtrip(data, "AABCBBABC")
    compressed = CodingAdventures::Deflate.compress(data)
    orig_len, _ll_count, dist_count = compressed.unpack("Nnn")
    assert_equal 9, orig_len
    assert dist_count > 0, "expected dist_entry_count>0 for input with a match"
  end

  # -------------------------------------------------------------------------
  # Match tests
  # -------------------------------------------------------------------------

  def test_overlapping_match
    roundtrip("AAAAAAA", "run of A")
    roundtrip("ABABABABABAB", "ABAB run")
  end

  def test_multiple_matches
    roundtrip("ABCABCABCABC", "ABCABC×3")
    roundtrip("hello hello hello world", "hello×3")
  end

  def test_max_match_length
    roundtrip("A" * 300, "A×300")
  end

  # -------------------------------------------------------------------------
  # Data variety
  # -------------------------------------------------------------------------

  def test_all_256_byte_values
    data = (0..255).map(&:chr).join.b
    roundtrip(data, "all-bytes")
  end

  def test_binary_data_1000_bytes
    data = (0..999).map { |i| (i % 256).chr }.join.b
    roundtrip(data, "binary-1000")
  end

  def test_longer_text_with_repetition
    base = "the quick brown fox jumps over the lazy dog "
    roundtrip(base * 10, "pangram×10")
  end

  # -------------------------------------------------------------------------
  # Compression ratio
  # -------------------------------------------------------------------------

  def test_compression_ratio
    data = "ABCABC" * 100
    compressed = CodingAdventures::Deflate.compress(data)
    assert compressed.bytesize < data.bytesize / 2,
           "expected significant compression: #{compressed.bytesize} >= #{data.bytesize / 2}"
  end

  # -------------------------------------------------------------------------
  # Various match lengths
  # -------------------------------------------------------------------------

  def test_various_match_lengths
    [3, 4, 10, 11, 13, 19, 35, 67, 131, 227, 255].each do |length|
      prefix = "A" * length
      data = prefix + "BBB" + prefix
      roundtrip(data, "length=#{length}")
    end
  end

  # -------------------------------------------------------------------------
  # Diverse round-trips
  # -------------------------------------------------------------------------

  def test_diverse_inputs
    [
      "\x00" * 100,
      "\xFF" * 100,
      "abcdefghijklmnopqrstuvwxyz",
      "The quick brown fox " * 20
    ].each_with_index do |data, i|
      roundtrip(data, "diverse-#{i}")
    end
  end
end
