# frozen_string_literal: true

require "minitest/autorun"

# Load pixel_container from sibling package directory.
$LOAD_PATH.unshift File.join(__dir__, "../../pixel_container/lib")
$LOAD_PATH.unshift File.join(__dir__, "../lib")

require "coding_adventures/pixel_container"
require "coding_adventures/image_codec_bmp"

PC  = CodingAdventures::PixelContainer
BMP = CodingAdventures::ImageCodecBmp

# =============================================================================
# Tests for CodingAdventures::ImageCodecBmp
# =============================================================================

class TestBmpCodecClass < Minitest::Test
  def test_mime_type
    codec = BMP::BmpCodec.new
    assert_equal "image/bmp", codec.mime_type
  end

  def test_codec_encode_roundtrip
    codec   = BMP::BmpCodec.new
    canvas  = PC.create(2, 2)
    PC.set_pixel(canvas, 0, 0, 10, 20, 30, 255)
    data    = codec.encode(canvas)
    result  = codec.decode(data)
    assert_equal [10, 20, 30, 255], PC.pixel_at(result, 0, 0)
  end
end

class TestBmpMagicAndHeader < Minitest::Test
  def setup
    @canvas = PC.create(2, 2)
    @bytes  = BMP.encode_bmp(@canvas)
  end

  def test_magic_bytes
    assert_equal "BM", @bytes.byteslice(0, 2)
  end

  def test_file_size_field
    # bfSize is at offset 2 (after the 2-byte "BM" magic)
    expected = 54 + 2 * 2 * 4
    assert_equal expected, @bytes.byteslice(2, 4).unpack1("V")
  end

  def test_reserved_fields_are_zero
    # bfReserved1 at offset 6, bfReserved2 at offset 8
    assert_equal 0, @bytes.byteslice(6, 2).unpack1("v")
    assert_equal 0, @bytes.byteslice(8, 2).unpack1("v")
  end

  def test_pixel_offset_is_54
    # bfOffBits at offset 10
    assert_equal 54, @bytes.byteslice(10, 4).unpack1("V")
  end

  def test_info_header_size_is_40
    assert_equal 40, @bytes.byteslice(14, 4).unpack1("V")
  end

  def test_bit_count_is_32
    assert_equal 32, @bytes.byteslice(28, 2).unpack1("v")
  end

  def test_compression_is_zero
    assert_equal 0, @bytes.byteslice(30, 4).unpack1("V")
  end

  def test_bi_height_is_negative_for_top_down
    bi_height = @bytes.byteslice(22, 4).unpack1("i<")
    assert bi_height.negative?, "expected negative biHeight for top-down BMP"
  end
end

class TestBmpEncodeDecodeRoundtrip < Minitest::Test
  def test_roundtrip_1x1
    canvas = PC.create(1, 1)
    PC.set_pixel(canvas, 0, 0, 255, 0, 128, 200)
    result = BMP.decode_bmp(BMP.encode_bmp(canvas))
    assert_equal [255, 0, 128, 200], PC.pixel_at(result, 0, 0)
  end

  def test_roundtrip_preserves_dimensions
    canvas = PC.create(5, 3)
    result = BMP.decode_bmp(BMP.encode_bmp(canvas))
    assert_equal 5, result.width
    assert_equal 3, result.height
  end

  def test_roundtrip_all_corners
    canvas = PC.create(4, 4)
    corners = { [0, 0] => [255, 0, 0, 255], [3, 0] => [0, 255, 0, 255],
                [0, 3] => [0, 0, 255, 255], [3, 3] => [128, 128, 128, 255] }
    corners.each { |(x, y), rgba| PC.set_pixel(canvas, x, y, *rgba) }
    result = BMP.decode_bmp(BMP.encode_bmp(canvas))
    corners.each { |(x, y), rgba| assert_equal rgba, PC.pixel_at(result, x, y) }
  end

  def test_roundtrip_full_grid
    canvas = PC.create(8, 8)
    (0..7).each do |x|
      (0..7).each do |y|
        PC.set_pixel(canvas, x, y, x * 10, y * 10, (x + y) * 5, 255)
      end
    end
    result = BMP.decode_bmp(BMP.encode_bmp(canvas))
    (0..7).each do |x|
      (0..7).each do |y|
        expected = [x * 10, y * 10, (x + y) * 5, 255]
        assert_equal expected, PC.pixel_at(result, x, y), "mismatch at (#{x},#{y})"
      end
    end
  end

  def test_bgra_channel_order_in_raw_bytes
    # Encode a pure red pixel and verify the raw bytes are BGRA (B=0, G=0, R=255, A=255)
    canvas = PC.create(1, 1)
    PC.set_pixel(canvas, 0, 0, 255, 0, 0, 255)
    data = BMP.encode_bmp(canvas)
    # Pixel data starts at offset 54
    assert_equal 0,   data.getbyte(54)     # B = 0
    assert_equal 0,   data.getbyte(55)     # G = 0
    assert_equal 255, data.getbyte(56)     # R = 255
    assert_equal 255, data.getbyte(57)     # A = 255
  end
