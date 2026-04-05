# frozen_string_literal: true

require "coding_adventures/pixel_container"

# =============================================================================
# CodingAdventures::ImageCodecQoi — QOI (Quite OK Image) encoder/decoder.
# =============================================================================
#
# QOI is a lossless image format designed to be extremely simple to implement
# while still achieving reasonable compression ratios (typically 20-50% vs raw).
# Spec: https://qoiformat.org/qoi-specification.pdf
#
# -----------------------------------------------------------------------------
# File layout
# -----------------------------------------------------------------------------
#
#   ┌──────────────────────────────┐ offset 0
#   │  Header (14 bytes)           │
#   ├──────────────────────────────┤ offset 14
#   │  Chunk stream (variable)     │
#   ├──────────────────────────────┤ end - 8
#   │  End marker (8 bytes)        │
#   └──────────────────────────────┘
#
# Header:
#   magic      4 bytes  ASCII "qoif"
#   width      4 bytes  uint32 big-endian
#   height     4 bytes  uint32 big-endian
#   channels   1 byte   3=RGB, 4=RGBA
#   colorspace 1 byte   0=sRGB+linear-alpha, 1=all-linear (informational only)
#
# End marker: 7 zero bytes + 1 one byte = [0,0,0,0,0,0,0,1]
#
# -----------------------------------------------------------------------------
# Pixel hash (running array index)
# -----------------------------------------------------------------------------
#
#   index = (r*3 + g*5 + b*7 + a*11) % 64
#
# A running array of 64 RGBA pixels is maintained during encode/decode.
# All 64 slots start as [0, 0, 0, 0].
#
# -----------------------------------------------------------------------------
# Chunk types (6 ops)
# -----------------------------------------------------------------------------
#
# QOI_OP_RGB    11111110  b  1-byte tag + 3 bytes RGB (alpha unchanged)
# QOI_OP_RGBA   11111111  b  1-byte tag + 4 bytes RGBA
# QOI_OP_INDEX  00xxxxxx  b  6-bit index into the running array
# QOI_OP_DIFF   01rdgdbd b  2-bit tag + 3×2-bit deltas (bias -2: range -2..1)
# QOI_OP_LUMA   10gggggg drdb  8-bit green-delta (bias -32), 4-bit dr-dg, 4-bit db-dg (bias -8)
# QOI_OP_RUN    11xxxxxx  b  6-bit run length (bias -1: encodes 1..62 as 0..61)
#
# -----------------------------------------------------------------------------
# Delta wrap helper
# -----------------------------------------------------------------------------
#
# Channel differences wrap modulo 256 (two's complement byte arithmetic):
#   wrap(delta) = ((delta & 0xFF) + 128) & 0xFF - 128
#
# This maps any byte delta to the signed range -128..127.
# For QOI_OP_DIFF we only use the -2..1 range; for LUMA, -32..31 (green)
# and -8..7 (red-green, blue-green differentials).
# =============================================================================

