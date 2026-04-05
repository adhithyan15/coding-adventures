# frozen_string_literal: true

require "minitest/autorun"

$LOAD_PATH.unshift File.join(__dir__, "../../pixel_container/lib")
$LOAD_PATH.unshift File.join(__dir__, "../lib")

require "coding_adventures/pixel_container"
require "coding_adventures/image_codec_qoi"

PC  = CodingAdventures::PixelContainer
QOI = CodingAdventures::ImageCodecQoi

# =============================================================================
# Tests for CodingAdventures::ImageCodecQoi
# =============================================================================

class TestQoiCodecClass < Minitest::Test
  def test_mime_type
    assert_equal "image/qoi", QOI::QoiCodec.new.mime_type
  end

  def test_codec_roundtrip
    codec  = QOI::QoiCodec.new
    canvas = PC.create(2, 2)
    PC.set_pixel(canvas, 0, 0, 10, 20, 30, 200)
    result = codec.decode(codec.encode(canvas))
    assert_equal [10, 20, 30, 200], PC.pixel_at(result, 0, 0)
  end
end

class TestQoiHelpers < Minitest::Test
  def test_pixel_hash_formula
    # index = (r*3 + g*5 + b*7 + a*11) % 64
    assert_equal (255 * 3 + 0 * 5 + 0 * 7 + 255 * 11) % 64, QOI.pixel_hash(255, 0, 0, 255)
    assert_equal (0 * 3 + 0 * 5 + 0 * 7 + 0 * 11) % 64,     QOI.pixel_hash(0, 0, 0, 0)
  end

  def test_pixel_hash_in_range
    256.times do |r|
      assert QOI.pixel_hash(r, 0, 0, 0) < 64
    end
  end

  def test_wrap_positive_small
    assert_equal 1, QOI.wrap(1)
    assert_equal 0, QOI.wrap(0)
  end

  def test_wrap_negative_small
    assert_equal(-1, QOI.wrap(255))
    assert_equal(-2, QOI.wrap(254))
  end

  def test_wrap_symmetric
    assert_equal(-128, QOI.wrap(128))
    assert_equal 127, QOI.wrap(127)
  end
end

class TestQoiHeader < Minitest::Test
  def setup
    @canvas = PC.create(4, 3)
    @data   = QOI.encode_qoi(@canvas)
  end

  def test_magic_bytes
    assert_equal "qoif", @data.byteslice(0, 4)
  end

  def test_width_big_endian
    assert_equal 4, @data.byteslice(4, 4).unpack1("N")
  end

  def test_height_big_endian
    assert_equal 3, @data.byteslice(8, 4).unpack1("N")
  end

  def test_channels_is_4
    assert_equal 4, @data.getbyte(12)
  end

  def test_colorspace_is_0
    assert_equal 0, @data.getbyte(13)
  end

  def test_end_marker_present
    assert @data.end_with?(QOI::END_MARKER)
  end
end

class TestQoiRoundtrip < Minitest::Test
  def test_roundtrip_1x1
    canvas = PC.create(1, 1)
    PC.set_pixel(canvas, 0, 0, 100, 150, 200, 255)
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    assert_equal [100, 150, 200, 255], PC.pixel_at(result, 0, 0)
  end

  def test_roundtrip_dimensions
    canvas = PC.create(5, 7)
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    assert_equal 5, result.width
    assert_equal 7, result.height
  end

  def test_roundtrip_all_black
    canvas = PC.create(4, 4)
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    (0..3).each do |x|
      (0..3).each do |y|
        assert_equal [0, 0, 0, 0], PC.pixel_at(result, x, y)
      end
    end
  end

  def test_roundtrip_solid_colour
    canvas = PC.create(8, 8)
    PC.fill_pixels(canvas, 255, 128, 0, 255)
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    (0..7).each do |x|
      (0..7).each do |y|
        assert_equal [255, 128, 0, 255], PC.pixel_at(result, x, y)
      end
    end
  end

  def test_roundtrip_full_grid
    canvas = PC.create(8, 8)
    (0..7).each do |x|
      (0..7).each do |y|
        r = (x * 31) & 0xFF
        g = (y * 37) & 0xFF
        b = ((x + y) * 13) & 0xFF
        PC.set_pixel(canvas, x, y, r, g, b, 255)
      end
    end
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    (0..7).each do |x|
      (0..7).each do |y|
        r = (x * 31) & 0xFF
        g = (y * 37) & 0xFF
        b = ((x + y) * 13) & 0xFF
        assert_equal [r, g, b, 255], PC.pixel_at(result, x, y)
      end
    end
  end

  def test_roundtrip_alpha_channel
    canvas = PC.create(4, 1)
    PC.set_pixel(canvas, 0, 0, 255, 0,   0,   0)
    PC.set_pixel(canvas, 1, 0, 0,   255, 0,   64)
    PC.set_pixel(canvas, 2, 0, 0,   0,   255, 128)
    PC.set_pixel(canvas, 3, 0, 128, 128, 128, 200)
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    assert_equal [255, 0,   0,   0],   PC.pixel_at(result, 0, 0)
    assert_equal [0,   255, 0,   64],  PC.pixel_at(result, 1, 0)
    assert_equal [0,   0,   255, 128], PC.pixel_at(result, 2, 0)
    assert_equal [128, 128, 128, 200], PC.pixel_at(result, 3, 0)
  end
