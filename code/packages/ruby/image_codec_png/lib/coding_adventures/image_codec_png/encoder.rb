# frozen_string_literal: true

require "coding_adventures_zip"
require_relative "constants"
require_relative "error"
require_relative "chunk"
require_relative "filter"
require_relative "adler32"

module CodingAdventures
  module ImageCodecPng
    # Encodes RGBA8 PixelContainer buffers into deterministic portable PNGs.
    module Encoder
      # Encodes a valid RGBA8 PixelContainer into a PNG binary string.
      def self.encode(pixels)
        unless pixels.respond_to?(:width) && pixels.respond_to?(:height) && pixels.respond_to?(:data)
          raise PngError.new(Constants::ERR_INVALID_IMAGE_DIMENSIONS)
        end

        width = pixels.width
        height = pixels.height

        unless width.is_a?(Integer) && height.is_a?(Integer) &&
            width.positive? && height.positive? &&
            width <= Constants::PNG_MAX_DIMENSION && height <= Constants::PNG_MAX_DIMENSION
          raise PngError.new(Constants::ERR_INVALID_IMAGE_DIMENSIONS)
        end

        pixel_count = width * height
        if pixel_count > Constants::PNG_MAX_PIXELS
          raise PngError.new(Constants::ERR_INVALID_IMAGE_DIMENSIONS)
        end

        data = pixels.data
        unless data.is_a?(String) && data.bytesize == pixel_count * Constants::BYTES_PER_RGBA_PIXEL
          raise PngError.new(Constants::ERR_INVALID_PIXEL_DATA_LENGTH)
        end

        ihdr_payload = [
          width,
          height,
          Constants::SUPPORTED_BIT_DEPTH,
          Constants::COLOUR_TYPE_RGBA,
          Constants::COMPRESSION_METHOD_DEFLATE,
          Constants::FILTER_METHOD_ADAPTIVE,
          Constants::INTERLACE_METHOD_NONE
        ].pack(Constants::IHDR_PACK_FORMAT).b

        filtered = filter_scanlines(data.b, width, height)
        adler = Adler32.compute(filtered)
        adler_bytes = [adler].pack(Constants::ADLER32_PACK_FORMAT).b

        deflated = CodingAdventures::Zip.raw_deflate(filtered)
        zlib_stream = [Constants::ZLIB_CMF_DEFLATE, Constants::ZLIB_FLG_DEFAULT].pack(Constants::ZLIB_HEADER_PACK_FORMAT).b + deflated + adler_bytes

        Constants::PNG_SIGNATURE +
          Chunk.format(Constants::CHUNK_IHDR, ihdr_payload) +
          Chunk.format(Constants::CHUNK_IDAT, zlib_stream) +
          Chunk.format(Constants::CHUNK_IEND)
      end

      # Applies adaptive row filtering across all scanlines.
      def self.filter_scanlines(raw_data, width, height)
        stride = width * Constants::BYTES_PER_RGBA_PIXEL
        bpp = Constants::CHANNELS_RGBA

        prior = Array.new(stride, 0)
        raw = Array.new(stride, 0)
        scratch = Array.new(stride, 0)
        best = Array.new(stride, 0)
        filtered_parts = []

        y = 0
        while y < height
          row_offset = y * stride
          i = 0
          while i < stride
            raw[i] = raw_data.getbyte(row_offset + i)
            i += 1
          end

          filter_type = Filter.choose_filter(raw, prior, bpp, scratch, best)
          filtered_parts << [filter_type].pack(Constants::UINT8_PACK_FORMAT).b
          filtered_parts << best.pack(Constants::BYTE_ARRAY_PACK_FORMAT).b

          prior.replace(raw)
          y += 1
        end

        filtered_parts.join.b
      end
    end
  end
end
