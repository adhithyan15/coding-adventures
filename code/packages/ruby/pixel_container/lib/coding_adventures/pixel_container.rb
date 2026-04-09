# frozen_string_literal: true

require_relative "pixel_container/version"

# =============================================================================
# CodingAdventures::PixelContainer — Fixed RGBA8 pixel buffer.
# =============================================================================
#
# A PixelContainer holds a rectangular grid of pixels in RGBA8 format:
# 4 bytes per pixel (Red, Green, Blue, Alpha), each byte 0–255.
#
# Memory layout (row-major, left-to-right, top-to-bottom):
#
#   offset(x, y) = (y * width + x) * 4
#
# Example for a 3×2 image (pixels labeled by (x,y)):
#
#   (0,0)(1,0)(2,0)
#   (0,1)(1,1)(2,1)
#
# Byte layout in `data` string:
#   [R,G,B,A, R,G,B,A, R,G,B,A,  ← row y=0
#    R,G,B,A, R,G,B,A, R,G,B,A]  ← row y=1
#
# The `data` field is a binary String (encoding: ASCII-8BIT / BINARY).
# We use String#getbyte and String#setbyte for O(1) byte access — faster
# than slicing or unpacking.
#
# Out-of-bounds reads return [0,0,0,0]. Out-of-bounds writes are no-ops.
# =============================================================================

module CodingAdventures
  module PixelContainer
    # -------------------------------------------------------------------------
    # Container
    #
    # A Struct groups the three fields tightly. Struct members are readable and
    # writable; width and height are treated as immutable by convention.
    #
    # Fields:
    #   width  [Integer] — number of columns (pixels per row)
    #   height [Integer] — number of rows
    #   data   [String]  — binary buffer of (width * height * 4) bytes
    # -------------------------------------------------------------------------
    Container = Struct.new(:width, :height, :data) do
      # Returns a human-readable summary, useful for debugging.
      # Example: "#<PixelContainer 640×480>"
      def to_s
        "#<PixelContainer #{width}×#{height}>"
      end

      # Total number of pixels in the buffer.
      def pixel_count
        width * height
      end

      # Total number of bytes in the buffer.
      def byte_count
        width * height * 4
      end
    end

    # -------------------------------------------------------------------------
    # ImageCodec — mixin interface for image format implementations.
    #
    # Any object that includes this module promises to implement three methods:
    #
    #   mime_type  → String   (e.g. "image/bmp", "image/x-portable-pixmap")
    #   encode(container) → String   (binary bytes of the encoded file)
    #   decode(data)      → Container
    #
    # This is an interface pattern common in Java (Comparable, Serializable).
    # Ruby uses modules for this; the module body is empty because Ruby cannot
    # enforce method signatures at compile time, but the module acts as a marker
    # and documentation anchor.
    # -------------------------------------------------------------------------
    module ImageCodec
      # Implementers define:
      #   def mime_type; end
      #   def encode(container); end
      #   def decode(data); end
    end

    # -------------------------------------------------------------------------
    # create(width, height) → Container
    #
    # Allocates a zeroed RGBA8 buffer for a width×height image.
    # All pixels start as transparent black: R=0, G=0, B=0, A=0.
    #
    # The string literal "\x00" repeated (width * height * 4) times creates a
    # binary string of the correct size. We force encoding to BINARY (ASCII-8BIT)
    # so byte operations never trigger encoding conversion errors.
    # -------------------------------------------------------------------------
    def self.create(width, height)
      raise ArgumentError, "width must be positive" unless width.positive?
      raise ArgumentError, "height must be positive" unless height.positive?

      # Allocate a zeroed buffer. "\x00".b creates a binary-encoded null byte;
      # multiplying by the total byte count fills the entire buffer with zeros.
      data = ("\x00" * (width * height * 4)).b
      Container.new(width, height, data)
    end

    # -------------------------------------------------------------------------
    # pixel_at(container, x, y) → [r, g, b, a]
    #
    # Returns the RGBA bytes of the pixel at column x, row y.
    # Returns [0, 0, 0, 0] for out-of-bounds coordinates (silent clamp).
    #
    # We calculate the byte offset as (y * width + x) * 4.
    # String#getbyte(i) returns the integer value of byte i, range 0–255.
    # -------------------------------------------------------------------------
    def self.pixel_at(container, x, y)
      # Bounds check: reject negative coords and coords beyond the image edge.
      return [0, 0, 0, 0] if x < 0 || y < 0 || x >= container.width || y >= container.height

      offset = (y * container.width + x) * 4
      [
        container.data.getbyte(offset),
        container.data.getbyte(offset + 1),
        container.data.getbyte(offset + 2),
        container.data.getbyte(offset + 3)
      ]
    end

    # -------------------------------------------------------------------------
    # set_pixel(container, x, y, r, g, b, a) → nil
    #
    # Writes one RGBA pixel at (x, y). No-op for out-of-bounds coordinates.
    # Each channel value is clamped to 0–255 via `& 0xFF` (bitwise AND mask),
    # which discards anything above byte range cleanly.
    #
    # String#setbyte(i, v) writes integer v into byte position i in place.
    # -------------------------------------------------------------------------
    def self.set_pixel(container, x, y, r, g, b, a)
      return if x < 0 || y < 0 || x >= container.width || y >= container.height

      offset = (y * container.width + x) * 4
      container.data.setbyte(offset,     r & 0xFF)
      container.data.setbyte(offset + 1, g & 0xFF)
      container.data.setbyte(offset + 2, b & 0xFF)
      container.data.setbyte(offset + 3, a & 0xFF)
      nil
    end

    # -------------------------------------------------------------------------
    # fill_pixels(container, r, g, b, a) → nil
    #
    # Sets every pixel in the buffer to the given RGBA colour. Useful for
    # clearing a canvas or creating a solid-colour background.
    #
    # We iterate over every (x, y) pair. An alternative (faster) approach would
    # be to build a 4-byte pattern and replicate it, but the simple loop is
    # easy to read and fast enough for educational purposes.
    # -------------------------------------------------------------------------
    def self.fill_pixels(container, r, g, b, a)
      rb = r & 0xFF
      gb = g & 0xFF
      bb = b & 0xFF
      ab = a & 0xFF

      # Write one RGBA tuple per pixel, advancing 4 bytes at a time.
      total_pixels = container.width * container.height
      total_pixels.times do |i|
        offset = i * 4
        container.data.setbyte(offset,     rb)
        container.data.setbyte(offset + 1, gb)
        container.data.setbyte(offset + 2, bb)
        container.data.setbyte(offset + 3, ab)
      end
      nil
    end
  end
end
