# test_lz78.rb — Comprehensive tests for the LZ78 compression implementation.
#
# Test vectors from the CMP01 specification.

require "minitest/autorun"
require_relative "../lib/coding_adventures_lz78"

LZ78 = CodingAdventures::LZ78
Token = LZ78::Token

class TestSpecVectors < Minitest::Test
  def test_empty_input
    assert_equal [], LZ78.encode("")
    assert_equal "".b, LZ78.decode([], original_length: 0)
  end

  def test_single_byte
    tokens = LZ78.encode("A")
    assert_equal [Token.new(0, 65)], tokens
    assert_equal "A".b, LZ78.decode(tokens, original_length: 1)
  end

  def test_no_repetition
    tokens = LZ78.encode("ABCDE")
    assert_equal 5, tokens.length
    tokens.each { |t| assert_equal 0, t.dict_index, "expected all literals" }
    assert_equal "ABCDE".b, LZ78.decode(tokens, original_length: 5)
  end

  def test_aabcbbabc
    want = [
      Token.new(0, 65),
      Token.new(1, 66),
      Token.new(0, 67),
      Token.new(0, 66),
      Token.new(4, 65),
      Token.new(4, 67),
    ]
    got = LZ78.encode("AABCBBABC")
    assert_equal want, got
    assert_equal "AABCBBABC".b, LZ78.decode(got, original_length: 9)
  end

  def test_ababab
    want = [
      Token.new(0, 65),
      Token.new(0, 66),
      Token.new(1, 66),
      Token.new(3,  0),
    ]
    got = LZ78.encode("ABABAB")
    assert_equal want, got
    assert_equal "ABABAB".b, LZ78.decompress(LZ78.compress("ABABAB"))
  end

  def test_all_identical_bytes
    tokens = LZ78.encode("AAAAAAA")
    assert_equal 4, tokens.length
    assert_equal Token.new(0, 65), tokens[0]
    assert_equal Token.new(1, 65), tokens[1]
    assert_equal Token.new(2, 65), tokens[2]
    assert_equal Token.new(1, 0),  tokens[3]
  end

  def test_repeated_pair
    result = LZ78.decompress(LZ78.compress("ABABABAB"))
    assert_equal "ABABABAB".b, result
    # Fewer tokens than 8 literals means compression occurred.
    assert_operator LZ78.encode("ABABABAB").length, :<, 8
  end
end

class TestRoundTrip < Minitest::Test
  CASES = %w[
    A ABCDE AAAAAAA ABABABAB AABCBBABC
    hello\ world ababababab aaaaaaaaaa
  ].map(&:freeze).freeze

  def test_ascii_strings
    CASES.each do |s|
      result = LZ78.decompress(LZ78.compress(s))
      assert_equal s.b, result, "round-trip failed for #{s.inspect}"
    end
  end

  def test_empty_string
    assert_equal "".b, LZ78.decompress(LZ78.compress(""))
  end

  def test_binary_null_bytes
    data = "\x00\x00\x00\xff\xff".b
    assert_equal data, LZ78.decompress(LZ78.compress(data))
  end

  def test_all_bytes
    data = (0..255).map(&:chr).join.b
    assert_equal data, LZ78.decompress(LZ78.compress(data))
  end

  def test_binary_repeat
    data = "\x00\x01\x02\x00\x01\x02".b
    assert_equal data, LZ78.decompress(LZ78.compress(data))
  end
end

class TestParameters < Minitest::Test
  def test_max_dict_size_respected
    tokens = LZ78.encode("ABCABCABCABCABC", max_dict: 10)
    tokens.each do |t|
      assert_operator t.dict_index, :<, 10, "dict_index #{t.dict_index} exceeds max=10"
    end
  end

  def test_max_dict_size_1_all_literals
    tokens = LZ78.encode("AAAA", max_dict: 1)
    tokens.each { |t| assert_equal 0, t.dict_index }
  end

  def test_max_dict_size_large
    data = ("ABC" * 100 + "X")
    assert_equal data.b, LZ78.decompress(LZ78.compress(data, max_dict: 100_000))
  end
end

class TestEdgeCases < Minitest::Test
  def test_single_byte_literal
    assert_equal [Token.new(0, 88)], LZ78.encode("X")
  end

  def test_two_bytes
    tokens = LZ78.encode("AB")
    assert_equal [Token.new(0, 65), Token.new(0, 66)], tokens
  end

  def test_flush_token_round_trip
    assert_equal "ABABAB".b, LZ78.decompress(LZ78.compress("ABABAB"))
  end

  def test_all_null_bytes
    data = ("\x00" * 100).b
    assert_equal data, LZ78.decompress(LZ78.compress(data))
  end

  def test_all_max_bytes
    data = ("\xff" * 100).b
    assert_equal data, LZ78.decompress(LZ78.compress(data))
  end

  def test_very_long_input
    data = ("Hello, World! " * 100 + (0..255).map(&:chr).join).b
    assert_equal data, LZ78.decompress(LZ78.compress(data))
  end
end

class TestSerialisation < Minitest::Test
  def test_format_size
    compressed = LZ78.compress("AB")
    tokens = LZ78.encode("AB")
    assert_equal 8 + tokens.length * 4, compressed.bytesize
  end

  def test_all_spec_vectors
    %w[A ABCDE AAAAAAA ABABABAB AABCBBABC].each do |v|
      assert_equal v.b, LZ78.decompress(LZ78.compress(v)), "failed for #{v}"
    end
  end

  def test_decompress_empty
    assert_equal "".b, LZ78.decompress(LZ78.compress(""))
  end

  def test_deterministic
    data = "hello world test data repeated repeated"
    assert_equal LZ78.compress(data), LZ78.compress(data)
  end
end

class TestBehaviour < Minitest::Test
  def test_repetitive_data_compresses
    data = "ABC" * 1000
    assert_operator LZ78.compress(data).bytesize, :<, data.bytesize
  end

  def test_incompressible_bound
    data = (0..255).map(&:chr).join
    compressed = LZ78.compress(data)
    assert_operator compressed.bytesize, :<=, 4 * data.bytesize + 10
  end

  def test_all_same_byte_compresses
    data = "A" * 10_000
    compressed = LZ78.compress(data)
    assert_operator compressed.bytesize, :<, data.bytesize
    assert_equal data.b, LZ78.decompress(compressed)
  end
end