end

class TestQoiOps < Minitest::Test
  # Test that a solid image (all same pixel) produces a RUN chunk.
  def test_run_op_produces_compact_output
    canvas = PC.create(62, 1)
    PC.fill_pixels(canvas, 50, 50, 50, 255)
    data = QOI.encode_qoi(canvas)
    # 14 header + at most a few bytes for the first pixel + 1 RUN byte + 8 end marker
    # = much smaller than 62*4 = 248 bytes
    assert data.bytesize < 30
  end

  def test_run_max_62_pixels
    # 63 identical pixels should produce TWO run chunks (62 + 1)
    canvas = PC.create(63, 1)
    PC.fill_pixels(canvas, 10, 20, 30, 255)
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    (0..62).each { |x| assert_equal [10, 20, 30, 255], PC.pixel_at(result, x, 0) }
  end

  def test_index_op_reuses_cached_pixel
    # Create a pattern where a pixel repeats non-consecutively (forces INDEX op)
    canvas = PC.create(4, 1)
    PC.set_pixel(canvas, 0, 0, 200, 100, 50, 255)  # pixel A
    PC.set_pixel(canvas, 1, 0, 10,  20,  30, 255)  # pixel B
    PC.set_pixel(canvas, 2, 0, 200, 100, 50, 255)  # pixel A again → INDEX
    PC.set_pixel(canvas, 3, 0, 10,  20,  30, 255)  # pixel B again → INDEX
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    assert_equal [200, 100, 50, 255], PC.pixel_at(result, 0, 0)
    assert_equal [10,  20,  30, 255], PC.pixel_at(result, 1, 0)
    assert_equal [200, 100, 50, 255], PC.pixel_at(result, 2, 0)
    assert_equal [10,  20,  30, 255], PC.pixel_at(result, 3, 0)
  end

  def test_diff_op_small_deltas
    # Sequential pixels with deltas in -2..1 per channel should use DIFF
    canvas = PC.create(4, 1)
    PC.set_pixel(canvas, 0, 0, 100, 100, 100, 255)
    PC.set_pixel(canvas, 1, 0, 101, 99,  100, 255)  # dr=+1, dg=-1, db=0 → all in -2..1
    PC.set_pixel(canvas, 2, 0, 102, 98,  101, 255)
    PC.set_pixel(canvas, 3, 0, 100, 97,  100, 255)  # dr=-2, dg=-1, db=-1
    result = QOI.decode_qoi(QOI.encode_qoi(canvas))
    assert_equal [100, 100, 100, 255], PC.pixel_at(result, 0, 0)
    assert_equal [101, 99,  100, 255], PC.pixel_at(result, 1, 0)
    assert_equal [102, 98,  101, 255], PC.pixel_at(result, 2, 0)
    assert_equal [100, 97,  100, 255], PC.pixel_at(result, 3, 0)
  end

  def test_luma_op_medium_green_delta
    # A pixel with dg within -32..31 but outside -2..1, dr_dg and db_dg in -8..7
    canvas = PC.create(2, 1)
    PC.set_pixel(canvas, 0, 0, 128, 128, 128, 255)
    PC.set_pixel(canvas, 1, 0, 130, 148, 132, 255)  # dg=20, dr_dg=-18 → too big for DIFF, good for LUMA
    # Actually dr=2, dg=20, db=4 → dr_dg=2-20=-18 which is outside -8..7 for LUMA
    # Let's use: dg=10, dr=12, db=8 → dr_dg=2, db_dg=-2 — fits LUMA
    canvas2 = PC.create(2, 1)
    PC.set_pixel(canvas2, 0, 0, 100, 100, 100, 255)
    PC.set_pixel(canvas2, 1, 0, 112, 110, 98,  255)  # dg=10, dr_dg=2, db_dg=-2
    result = QOI.decode_qoi(QOI.encode_qoi(canvas2))
    assert_equal [100, 100, 100, 255], PC.pixel_at(result, 0, 0)
    assert_equal [112, 110, 98,  255], PC.pixel_at(result, 1, 0)
  end
end

class TestQoiDecodeErrors < Minitest::Test
  def test_raises_on_short_data
    assert_raises(ArgumentError) { QOI.decode_qoi("qoif") }
  end

  def test_raises_on_bad_magic
    bad = "XXXX" + ("\x00" * 10)
    assert_raises(ArgumentError) { QOI.decode_qoi(bad) }
  end
end
