# frozen_string_literal: true

require_relative "test_helper"
require "zlib"

class TestImageCodecPng < Minitest::Test
  def test_codec_adapter_interface
    codec = ImageCodecPng::PngCodec.new
    assert_equal "image/png", codec.mime_type

    container = CodingAdventures::PixelContainer.create(2, 2)
    4.times { |i| container.data.setbyte(i * 4 + 3, 255) }

    encoded = codec.encode(container)
    assert_instance_of String, encoded
    assert_equal Encoding::BINARY, encoded.encoding

    decoded = codec.decode(encoded)
    assert_instance_of CodingAdventures::PixelContainer::Container, decoded
    assert_equal 2, decoded.width
    assert_equal 2, decoded.height
    assert_equal container.data, decoded.data
  end

  def test_codec_max_pixels_limit
    bounded_codec = ImageCodecPng::PngCodec.new(max_pixels: 50)
    assert_equal 50, bounded_codec.max_pixels

    # Within limit: 5x5 = 25 pixels
    small_container = CodingAdventures::PixelContainer.create(5, 5)
    25.times { |i| small_container.data.setbyte(i * 4 + 3, 255) }
    small_png = bounded_codec.encode(small_container)
    assert_equal 25, bounded_codec.decode(small_png).pixel_count

    # Exceeding limit: 10x10 = 100 pixels
    large_container = CodingAdventures::PixelContainer.create(10, 10)
    100.times { |i| large_container.data.setbyte(i * 4 + 3, 255) }
    large_png = ImageCodecPng.encode_png(large_container)

    err = assert_raises(ImageCodecPng::PngError) do
      bounded_codec.decode(large_png)
    end
    assert_equal "pixel-limit", err.code

    # Invalid limits in constructor
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng::PngCodec.new(max_pixels: 0) }
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng::PngCodec.new(max_pixels: -10) }
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng::PngCodec.new(max_pixels: 50.5) }
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng::PngCodec.new(max_pixels: "50") }
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng::PngCodec.new(max_pixels: 33_554_433) }
  end

  def test_singleton_codec
    codec1 = ImageCodecPng.codec
    codec2 = ImageCodecPng.codec
    assert_same codec1, codec2
    assert_instance_of ImageCodecPng::PngCodec, codec1
  end

  def test_png_error_value_semantics
    err1 = ImageCodecPng::PngError.new("file-too-short")
    err2 = ImageCodecPng::PngError.new("file-too-short")
    err3 = ImageCodecPng::PngError.new("invalid-signature")

    assert_equal err1, err2
    assert err1.eql?(err2)
    assert_equal err1.hash, err2.hash
    refute_equal err1, err3
    refute err1.eql?(err3)
    refute_equal err1, "file-too-short"
  end

  def test_adler32_cross_chunk_boundary
    # Standard RFC test vector
    assert_equal 0x11E60398, ImageCodecPng.adler32("Wikipedia")

    # Empty string
    assert_equal 1, ImageCodecPng.adler32("")

    # Multi-chunk boundary test (> 5552 bytes)
    large_data = (0...12_000).map { |i| (i * 31) & 0xFF }.pack("C*")
    expected = Zlib.adler32(large_data)
    assert_equal expected, ImageCodecPng.adler32(large_data)
  end

  def test_encoder_validations
    # Invalid dimensions
    bad1 = CodingAdventures::PixelContainer::Container.new(0, 5, "")
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad1) }

    bad2 = CodingAdventures::PixelContainer::Container.new(-1, 5, "")
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad2) }

    bad3 = CodingAdventures::PixelContainer::Container.new(5.5, 5, "")
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad3) }

    bad4 = CodingAdventures::PixelContainer::Container.new(16_385, 1, "\x00" * 16_385 * 4)
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad4) }

    # Pixel count limit
    bad5 = CodingAdventures::PixelContainer::Container.new(10_000, 4_000, "")
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad5) }

    # Data buffer length mismatch
    bad6 = CodingAdventures::PixelContainer::Container.new(2, 2, "\x00" * 15) # needs 16 bytes
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad6) }

    bad7 = CodingAdventures::PixelContainer::Container.new(2, 2, nil)
    assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(bad7) }
  end

  def test_filter_paeth_edge_cases
    # Equal distance all tie: pa == pb == pc => a
    assert_equal 10, ImageCodecPng::Filter.paeth(10, 10, 10)

    # pa == pb < pc: ties favor a over b
    # a=10, b=10, c=20 => p = 0. pa = 10, pb = 10, pc = 20 => returns a (10)
    assert_equal 10, ImageCodecPng::Filter.paeth(10, 10, 20)

    # pb < pa and pb < pc: returns b
    # a=10, b=20, c=0 => p = 30. pa = 20, pb = 10, pc = 30 => returns b (20)
    assert_equal 20, ImageCodecPng::Filter.paeth(10, 20, 0)

    # pc < pa and pc < pb: returns c
    # a=10, b=30, c=20 => p = 20. pa = 10, pb = 10, pc = 0 => returns c (20)
    assert_equal 20, ImageCodecPng::Filter.paeth(10, 30, 20)
  end

  def test_filter_undo_filter_invalid
    row = [1, 2, 3, 4]
    prior = [0, 0, 0, 0]
    err = assert_raises(ImageCodecPng::PngError) do
      ImageCodecPng::Filter.undo_filter(99, row, prior, 4)
    end
    assert_equal "invalid-filter", err.code
  end

  def test_type_guards
    # decode_png with byte array
    container = CodingAdventures::PixelContainer.create(1, 1)
    container.data.setbyte(3, 255)
    png = ImageCodecPng.encode_png(container)
    decoded = ImageCodecPng.decode_png(png.bytes)
    assert_equal container.data, decoded.data

    # decode_png with non-string, non-byte-array
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(nil) }
    assert_equal "file-too-short", err.code

    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png(12_345) }
    assert_equal "file-too-short", err.code

    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.decode_png([256, 1]) }
    assert_equal "file-too-short", err.code

    # encode_png with object not responding to width/height/data
    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png(nil) }
    assert_equal "invalid-image-dimensions", err.code

    err = assert_raises(ImageCodecPng::PngError) { ImageCodecPng.encode_png("not-a-container") }
    assert_equal "invalid-image-dimensions", err.code
  end
end
