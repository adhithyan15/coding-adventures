# frozen_string_literal: true

require "coding_adventures/pixel_container"

# =============================================================================
# CodingAdventures::ImageCodecBmp — BMP (Bitmap) image encoder/decoder.
# =============================================================================
#
# BMP is Microsoft's uncompressed raster format. Every .bmp file begins with
# two headers followed by raw pixel data.
#
# File structure (32-bit RGBA, which is what we produce):
#
#   ┌─────────────────────────────┐ offset 0
#   │  BITMAPFILEHEADER (14 bytes)│
#   ├─────────────────────────────┤ offset 14
#   │  BITMAPINFOHEADER (40 bytes)│
#   ├─────────────────────────────┤ offset 54
#   │  Pixel data (BGRA order)    │
#   └─────────────────────────────┘
#
# BITMAPFILEHEADER layout:
#   bfType      2 bytes  "BM" magic number
#   bfSize      4 bytes  total file size in bytes (uint32 LE)
#   bfReserved1 2 bytes  0 (uint16 LE)
#   bfReserved2 2 bytes  0 (uint16 LE)
#   bfOffBits   4 bytes  byte offset to pixel data = 54 (uint32 LE)
#
# BITMAPINFOHEADER layout (40 bytes total):
#   biSize          4 bytes  size of this header = 40 (uint32 LE)
#   biWidth         4 bytes  image width in pixels (int32 LE, signed)
#   biHeight        4 bytes  image height (int32 LE; negative = top-down)
#   biPlanes        2 bytes  1 (uint16 LE)
#   biBitCount      2 bytes  32 (uint16 LE, bits per pixel)
#   biCompression   4 bytes  0 = BI_RGB, uncompressed (uint32 LE)
#   biSizeImage     4 bytes  pixel data size = width*height*4 (uint32 LE)
#   biXPelsPerMeter 4 bytes  0 (int32 LE, pixels per metre)
#   biYPelsPerMeter 4 bytes  0 (int32 LE)
#   biClrUsed       4 bytes  0 (uint32 LE, no palette)
#   biClrImportant  4 bytes  0 (uint32 LE)
#
# Pixel data order:
#   - Each pixel is 4 bytes in BGRA order (Blue, Green, Red, Alpha).
#   - When biHeight > 0 (bottom-up): row 0 of data = BOTTOM row of image.
#   - When biHeight < 0 (top-down):  row 0 of data = TOP row of image.
#   - We always write top-down (negative biHeight) to simplify logic.
#   - On read we handle both orientations.
#
# Why BGRA and not RGBA?
#   The BMP specification stores colour channels as Blue-Green-Red (legacy from
#   the era when BGR matched the VGA hardware). Alpha was added later for 32-bit
#   images. We swap channels when encoding/decoding.
# =============================================================================

