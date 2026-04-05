# frozen_string_literal: true

require "coding_adventures/pixel_container"

# =============================================================================
# CodingAdventures::ImageCodecPpm — PPM (Portable Pixmap) encoder/decoder.
# =============================================================================
#
# PPM (P6) is the simplest binary colour image format, part of the Netpbm suite.
# It uses a plain-text header followed by raw binary RGB pixels.
#
# File structure:
#
#   P6\n
#   <width> <height>\n
#   255\n
#   <raw RGB bytes, 3 bytes per pixel, top-to-bottom, left-to-right>
#
# Key properties:
#   - No alpha channel: pixels are stored as 3 bytes (R, G, B).
#   - Max value is always 255 (we only produce and accept this).
#   - The header may contain comment lines starting with '#', which we skip.
#   - Top-left pixel comes first in the data stream.
#
# Compared to BMP, PPM is far simpler: the header is human-readable text,
# and pixel data follows directly after the final newline of the header.
#
# Alpha handling:
#   - encode_ppm:  drops the alpha channel (only R, G, B written).
#   - decode_ppm:  reconstructed pixels have A = 255 (fully opaque).
# =============================================================================

module CodingAdventures
  module ImageCodecPpm
    MAX_DIMENSION = 16384

    # -------------------------------------------------------------------------
    # PpmCodec — satisfies the ImageCodec mixin interface.
    # -------------------------------------------------------------------------
    class PpmCodec
      include CodingAdventures::PixelContainer::ImageCodec

      def mime_type
        "image/x-portable-pixmap"
      end

      # @param container [CodingAdventures::PixelContainer::Container]
      # @return [String] binary PPM file content
      def encode(container)
        ImageCodecPpm.encode_ppm(container)
      end

      # @param data [String] binary PPM file content
      # @return [CodingAdventures::PixelContainer::Container]
      def decode(data)
        ImageCodecPpm.decode_ppm(data)
      end
    end

    # =========================================================================
    # encode_ppm(container) → String (binary)
    #
    # Produces a P6 PPM file from an RGBA8 Container.
    # Alpha is silently dropped; only R, G, B bytes are written.
    # =========================================================================
    def self.encode_ppm(container)
      width  = container.width
      height = container.height

      # -----------------------------------------------------------------------
      # Header: three text lines ending with newline.
      # "P6" identifies binary PPM.
      # Width and height are space-separated on one line.
      # "255" is the maximum colour component value.
      # -----------------------------------------------------------------------
      header = "P6\n#{width} #{height}\n255\n"

      # -----------------------------------------------------------------------
      # Pixel data: R, G, B bytes for each pixel, row by row.
      # No padding; each row is exactly width*3 bytes.
      # -----------------------------------------------------------------------
      pc     = CodingAdventures::PixelContainer
      pixels = "".b
      height.times do |y|
        width.times do |x|
          r, g, b, = pc.pixel_at(container, x, y)
          pixels << [r, g, b].pack("CCC")
        end
      end

      header.b + pixels
    end

    # =========================================================================
    # decode_ppm(data) → Container
    #
    # Parses a P6 PPM file and returns an RGBA8 Container (A=255 everywhere).
    #
    # Algorithm:
    #   1. Split on whitespace tokens to read the header fields while skipping
    #      '#' comment lines.
    #   2. Validate magic ("P6"), max value (255).
    #   3. After consuming the header's last whitespace byte, read raw pixels.
    # =========================================================================
    def self.decode_ppm(data)
      # Work with a string that we can index byte by byte.
      s   = data.b
      pos = 0

      # -----------------------------------------------------------------------
      # Header parsing helper: read the next non-comment token.
      #
      # PPM headers may contain comment lines. A token is a maximal sequence
      # of non-whitespace bytes. If we hit '#', skip to end of line.
      # -----------------------------------------------------------------------
      read_token = lambda do
        # Skip whitespace and comments.
        loop do
          raise ArgumentError, "Unexpected end of PPM data" if pos >= s.bytesize

          byte = s.getbyte(pos)
          if byte == 0x23  # '#' — skip to end of line
            pos += 1 while pos < s.bytesize && s.getbyte(pos) != 0x0A
            pos += 1 if pos < s.bytesize  # consume the newline itself
          elsif byte <= 0x20  # whitespace: space, tab, CR, LF
            pos += 1
          else
            break
          end
        end
        # Accumulate the token bytes.
        token_start = pos
        pos += 1 while pos < s.bytesize && s.getbyte(pos) > 0x20
        s.byteslice(token_start, pos - token_start)
      end

      # -----------------------------------------------------------------------
      # Read header fields: magic, width, height, maxval.
      # -----------------------------------------------------------------------
      magic  = read_token.call
      raise ArgumentError, "Not a P6 PPM file (magic: #{magic.inspect})" unless magic == "P6"

      width  = Integer(read_token.call)
      height = Integer(read_token.call)
      raise ArgumentError, "PPM: invalid dimensions" unless width.positive? && height.positive?
      raise ArgumentError, "PPM: dimensions too large" if width > MAX_DIMENSION || height > MAX_DIMENSION
      maxval = Integer(read_token.call)
      raise ArgumentError, "PPM maxval must be 255 (got #{maxval})" unless maxval == 255

      # After maxval the header ends with exactly one whitespace byte (usually '\n').
      # That byte is the separator before the pixel data; skip it.
      pos += 1

      # -----------------------------------------------------------------------
      # Pixel data: width * height * 3 bytes of raw RGB.
      # -----------------------------------------------------------------------
      expected_bytes = width * height * 3
      available = s.bytesize - pos
      if available < expected_bytes
        raise ArgumentError, "PPM pixel data too short: need #{expected_bytes}, got #{available}"
      end

      pc     = CodingAdventures::PixelContainer
      canvas = pc.create(width, height)

      height.times do |y|
        width.times do |x|
          r = s.getbyte(pos)
          g = s.getbyte(pos + 1)
          b = s.getbyte(pos + 2)
          pos += 3
          # Alpha is not stored in PPM; we reconstruct as fully opaque (255).
          pc.set_pixel(canvas, x, y, r, g, b, 255)
        end
      end

      canvas
    end
  end
end
