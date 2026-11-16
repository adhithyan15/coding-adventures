# frozen_string_literal: true

require_relative "test_helper"
require "json"
require "zlib"

class TestCases < Minitest::Test
  FIXTURE_PATH = File.expand_path("../../../../specs/fixtures/image-codec-png-v1/cases.json", __dir__)

  def setup
    @fixtures = JSON.parse(File.read(FIXTURE_PATH))
    @cases = @fixtures.fetch("cases")
  end

  def test_fixture_pins_contract
    limits = @fixtures.fetch("limits")
    assert_equal ImageCodecPng::PNG_MAX_DIMENSION, limits["max_dimension"]
    assert_equal ImageCodecPng::PNG_MAX_PIXELS, limits["default_max_pixels"]
    assert_equal @fixtures.fetch("error_ids"), ImageCodecPng::PNG_ERROR_CODES
    assert_equal 85, @cases.length
  end

  def test_replay_all_85_fixture_cases
    @cases.each do |case_data|
      operation = case_data.fetch("operation")
      expected = case_data.fetch("expected")
      options = case_data["options"]
      max_pixels = options ? options["max_pixels"] : nil
      case_id = case_data.fetch("id")

      case operation
      when "decode"
        png_bytes = [case_data.fetch("png_hex")].pack("H*")
        container = ImageCodecPng.decode_png(png_bytes, max_pixels: max_pixels)
        assert_equal expected["width"], container.width, "Width mismatch for #{case_id}"
        assert_equal expected["height"], container.height, "Height mismatch for #{case_id}"
        assert_equal expected["rgba_hex"], container.data.unpack1("H*"), "RGBA mismatch for #{case_id}"
      when "decode-error"
        png_bytes = [case_data.fetch("png_hex")].pack("H*")
        err = assert_raises(ImageCodecPng::PngError, "Expected PngError for #{case_id}") do
          ImageCodecPng.decode_png(png_bytes, max_pixels: max_pixels)
        end
        assert_equal expected["error_id"], err.code, "Error ID mismatch for #{case_id}"
      when "encode"
        input = case_data.fetch("input")
        container = CodingAdventures::PixelContainer::Container.new(
          input["width"],
          input["height"],
          [input["rgba_hex"]].pack("H*")
        )
        encoded = ImageCodecPng.encode_png(container)
        chunks = parse_chunks(encoded)

        assert_equal expected["chunk_types"], chunks.map(&:first), "Chunk types mismatch for #{case_id}"
        assert_equal expected["bit_depth"], encoded.getbyte(24), "Bit depth mismatch for #{case_id}"
        assert_equal expected["colour_type"], encoded.getbyte(25), "Colour type mismatch for #{case_id}"
        assert_equal expected["interlace"], encoded.getbyte(28), "Interlace mismatch for #{case_id}"

        idat_stream = chunks.each_with_object("".b) { |(type, payload), acc| acc << payload if type == "IDAT" }
        filtered = Zlib.inflate(idat_stream)
        stride = input["width"] * 4
        actual_filter_types = (0...input["height"]).map { |row| filtered.getbyte(row * (stride + 1)) }
        assert_equal expected["filter_types"], actual_filter_types, "Filter types mismatch for #{case_id}"

        recovered_hex = independent_unfilter(filtered, input["width"], input["height"])
        assert_equal input["rgba_hex"], recovered_hex, "Independent oracle RGBA mismatch for #{case_id}"
      when "encode-error"
        input = case_data.fetch("input")
        container = CodingAdventures::PixelContainer::Container.new(
          input["width"],
          input["height"],
          [input["rgba_hex"]].pack("H*")
        )
        err = assert_raises(ImageCodecPng::PngError, "Expected PngError for #{case_id}") do
          ImageCodecPng.encode_png(container)
        end
        assert_equal expected["error_id"], err.code, "Error ID mismatch for #{case_id}"
      when "adler32"
        input_bytes = [case_data.fetch("input_hex")].pack("H*")
        actual_adler = ImageCodecPng.adler32(input_bytes)
        assert_equal expected["adler32_hex"], format("%08x", actual_adler), "Adler-32 mismatch for #{case_id}"
      else
        flunk "Unknown operation #{operation} in #{case_id}"
      end
    end
  end

  private

  def parse_chunks(png_bytes)
    chunks = []
    pos = 8
    total = png_bytes.bytesize
    while pos < total
      length = png_bytes.byteslice(pos, 4).unpack1("N")
      type = png_bytes.byteslice(pos + 4, 4)
      payload = png_bytes.byteslice(pos + 8, length)
      chunks << [type, payload]
      pos += 12 + length
    end
    chunks
  end

  def independent_unfilter(filtered, width, height)
    stride = width * 4
    row_size = stride + 1
    rgba = Array.new(height * stride, 0)
    prior = Array.new(stride, 0)

    height.times do |y|
      offset = y * row_size
      filter_type = filtered.getbyte(offset)
      row = (0...stride).map { |i| filtered.getbyte(offset + 1 + i) }

      case filter_type
      when 0
        # none
      when 1
        (4...stride).each { |i| row[i] = (row[i] + row[i - 4]) & 0xFF }
      when 2
        stride.times { |i| row[i] = (row[i] + prior[i]) & 0xFF }
      when 3
        stride.times do |i|
          left = (i >= 4) ? row[i - 4] : 0
          row[i] = (row[i] + ((left + prior[i]) / 2)) & 0xFF
        end
      when 4
        stride.times do |i|
          left = (i >= 4) ? row[i - 4] : 0
          above = prior[i]
          above_left = (i >= 4) ? prior[i - 4] : 0
          p = left + above - above_left
          pa = (p - left).abs
          pb = (p - above).abs
          pc = (p - above_left).abs
          pred = if pa <= pb && pa <= pc
            left
          elsif pb <= pc
            above
          else
            above_left
          end
          row[i] = (row[i] + pred) & 0xFF
        end
      end

      dest_offset = y * stride
      row.each_with_index { |b, i| rgba[dest_offset + i] = b }
      prior = row
    end

    rgba.pack("C*").unpack1("H*")
  end
end