module CodingAdventures
  module ImageCodecBmp
    MAX_DIMENSION = 16384

    # -------------------------------------------------------------------------
    # BmpCodec — class that satisfies the ImageCodec mixin interface.
    # -------------------------------------------------------------------------
    class BmpCodec
      include CodingAdventures::PixelContainer::ImageCodec

      # MIME type registered by IANA for BMP files.
      def mime_type
        "image/bmp"
      end

      # Encode a Container to a BMP binary string.
      # @param container [CodingAdventures::PixelContainer::Container]
      # @return [String] binary BMP file bytes
      def encode(container)
        ImageCodecBmp.encode_bmp(container)
      end

      # Decode a BMP binary string into a Container.
      # @param data [String] binary BMP file bytes
      # @return [CodingAdventures::PixelContainer::Container]
      def decode(data)
        ImageCodecBmp.decode_bmp(data)
      end
    end

    # =========================================================================
    # encode_bmp(container) → String (binary)
    #
    # Produces a 32-bit top-down BMP file from an RGBA8 Container.
    #
    # Pack format strings used:
    #   "V"    unsigned 32-bit LE integer (uint32)
    #   "v"    unsigned 16-bit LE integer (uint16)
    #   "i<"   signed   32-bit LE integer (int32)
    #   "C"    unsigned byte
    #   "a2"   2-byte ASCII string (no null padding)
    # =========================================================================
    def self.encode_bmp(container)
      width  = container.width
      height = container.height
      pixel_data_size = width * height * 4
      file_size       = 54 + pixel_data_size

      # -----------------------------------------------------------------------
      # BITMAPFILEHEADER (14 bytes)
      #
      # "BM" is the magic that identifies BMP files. bfOffBits = 54 because the
      # two headers together are exactly 14 + 40 = 54 bytes.
      # -----------------------------------------------------------------------
      file_header = "BM".b +
        [file_size].pack("V") +   # bfSize: total file size
        [0].pack("v") +           # bfReserved1: must be 0
        [0].pack("v") +           # bfReserved2: must be 0
        [54].pack("V")            # bfOffBits: pixel data starts at byte 54

      # -----------------------------------------------------------------------
      # BITMAPINFOHEADER (40 bytes)
      #
      # biHeight is stored as -height (negative) to signal a top-down image.
      # This means row 0 in the pixel data is the TOP of the image, matching
      # our Container's (0,0) = top-left convention.
      #
      # biBitCount = 32 → 4 bytes per pixel (BGRA).
      # biCompression = 0 (BI_RGB) → uncompressed raw pixels.
      # -----------------------------------------------------------------------
      info_header =
        [40].pack("V")       +    # biSize: 40 bytes
        [width].pack("i<")   +    # biWidth: pixels per row (signed int32)
        [-height].pack("i<") +    # biHeight: negative = top-down
        [1].pack("v")        +    # biPlanes: always 1
        [32].pack("v")       +    # biBitCount: 32 bits per pixel
        [0].pack("V")        +    # biCompression: 0 = BI_RGB
        [pixel_data_size].pack("V") + # biSizeImage
        [0].pack("i<")       +    # biXPelsPerMeter
        [0].pack("i<")       +    # biYPelsPerMeter
        [0].pack("V")        +    # biClrUsed
        [0].pack("V")             # biClrImportant

      # -----------------------------------------------------------------------
      # Pixel data
      #
      # We write pixels top-to-bottom, left-to-right.
      # Each pixel is stored as 4 bytes in BGRA order (note: B before R).
      # -----------------------------------------------------------------------
      pixels = "".b
      pc_mod = CodingAdventures::PixelContainer
      height.times do |y|
        width.times do |x|
          r, g, b, a = pc_mod.pixel_at(container, x, y)
          pixels << [b, g, r, a].pack("CCCC")
        end
      end

      file_header + info_header + pixels
    end

    # =========================================================================
    # decode_bmp(data) → Container
    #
    # Parses a 32-bit BMP file and returns an RGBA8 Container.
    #
    # Validates:
    #   - "BM" magic at offset 0
    #   - biBitCount == 32 (we only support 32-bit RGBA BMPs)
    #   - biCompression == 0 (BI_RGB, uncompressed)
    #
    # Handles both bottom-up (biHeight > 0) and top-down (biHeight < 0).
    # =========================================================================
    def self.decode_bmp(data)
      raise ArgumentError, "BMP data too short" if data.bytesize < 54

      # -----------------------------------------------------------------------
      # Validate magic bytes: the first two bytes must be ASCII "BM".
      # -----------------------------------------------------------------------
      magic = data.byteslice(0, 2)
      raise ArgumentError, "Not a BMP file (bad magic: #{magic.inspect})" unless magic == "BM"

      # -----------------------------------------------------------------------
      # Parse BITMAPFILEHEADER
      # -----------------------------------------------------------------------
      # bfOffBits: byte offset where the pixel data starts
      pix_offset = data.byteslice(10, 4).unpack1("V")

      # -----------------------------------------------------------------------
      # Parse BITMAPINFOHEADER
      # Offset 14: start of info header.
      # -----------------------------------------------------------------------
      width    = data.byteslice(18, 4).unpack1("i<")  # signed int32
      bi_height = data.byteslice(22, 4).unpack1("i<") # signed int32 (may be negative)
      bit_count   = data.byteslice(28, 2).unpack1("v") # uint16
      compression = data.byteslice(30, 4).unpack1("V") # uint32

      raise ArgumentError, "Unsupported BMP bit depth: #{bit_count} (expected 32)" unless bit_count == 32
      raise ArgumentError, "Unsupported BMP compression: #{compression} (expected 0)" unless compression == 0

      raise ArgumentError, "BMP: invalid dimensions" if width.zero? || bi_height.zero?
      raise ArgumentError, "BMP: dimensions too large" if width > MAX_DIMENSION || bi_height.abs > MAX_DIMENSION

      # -----------------------------------------------------------------------
      # Determine scan-line direction.
      #
      # biHeight > 0 → bottom-up: the pixel data stores the BOTTOM row first.
      #   We must flip the row order when loading into our Container.
      # biHeight < 0 → top-down: pixel data stores the TOP row first.
      #   No flip needed; abs(biHeight) is the actual pixel height.
      # -----------------------------------------------------------------------
      top_down = bi_height.negative?
      height   = bi_height.abs

      pc     = CodingAdventures::PixelContainer
      canvas = pc.create(width, height)

      height.times do |row_index|
        # Map the file row to the image row.
        # Top-down: file row 0 = image row 0.
        # Bottom-up: file row 0 = image row (height-1).
        image_y = top_down ? row_index : (height - 1 - row_index)

        row_byte_offset = pix_offset + row_index * width * 4
        width.times do |x|
          byte_pos = row_byte_offset + x * 4
          # BMP stores BGRA; we convert to RGBA for the Container.
          b = data.getbyte(byte_pos)
          g = data.getbyte(byte_pos + 1)
          r = data.getbyte(byte_pos + 2)
          a = data.getbyte(byte_pos + 3)
          pc.set_pixel(canvas, x, image_y, r, g, b, a)
        end
      end

      canvas
    end
  end
end