module CodingAdventures
  module ImageCodecQoi
    # Magic bytes that identify a QOI file.
    MAGIC = "qoif"

    # End-of-stream marker: 7 zero bytes then 0x01.
    END_MARKER = [0, 0, 0, 0, 0, 0, 0, 1].pack("C*")

    # Chunk tag constants
    TAG_RGB   = 0xFE  # 11111110
    TAG_RGBA  = 0xFF  # 11111111
    TAG_INDEX = 0x00  # 00xxxxxx  (top 2 bits = 00)
    TAG_DIFF  = 0x40  # 01xxxxxx  (top 2 bits = 01)
    TAG_LUMA  = 0x80  # 10xxxxxx  (top 2 bits = 10)
    TAG_RUN   = 0xC0  # 11xxxxxx  (top 2 bits = 11) — but 0xFE/0xFF are RGB/RGBA

    # -------------------------------------------------------------------------
    # QoiCodec — satisfies the ImageCodec mixin interface.
    # -------------------------------------------------------------------------
    class QoiCodec
      include CodingAdventures::PixelContainer::ImageCodec

      def mime_type
        "image/qoi"
      end

      def encode(container)
        ImageCodecQoi.encode_qoi(container)
      end

      def decode(data)
        ImageCodecQoi.decode_qoi(data)
      end
    end

    # =========================================================================
    # pixel_hash(r, g, b, a) → Integer (0..63)
    #
    # QOI's running-array index function. Maps an RGBA pixel to one of 64
    # slots. The prime multipliers (3, 5, 7, 11) spread colours across slots
    # with few collisions.
    # =========================================================================
    def self.pixel_hash(r, g, b, a)
      (r * 3 + g * 5 + b * 7 + a * 11) % 64
    end

    # =========================================================================
    # wrap(delta) → Integer (-128..127)
    #
    # Treats a byte difference as a signed 8-bit value.
    # Example: wrap(255) = -1, wrap(254) = -2, wrap(1) = 1.
    #
    # Step by step:
    #   1. `& 0xFF`  — keep only the low 8 bits (handles negative Ruby ints)
    #   2. `+ 128`   — shift range from 0..255 to 128..383
    #   3. `& 0xFF`  — wrap to 0..255 again
    #   4. `- 128`   — shift back to -128..127
    # =========================================================================
    def self.wrap(delta)
      (((delta & 0xFF) + 128) & 0xFF) - 128
    end

    # =========================================================================
    # encode_qoi(container) → String (binary)
    #
    # QOI encoder. Iterates pixels left-to-right, top-to-bottom and chooses
    # the most compact op for each pixel, in priority order:
    #
    #   1. QOI_OP_RUN   — same pixel as previous, up to 62 in a row
    #   2. QOI_OP_INDEX — pixel exists in running array
    #   3. QOI_OP_DIFF  — small per-channel delta (-2..1)
    #   4. QOI_OP_LUMA  — medium delta with green dominance
    #   5. QOI_OP_RGB   — emit full RGB (alpha unchanged from previous)
    #   6. QOI_OP_RGBA  — emit full RGBA (alpha changed)
    # =========================================================================
    def self.encode_qoi(container)
      width  = container.width
      height = container.height
      pc     = CodingAdventures::PixelContainer

      # -----------------------------------------------------------------------
      # Header
      # -----------------------------------------------------------------------
      header =
        MAGIC.b +
        [width].pack("N") +    # uint32 big-endian
        [height].pack("N") +   # uint32 big-endian
        [4].pack("C") +        # channels = 4 (RGBA)
        [0].pack("C")          # colorspace = 0 (sRGB)

      # -----------------------------------------------------------------------
      # Encoder state
      # -----------------------------------------------------------------------
      # running_array: 64 RGBA pixels, all starting at [0,0,0,0]
      running = Array.new(64) { [0, 0, 0, 0] }

      # previous pixel starts as fully transparent black
      pr, pg, pb, pa = 0, 0, 0, 255

      run_length = 0  # current run of identical pixels
      chunks     = "".b

      flush_run = lambda do
        # Emit accumulated run (run_length is 1..62, stored as 0..61)
        if run_length > 0
          chunks << [TAG_RUN | (run_length - 1)].pack("C")
          run_length = 0
        end
      end

      total_pixels = width * height
      pixel_index  = 0

      height.times do |y|
        width.times do |x|
          r, g, b, a = pc.pixel_at(container, x, y)
          pixel_index += 1

          if r == pr && g == pg && b == pb && a == pa
            # -------------------------------------------------------------------
            # QOI_OP_RUN — same as previous pixel
            #
            # The run counter can hold at most 62 pixels. When it hits 62 we
            # must flush and start fresh (even if the next pixel is also a repeat).
            # -------------------------------------------------------------------
            run_length += 1
            if run_length == 62
              flush_run.call
            end
          else
            # Flush any pending run before emitting a different-pixel chunk.
            flush_run.call

            idx = pixel_hash(r, g, b, a)

            if running[idx] == [r, g, b, a]
              # -----------------------------------------------------------------
              # QOI_OP_INDEX — pixel is in the running array at slot idx
              # -----------------------------------------------------------------
              chunks << [TAG_INDEX | idx].pack("C")
            else
              # Store this pixel in the running array.
              running[idx] = [r, g, b, a]

              # Compute per-channel signed deltas (wrap to -128..127)
              dr = wrap(r - pr)
              dg = wrap(g - pg)
              db = wrap(b - pb)

              if a == pa
                # Alpha hasn't changed — try compact ops first.

                if dr >= -2 && dr <= 1 && dg >= -2 && dg <= 1 && db >= -2 && db <= 1
                  # -------------------------------------------------------------
                  # QOI_OP_DIFF — all deltas fit in 2 bits with bias -2
                  # Biased representation: delta + 2 → 0..3
                  # Packed as:  01_RR_GG_BB  (2+2+2 = 6 bits in low byte)
                  # -------------------------------------------------------------
                  byte = TAG_DIFF | ((dr + 2) << 4) | ((dg + 2) << 2) | (db + 2)
                  chunks << [byte].pack("C")
                else
                  # dr_dg = dr - dg, db_dg = db - dg
                  dr_dg = wrap(dr - dg)
                  db_dg = wrap(db - dg)

                  if dg >= -32 && dg <= 31 && dr_dg >= -8 && dr_dg <= 7 && db_dg >= -8 && db_dg <= 7
                    # -----------------------------------------------------------
                    # QOI_OP_LUMA — medium delta
                    # Byte 1: 10_GGGGGG  (dg biased +32, fits in 6 bits)
                    # Byte 2: RRRR_BBBB  (dr_dg biased +8 in high nibble,
                    #                     db_dg biased +8 in low nibble)
                    # -----------------------------------------------------------
                    chunks << [TAG_LUMA | (dg + 32)].pack("C")
                    chunks << [((dr_dg + 8) << 4) | (db_dg + 8)].pack("C")
                  else
                    # -----------------------------------------------------------
                    # QOI_OP_RGB — full RGB, alpha unchanged
                    # -----------------------------------------------------------
                    chunks << [TAG_RGB, r, g, b].pack("CCCC")
                  end
                end
              else
                # ---------------------------------------------------------------
                # QOI_OP_RGBA — full RGBA (alpha changed)
                # ---------------------------------------------------------------
                chunks << [TAG_RGBA, r, g, b, a].pack("CCCCC")
              end
            end
          end

          pr, pg, pb, pa = r, g, b, a
        end
      end

      # Flush the final run, if any.
      flush_run.call

      header + chunks + END_MARKER.b
    end

    # =========================================================================
    # decode_qoi(data) → Container
    #
    # QOI decoder. Reads the header, then processes chunk bytes until the end
    # marker is found (or the expected pixel count is reached).
    # =========================================================================
    def self.decode_qoi(data)
      s = data.b

      raise ArgumentError, "QOI data too short" if s.bytesize < 14

      # -----------------------------------------------------------------------
      # Parse header
      # -----------------------------------------------------------------------
      magic = s.byteslice(0, 4)
      raise ArgumentError, "Not a QOI file (magic: #{magic.inspect})" unless magic == MAGIC

      width     = s.byteslice(4, 4).unpack1("N")  # uint32 BE
      height    = s.byteslice(8, 4).unpack1("N")
      _channels = s.getbyte(12)  # informational; we always produce RGBA containers
      # colorspace at offset 13 is informational — not validated

      pc     = CodingAdventures::PixelContainer
      canvas = pc.create(width, height)

      # -----------------------------------------------------------------------
      # Decoder state
      # -----------------------------------------------------------------------
      running = Array.new(64) { [0, 0, 0, 0] }
      r, g, b, a = 0, 0, 0, 255  # previous/current pixel

      total_pixels = width * height
      pixel_index  = 0
      pos          = 14  # start of chunk stream

      while pixel_index < total_pixels
        tag = s.getbyte(pos)
        pos += 1

        if tag == TAG_RGBA
          # -------------------------------------------------------------------
          # QOI_OP_RGBA — 5 bytes: tag + R + G + B + A
          # -------------------------------------------------------------------
          r = s.getbyte(pos)
          g = s.getbyte(pos + 1)
          b = s.getbyte(pos + 2)
          a = s.getbyte(pos + 3)
          pos += 4
          running[pixel_hash(r, g, b, a)] = [r, g, b, a]

          x = pixel_index % width
          y = pixel_index / width
          pc.set_pixel(canvas, x, y, r, g, b, a)
          pixel_index += 1

        elsif tag == TAG_RGB
          # -------------------------------------------------------------------
          # QOI_OP_RGB — 4 bytes: tag + R + G + B  (alpha unchanged)
          # -------------------------------------------------------------------
          r = s.getbyte(pos)
          g = s.getbyte(pos + 1)
          b = s.getbyte(pos + 2)
          pos += 3
          running[pixel_hash(r, g, b, a)] = [r, g, b, a]

          x = pixel_index % width
          y = pixel_index / width
          pc.set_pixel(canvas, x, y, r, g, b, a)
          pixel_index += 1

        elsif (tag & 0xC0) == TAG_RUN
          # -------------------------------------------------------------------
          # QOI_OP_RUN — low 6 bits are run length minus 1 (bias -1)
          # Run length encodes 1..62 as 0..61.
          # Note: 0xFE and 0xFF are taken by RGB/RGBA; so max stored value is
          # 0b00111101 = 61 → run of 62.
          # -------------------------------------------------------------------
          run = (tag & 0x3F) + 1
          run.times do
            x = pixel_index % width
            y = pixel_index / width
            pc.set_pixel(canvas, x, y, r, g, b, a)
            pixel_index += 1
          end
          # running array not updated on RUN (the pixel is already there)

        elsif (tag & 0xC0) == TAG_INDEX
          # -------------------------------------------------------------------
          # QOI_OP_INDEX — low 6 bits are the running array index
          # -------------------------------------------------------------------
          idx        = tag & 0x3F
          r, g, b, a = running[idx]

          x = pixel_index % width
          y = pixel_index / width
          pc.set_pixel(canvas, x, y, r, g, b, a)
          pixel_index += 1

        elsif (tag & 0xC0) == TAG_DIFF
          # -------------------------------------------------------------------
          # QOI_OP_DIFF — 1 byte: 01_RR_GG_BB
          # Each 2-bit field is biased by -2 (0 = -2, 1 = -1, 2 = 0, 3 = 1)
          # -------------------------------------------------------------------
          dr = ((tag >> 4) & 0x03) - 2
          dg = ((tag >> 2) & 0x03) - 2
          db = (tag & 0x03) - 2
          r = (r + dr) & 0xFF
          g = (g + dg) & 0xFF
          b = (b + db) & 0xFF
          running[pixel_hash(r, g, b, a)] = [r, g, b, a]

          x = pixel_index % width
          y = pixel_index / width
          pc.set_pixel(canvas, x, y, r, g, b, a)
          pixel_index += 1

        elsif (tag & 0xC0) == TAG_LUMA
          # -------------------------------------------------------------------
          # QOI_OP_LUMA — 2 bytes
          # Byte 1: 10_GGGGGG  — green delta biased by -32
          # Byte 2: RRRR_BBBB  — (dr - dg) biased by -8, (db - dg) biased by -8
          # -------------------------------------------------------------------
          dg    = (tag & 0x3F) - 32
          byte2 = s.getbyte(pos)
          pos += 1
          dr_dg = ((byte2 >> 4) & 0x0F) - 8
          db_dg = (byte2 & 0x0F) - 8
          dr    = dr_dg + dg
          dbd   = db_dg + dg
          r = (r + dr)  & 0xFF
          g = (g + dg)  & 0xFF
          b = (b + dbd) & 0xFF
          running[pixel_hash(r, g, b, a)] = [r, g, b, a]

          x = pixel_index % width
          y = pixel_index / width
          pc.set_pixel(canvas, x, y, r, g, b, a)
          pixel_index += 1
        end
      end

      canvas
    end
  end
end
