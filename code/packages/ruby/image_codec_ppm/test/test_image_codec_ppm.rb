# frozen_string_literal: true

require "minitest/autorun"

$LOAD_PATH.unshift File.join(__dir__, "../../pixel_container/lib")
$LOAD_PATH.unshift File.join(__dir__, "../lib")

require "coding_adventures/pixel_container"
require "coding_adventures/image_codec_ppm"

PC  = CodingAdventures::PixelContainer
PPM = CodingAdventures::ImageCodecPpm

# =============================================================================
# Tests for CodingAdventures::ImageCodecPpm
# =============================================================================

class TestPpmCodecClass < Minitest::Test
  def test_mime_type
    codec = PPM::PpmCodec.new
    assert_equal "image/x-portable-pixmap", codec.mime_type
  end

  def test_codec_encode_returns_string
    codec  = PPM::PpmCodec.new
    canvas = PC.create(2, 2)
    assert_instance_of String, codec.encode(canvas)
  end

  def test_codec_roundtrip
    codec  = PPM::PpmCodec.new
    canvas = PC.create(2, 2)
    PC.set_pixel(canvas, 0, 0, 100, 150, 200, 255)
    result = codec.decode(codec.encode(canvas))
    assert_equal [100, 150, 200, 255], PC.pixel_at(result, 0, 0)
  end
end

class TestPpmHeader < Minitest::Test
  def setup
    @canvas = PC.create(3, 2)
    @data   = PPM.encode_ppm(@canvas)
  end

  def test_magic_is_p6
    assert @data.start_with?("P6\n")
  end

  def test_header_contains_dimensions
    header_line = @data.lines[1]
    assert_equal "3 2\n", header_line
  end

  def test_header_contains_maxval
    assert_includes @data.lines[2], "255"
  end

  def test_pixel_data_length
    # Header is "P6\n3 2\n255\n" = 12 bytes; pixels = 3*2*3 = 18 bytes
    expected_size = "P6\n3 2\n255\n".bytesize + 3 * 2 * 3
    assert_equal expected_size, @data.bytesize
  end

  def test_encoding_is_binary
    assert_equal Encoding::ASCII_8BIT, @data.encoding
  end
end

class TestPpmEncodePixelOrder < Minitest::Test
  def test_first_pixel_bytes_are_rgb
    canvas = PC.create(1, 1)
    PC.set_pixel(canvas, 0, 0, 10, 20, 30, 255)
    data = PPM.encode_ppm(canvas)
    # Header is "P6\n1 1\n255\n" = 11 bytes; pixel at offset 11
    offset = "P6\n1 1\n255\n".bytesize
    assert_equal 10, data.getbyte(offset)
    assert_equal 20, data.getbyte(offset + 1)
    assert_equal 30, data.getbyte(offset + 2)
  end

  def test_alpha_is_dropped
    canvas = PC.create(1, 1)
    PC.set_pixel(canvas, 0, 0, 1, 2, 3, 99)
    data  = PPM.encode_ppm(canvas)
    # Only 3 bytes of pixel data
    header_size = "P6\n1 1\n255\n".bytesize
    assert_equal 3, data.bytesize - header_size
  end
end

class TestPpmRoundtrip < Minitest::Test
  def test_roundtrip_1x1
    canvas = PC.create(1, 1)
    PC.set_pixel(canvas, 0, 0, 255, 128, 0, 255)
    result = PPM.decode_ppm(PPM.encode_ppm(canvas))
    assert_equal [255, 128, 0, 255], PC.pixel_at(result, 0, 0)
  end

  def test_roundtrip_dimensions
    canvas = PC.create(7, 5)
    result = PPM.decode_ppm(PPM.encode_ppm(canvas))
    assert_equal 7, result.width
    assert_equal 5, result.height
  end

  def test_roundtrip_alpha_becomes_255
    canvas = PC.create(2, 2)
    PC.set_pixel(canvas, 0, 0, 10, 20, 30, 0)  # alpha 0 → becomes 255 after roundtrip
    result = PPM.decode_ppm(PPM.encode_ppm(canvas))
    r, g, b, a = PC.pixel_at(result, 0, 0)
    assert_equal [10, 20, 30, 255], [r, g, b, a]
  end

  def test_roundtrip_full_grid
    canvas = PC.create(6, 6)
    (0..5).each do |x|
      (0..5).each do |y|
        PC.set_pixel(canvas, x, y, x * 40, y * 40, 0, 255)
      end
    end
    result = PPM.decode_ppm(PPM.encode_ppm(canvas))
    (0..5).each do |x|
      (0..5).each do |y|
        assert_equal [x * 40, y * 40, 0, 255], PC.pixel_at(result, x, y)
      end
    end
  end

  def test_roundtrip_all_corners
    canvas = PC.create(4, 4)
    PC.set_pixel(canvas, 0, 0, 255, 0,   0, 255)
    PC.set_pixel(canvas, 3, 0, 0,   255, 0, 255)
    PC.set_pixel(canvas, 0, 3, 0,   0, 255, 255)
    PC.set_pixel(canvas, 3, 3, 128, 128, 128, 255)
    result = PPM.decode_ppm(PPM.encode_ppm(canvas))
    assert_equal [255, 0,   0,   255], PC.pixel_at(result, 0, 0)
    assert_equal [0,   255, 0,   255], PC.pixel_at(result, 3, 0)
    assert_equal [0,   0,   255, 255], PC.pixel_at(result, 0, 3)
    assert_equal [128, 128, 128, 255], PC.pixel_at(result, 3, 3)
  end
end

class TestPpmDecodeComments < Minitest::Test
  def test_decode_with_comment_line
    # Manually construct a PPM with a comment line.
    ppm = "P6\n# created by test\n2 2\n255\n" + ("\xFF\x00\x00" * 4)
    result = PPM.decode_ppm(ppm)
    assert_equal 2, result.width
    assert_equal 2, result.height
    assert_equal [255, 0, 0, 255], PC.pixel_at(result, 0, 0)
  end

  def test_decode_with_multiple_comment_lines
    ppm = "P6\n# line 1\n# line 2\n1 1\n255\n\x01\x02\x03"
    result = PPM.decode_ppm(ppm)
    assert_equal [1, 2, 3, 255], PC.pixel_at(result, 0, 0)
  end
end

class TestPpmDecodeErrors < Minitest::Test
  def test_raises_on_wrong_magic
    assert_raises(ArgumentError) { PPM.decode_ppm("P3\n1 1\n255\n\xFF\x00\x00") }
  end

  def test_raises_on_wrong_maxval
    assert_raises(ArgumentError) { PPM.decode_ppm("P6\n1 1\n256\n\xFF\x00\x00") }
  end

  def test_raises_on_truncated_pixel_data
    assert_raises(ArgumentError) { PPM.decode_ppm("P6\n2 2\n255\n\xFF") }
  end
end
