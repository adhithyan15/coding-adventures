# frozen_string_literal: true

require "coding_adventures/pixel_container"
require_relative "coding_adventures/paint_codec_png_native/version"

begin
  require "paint_codec_png_native"
rescue LoadError
  nil
end

module CodingAdventures
  module PaintCodecPngNative
    class Error < StandardError; end

    module_function

    def available?
      respond_to?(:encode_rgba8_native)
    end

    def encode(pixels)
      raise Error, "paint_codec_png_native extension is not available" unless available?

      encode_rgba8_native(pixels.width, pixels.height, pixels.data.bytes).b
    end
  end
end
