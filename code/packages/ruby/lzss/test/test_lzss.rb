require "minitest/autorun"
require_relative "../lib/coding_adventures_lzss"

L = CodingAdventures::LZSS::Literal
M = CodingAdventures::LZSS::Match

class TestLZSS < Minitest::Test

  def rt(data) = CodingAdventures::LZSS.decompress(CodingAdventures::LZSS.compress(data))

  # ─── Spec vectors ─────────────────────────────────────────────────────────

  def test_encode_empty
    assert_equal [], CodingAdventures::LZSS.encode("".b)
  end

  def test_encode_single_byte
    assert_equal [L.new(65)], CodingAdventures::LZSS.encode("A".b)
  end

  def test_encode_no_repetition
    tokens = CodingAdventures::LZSS.encode("ABCDE".b)
    assert_equal 5, tokens.length
    assert tokens.all? { |t| t.is_a?(L) }
  end

  def test_encode_aabcbbabc
    tokens = CodingAdventures::LZSS.encode("AABCBBABC".b)
    assert_equal 7, tokens.length
    assert_equal M.new(5, 3), tokens.last
  end

  def test_encode_ababab
    tokens = CodingAdventures::LZSS.encode("ABABAB".b)
    assert_equal [L.new(65), L.new(66), M.new(2, 4)], tokens
  end

  def test_encode_all_identical
    tokens = CodingAdventures::LZSS.encode("AAAAAAA".b)
    assert_equal [L.new(65), M.new(1, 6)], tokens
  end

  # ─── Encode properties ──────────────────────────────────────────────────

  def test_match_offset_positive
    tokens = CodingAdventures::LZSS.encode("ABABABAB".b)
    tokens.each do |t|
      assert t.offset >= 1 if t.is_a?(M)
    end
  end

  def test_match_length_ge_min_match
    tokens = CodingAdventures::LZSS.encode("ABABABABABAB".b)
    tokens.each do |t|
      assert t.length >= 3 if t.is_a?(M)
    end
  end

  def test_match_offset_within_window
    data = ("ABCABC" * 100).b
    tokens = CodingAdventures::LZSS.encode(data, window_size: 4)
    tokens.each do |t|
      assert t.offset <= 4 if t.is_a?(M)
    end
  end

  def test_match_length_within_max
    tokens = CodingAdventures::LZSS.encode(("A" * 100).b, max_match: 5)
    tokens.each do |t|
      assert t.length <= 5 if t.is_a?(M)
    end
  end

  def test_min_match_large_forces_literals
    tokens = CodingAdventures::LZSS.encode("ABABAB".b, min_match: 100)
    assert tokens.all? { |t| t.is_a?(L) }
  end

  # ─── Decode ───────────────────────────────────────────────────────────

  def test_decode_empty
    assert_equal "".b, CodingAdventures::LZSS.decode([], original_length: 0)
  end

  def test_decode_single_literal
    assert_equal "A".b, CodingAdventures::LZSS.decode([L.new(65)], original_length: 1)
  end

  def test_decode_overlapping_match
    tokens = [L.new(65), M.new(1, 6)]
    assert_equal "AAAAAAA".b, CodingAdventures::LZSS.decode(tokens, original_length: 7)
  end

  def test_decode_ababab
    tokens = [L.new(65), L.new(66), M.new(2, 4)]
    assert_equal "ABABAB".b, CodingAdventures::LZSS.decode(tokens, original_length: 6)
  end

  def test_decode_truncates
    tokens = [L.new(65), L.new(66), L.new(67)]
    assert_equal "AB".b, CodingAdventures::LZSS.decode(tokens, original_length: 2)
  end

  # ─── Round-trip ───────────────────────────────────────────────────────

  def test_rt_empty;           assert_equal "".b,             rt("".b);             end
  def test_rt_single;          assert_equal "A".b,            rt("A".b);            end
  def test_rt_no_repetition;   assert_equal "ABCDE".b,        rt("ABCDE".b);        end
  def test_rt_all_identical;   assert_equal "AAAAAAA".b,      rt("AAAAAAA".b);      end
  def test_rt_ababab;          assert_equal "ABABAB".b,        rt("ABABAB".b);       end
  def test_rt_aabcbbabc;       assert_equal "AABCBBABC".b,    rt("AABCBBABC".b);    end
  def test_rt_hello_world;     assert_equal "hello world".b,  rt("hello world".b);  end
  def test_rt_repeated_abc;    assert_equal ("ABC" * 100).b,  rt(("ABC" * 100).b);  end

  def test_rt_binary_nulls
    data = "\x00\x00\x00\xff\xff".b
    assert_equal data, rt(data)
  end

  def test_rt_full_byte_range
    data = (0..255).map(&:chr).join.b
    assert_equal data, rt(data)
  end

  def test_rt_repeated_pattern
    data = ([0, 1, 2].map(&:chr).join * 100).b
    assert_equal data, rt(data)
  end

  def test_rt_long
    data = ("ABCDEF" * 500).b
    assert_equal data, rt(data)
  end

  # ─── Wire format ─────────────────────────────────────────────────────

  def test_compress_stores_original_length
    data       = "hello".b
    compressed = CodingAdventures::LZSS.compress(data)
    stored_len = compressed[0, 4].unpack1("N")
    assert_equal 5, stored_len
  end

  def test_compress_deterministic
    data = "hello world test".b
    assert_equal CodingAdventures::LZSS.compress(data), CodingAdventures::LZSS.compress(data)
  end

  def test_compress_empty_header
    c = CodingAdventures::LZSS.compress("".b)
    orig_len, block_count = c[0, 8].unpack("NN")
    assert_equal 0, orig_len
    assert_equal 0, block_count
    assert_equal 8, c.bytesize
  end

  def test_crafted_large_block_count_safe
    bad_header = [4, 2**30].pack("NN")
    payload    = bad_header + "\x00ABCD"
    result     = CodingAdventures::LZSS.decompress(payload)
    assert_kind_of String, result
  end

  # ─── Compression effectiveness ───────────────────────────────────────

  def test_repetitive_compresses
    data = ("ABC" * 1000).b
    assert CodingAdventures::LZSS.compress(data).bytesize < data.bytesize
  end

  def test_all_same_byte_compresses
    data       = ("\x42" * 10000).b
    compressed = CodingAdventures::LZSS.compress(data)
    assert compressed.bytesize < data.bytesize
    assert_equal data, CodingAdventures::LZSS.decompress(compressed)
  end

end
