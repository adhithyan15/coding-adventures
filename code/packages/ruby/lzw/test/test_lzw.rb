require "minitest/autorun"
require_relative "../lib/coding_adventures/lzw"

class TestLZW < Minitest::Test
  include CodingAdventures

  # ---- Constants -------------------------------------------------------------

  def test_constants
    assert_equal 256, LZW::CLEAR_CODE
    assert_equal 257, LZW::STOP_CODE
    assert_equal 258, LZW::INITIAL_NEXT
    assert_equal 9,   LZW::INITIAL_CODE_SIZE
    assert_equal 16,  LZW::MAX_CODE_SIZE
  end

  # ---- encode_codes ----------------------------------------------------------

  def test_encode_empty
    codes, orig = LZW.encode_codes([])
    assert_equal 0, orig
    assert_equal LZW::CLEAR_CODE, codes.first
    assert_equal LZW::STOP_CODE,  codes.last
    assert_equal 2, codes.size
  end

  def test_encode_single_byte
    codes, orig = LZW.encode_codes([65])
    assert_equal 1, orig
    assert_equal LZW::CLEAR_CODE, codes.first
    assert_equal LZW::STOP_CODE,  codes.last
    assert_includes codes, 65
  end

  def test_encode_two_distinct
    codes, orig = LZW.encode_codes([65, 66])
    assert_equal 2, orig
    assert_equal [LZW::CLEAR_CODE, 65, 66, LZW::STOP_CODE], codes
  end

  def test_encode_repeated_pair
    codes, _orig = LZW.encode_codes("ABABAB".bytes)
    assert_equal [LZW::CLEAR_CODE, 65, 66, 258, 258, LZW::STOP_CODE], codes
  end

  def test_encode_all_same
    codes, _orig = LZW.encode_codes("AAAAAAA".bytes)
    assert_equal [LZW::CLEAR_CODE, 65, 258, 259, 65, LZW::STOP_CODE], codes
  end

  # ---- decode_codes ----------------------------------------------------------

  def test_decode_empty_stream
    assert_equal [], LZW.decode_codes([LZW::CLEAR_CODE, LZW::STOP_CODE])
  end

  def test_decode_single_byte
    assert_equal [65], LZW.decode_codes([LZW::CLEAR_CODE, 65, LZW::STOP_CODE])
  end

  def test_decode_two_distinct
    assert_equal [65, 66], LZW.decode_codes([LZW::CLEAR_CODE, 65, 66, LZW::STOP_CODE])
  end

  def test_decode_repeated_pair
    result = LZW.decode_codes([LZW::CLEAR_CODE, 65, 66, 258, 258, LZW::STOP_CODE])
    assert_equal "ABABAB".bytes, result
  end

  def test_decode_tricky_token
    result = LZW.decode_codes([LZW::CLEAR_CODE, 65, 258, 259, 65, LZW::STOP_CODE])
    assert_equal "AAAAAAA".bytes, result
  end

  def test_decode_clear_mid_stream
    result = LZW.decode_codes([LZW::CLEAR_CODE, 65, LZW::CLEAR_CODE, 66, LZW::STOP_CODE])
    assert_equal [65, 66], result
  end

  def test_decode_invalid_code_skipped
    result = LZW.decode_codes([LZW::CLEAR_CODE, 9999, 65, LZW::STOP_CODE])
    assert_equal [65], result
  end

  # ---- pack / unpack codes ---------------------------------------------------

  def test_header_original_length
    packed = LZW.pack_codes([LZW::CLEAR_CODE, LZW::STOP_CODE], 42)
    assert_equal 42, packed.unpack1("N")
  end

  def test_roundtrip_pack_unpack_empty
    codes = [LZW::CLEAR_CODE, LZW::STOP_CODE]
    packed = LZW.pack_codes(codes, 0)
    unpacked, orig = LZW.unpack_codes(packed)
    assert_equal 0, orig
    assert_includes unpacked, LZW::CLEAR_CODE
    assert_includes unpacked, LZW::STOP_CODE
  end

  def test_roundtrip_pack_unpack_ababab
    codes = [LZW::CLEAR_CODE, 65, 66, 258, 258, LZW::STOP_CODE]
    packed = LZW.pack_codes(codes, 6)
    unpacked, orig = LZW.unpack_codes(packed)
    assert_equal 6, orig
    assert_equal codes, unpacked
  end

  def test_roundtrip_pack_unpack_all_same
    codes = [LZW::CLEAR_CODE, 65, 258, 259, 65, LZW::STOP_CODE]
    packed = LZW.pack_codes(codes, 7)
    unpacked, orig = LZW.unpack_codes(packed)
    assert_equal 7, orig
    assert_equal codes, unpacked
  end

  def test_truncated_unpack
    result, _orig = LZW.unpack_codes("\x00\x00")
    assert_instance_of Array, result
  end

  # ---- compress / decompress -------------------------------------------------

  def roundtrip(data)
    compressed = LZW.compress(data)
    LZW.decompress(compressed)
  end

  def test_compress_empty
    assert_equal "", roundtrip("")
  end

  def test_compress_single_byte
    assert_equal "A", roundtrip("A")
  end

  def test_compress_two_distinct
    assert_equal "AB", roundtrip("AB")
  end

  def test_compress_repeated_pair
    assert_equal "ABABAB", roundtrip("ABABAB")
  end

  def test_compress_all_same
    assert_equal "AAAAAAA", roundtrip("AAAAAAA")
  end

  def test_compress_long_string
    text = "the quick brown fox jumps over the lazy dog " * 20
    assert_equal text, roundtrip(text)
  end

  def test_compress_binary
    data = (0..255).map(&:chr).join * 4
    assert_equal data, roundtrip(data)
  end

  def test_compress_all_zeros
    data = ("\x00" * 100).b
    assert_equal data, roundtrip(data)
  end

  def test_compress_all_ff
    data = ("\xFF" * 100).b
    assert_equal data, roundtrip(data)
  end

  def test_compress_aababc
    assert_equal "AABABC", roundtrip("AABABC")
  end

  def test_compression_ratio
    data = "ABCABC" * 100
    compressed = LZW.compress(data)
    assert compressed.bytesize < data.bytesize,
           "expected compression: #{compressed.bytesize} < #{data.bytesize}"
  end

  def test_header_stored_length
    data = "hello world"
    compressed = LZW.compress(data)
    assert_equal data.bytesize, compressed.unpack1("N")
  end
end
