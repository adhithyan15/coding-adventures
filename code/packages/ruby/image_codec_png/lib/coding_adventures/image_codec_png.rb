# frozen_string_literal: true

require_relative "image_codec_png/version"
require_relative "image_codec_png/constants"
require_relative "image_codec_png/messages"
require_relative "image_codec_png/error"
require_relative "image_codec_png/adler32"
require_relative "image_codec_png/filter"
require_relative "image_codec_png/chunk"
require_relative "image_codec_png/decoder"
require_relative "image_codec_png/encoder"
require_relative "image_codec_png/codec"

module CodingAdventures
  # IC18 bounded PNG encoding and decoding over repository primitives.
  module ImageCodecPng
    PNG_MAX_DIMENSION = Constants::PNG_MAX_DIMENSION
    PNG_MAX_PIXELS = Constants::PNG_MAX_PIXELS
    PNG_ERROR_CODES = Constants::PNG_ERROR_CODES

    # Encodes an RGBA8 PixelContainer::Container to binary PNG bytes.
    def self.encode_png(pixels)
      Encoder.encode(pixels)
    end

    # Decodes a bounded, non-interlaced, 8-bit PNG into an RGBA8 PixelContainer::Container.
    def self.decode_png(data, max_pixels: nil)
      Decoder.decode(data, max_pixels: max_pixels)
    end

    # Computes the RFC 1950 Adler-32 checksum.
    def self.adler32(data)
      Adler32.compute(data)
    end

    # Default singleton codec instance.
    def self.codec
      @codec ||= PngCodec.new
    end
  end
end
