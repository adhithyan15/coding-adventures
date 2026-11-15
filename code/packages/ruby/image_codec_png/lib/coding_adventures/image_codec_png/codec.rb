# frozen_string_literal: true

require "coding_adventures/pixel_container"
require_relative "constants"
require_relative "decoder"
require_relative "encoder"

module CodingAdventures
  module ImageCodecPng
    # Adapter class satisfying CodingAdventures::PixelContainer::ImageCodec.
    class PngCodec
      include CodingAdventures::PixelContainer::ImageCodec

      attr_reader :max_pixels

      # Initialize a PNG codec with an optional validated pixel count limit.
      def initialize(max_pixels: nil)
        @max_pixels = Decoder.validate_max_pixels(max_pixels)
      end

      # MIME type for PNG images.
      def mime_type
        Constants::MIME_TYPE_PNG
      end

      # Encodes an RGBA8 PixelContainer::Container to binary PNG bytes.
      def encode(container)
        Encoder.encode(container)
      end

      # Decodes binary PNG bytes into an RGBA8 PixelContainer::Container.
      def decode(data)
        Decoder.decode(data, max_pixels: @max_pixels)
      end
    end
  end
end