end

class TestBmpDecodeErrors < Minitest::Test
  def test_raises_on_short_data
    assert_raises(ArgumentError) { BMP.decode_bmp("BM") }
  end

  def test_raises_on_bad_magic
    bad = "XX" + ("\x00" * 52)
    assert_raises(ArgumentError) { BMP.decode_bmp(bad) }
  end

  def test_raises_on_wrong_bit_count
    canvas = PC.create(2, 2)
    data = BMP.encode_bmp(canvas)
    # Patch biBitCount at offset 28 to 24
    data.setbyte(28, 24)
    data.setbyte(29, 0)
    assert_raises(ArgumentError) { BMP.decode_bmp(data) }
  end

  def test_raises_on_nonzero_compression
    canvas = PC.create(2, 2)
    data = BMP.encode_bmp(canvas)
    # Patch biCompression at offset 30 to 1
    data.setbyte(30, 1)
    assert_raises(ArgumentError) { BMP.decode_bmp(data) }
  end
end

class TestBmpBottomUpDecode < Minitest::Test
  # Manually craft a bottom-up BMP (positive biHeight) and verify we flip rows.
  def test_bottom_up_bmp_row_order
    # Build a known top-down BMP, then patch it to bottom-up.
    canvas = PC.create(2, 2)
    # Top row (y=0): red pixels
    PC.set_pixel(canvas, 0, 0, 255, 0, 0, 255)
    PC.set_pixel(canvas, 1, 0, 255, 0, 0, 255)
    # Bottom row (y=1): blue pixels
    PC.set_pixel(canvas, 0, 1, 0, 0, 255, 255)
    PC.set_pixel(canvas, 1, 1, 0, 0, 255, 255)

    data = BMP.encode_bmp(canvas)

    # The encoded BMP is top-down (biHeight = -2). Flip to bottom-up:
    # In bottom-up BMP, pixel row 0 in the file = BOTTOM of image.
    # So we need to swap the two 4-byte-wide rows in the pixel data.
    # Also change biHeight from -2 to +2.
    [data.byteslice(22, 4).unpack1("i<")].tap do |vals|
      data[22, 4] = [2].pack("i<")  # positive height = bottom-up
    end

    # Swap pixel rows in the file: rows are at offsets 54 and 54+8
    row0 = data.byteslice(54, 8)
    row1 = data.byteslice(62, 8)
    data[54, 8] = row1
    data[62, 8] = row0

    result = BMP.decode_bmp(data)
    # After flipping back, top row (y=0) should be red
    assert_equal [255, 0, 0, 255], PC.pixel_at(result, 0, 0)
    # Bottom row (y=1) should be blue
    assert_equal [0, 0, 255, 255], PC.pixel_at(result, 0, 1)
  end
end
