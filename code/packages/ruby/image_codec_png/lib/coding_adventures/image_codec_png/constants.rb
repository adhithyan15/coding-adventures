# frozen_string_literal: true

module CodingAdventures
  module ImageCodecPng
    module Constants
      # Magic 8-byte PNG file signature (RFC 2083 Section 2.1).
      PNG_SIGNATURE = "\x89PNG\r\n\x1a\n".b.freeze
      SIGNATURE_LENGTH = 8

      # MIME type for PNG images.
      MIME_TYPE_PNG = "image/png"

      # Chunk layout boundaries (RFC 2083 Section 5.3).
      CHUNK_LENGTH_SIZE = 4
      CHUNK_TYPE_SIZE = 4
      CHUNK_CRC_SIZE = 4
      CHUNK_HEADER_SIZE = 8
      CHUNK_OVERHEAD = 12

      # IHDR chunk payload length (RFC 2083 Section 4.1.1).
      IHDR_LENGTH = 13

      # Architectural limits preventing integer overflow and memory exhaustion DoS.
      PNG_MAX_DIMENSION = 16_384
      PNG_MAX_PIXELS = 32 * 1024 * 1024 # 33,554,432 pixels (128 MiB RGBA buffer)
      DEFAULT_MAX_PIXELS = PNG_MAX_PIXELS

      # Bit masks and arithmetic boundaries.
      BITS_PER_BYTE = 8
      BYTE_MASK = 0xFF
      BYTE_MIDPOINT = 128
      BYTE_MODULUS = 256
      UINT32_MASK = 0xFFFF_FFFF
      ASCII_UPPER_A = 0x41
      ASCII_UPPER_Z = 0x5A
      ASCII_LOWER_A = 0x61
      ASCII_LOWER_Z = 0x7A

      # Chunk type flags encoded in ASCII letter case (RFC 2083 Section 5.4).
      CHUNK_ANCILLARY_BIT = 0x20
      CHUNK_RESERVED_BIT = 0x20

      # PNG colour types (RFC 2083 Section 4.1.1).
      COLOUR_TYPE_GREY = 0
      COLOUR_TYPE_RGB = 2
      COLOUR_TYPE_INDEXED = 3
      COLOUR_TYPE_GREY_ALPHA = 4
      COLOUR_TYPE_RGBA = 6
      SUPPORTED_COLOUR_TYPES = [COLOUR_TYPE_GREY, COLOUR_TYPE_RGB, COLOUR_TYPE_GREY_ALPHA, COLOUR_TYPE_RGBA].freeze

      # Channel counts by colour type.
      CHANNELS_GREY = 1
      CHANNELS_RGB = 3
      CHANNELS_GREY_ALPHA = 2
      CHANNELS_RGBA = 4
      BYTES_PER_RGBA_PIXEL = 4
      CHANNELS_BY_COLOUR_TYPE = {
        COLOUR_TYPE_GREY => CHANNELS_GREY,
        COLOUR_TYPE_RGB => CHANNELS_RGB,
        COLOUR_TYPE_GREY_ALPHA => CHANNELS_GREY_ALPHA,
        COLOUR_TYPE_RGBA => CHANNELS_RGBA
      }.freeze

      # Supported profile constraints.
      SUPPORTED_BIT_DEPTH = 8
      COMPRESSION_METHOD_DEFLATE = 0
      FILTER_METHOD_ADAPTIVE = 0
      INTERLACE_METHOD_NONE = 0
      INTERLACE_METHOD_ADAM7 = 1

      # Scanline filter types (RFC 2083 Section 6).
      FILTER_NONE = 0
      FILTER_SUB = 1
      FILTER_UP = 2
      FILTER_AVERAGE = 3
      FILTER_PAETH = 4
      MAX_FILTER_TYPE = 4
      FILTER_COUNT = 5

      # Alpha channel constants.
      OPAQUE_ALPHA = 255
      TRANSPARENT_ALPHA = 0

      # Chunk type names as frozen binary strings.
      CHUNK_IHDR = "IHDR".b.freeze
      CHUNK_PLTE = "PLTE".b.freeze
      CHUNK_IDAT = "IDAT".b.freeze
      CHUNK_IEND = "IEND".b.freeze
      CHUNK_TRNS = "tRNS".b.freeze

      # APNG semantic chunks rejected by profile.
      CHUNK_ACTL = "acTL".b.freeze
      CHUNK_FCTL = "fcTL".b.freeze
      CHUNK_FDAT = "fdAT".b.freeze
      APNG_CHUNKS = [CHUNK_ACTL, CHUNK_FCTL, CHUNK_FDAT].freeze

      # Chunk payload size constraints.
      PLTE_MIN_LENGTH = 3
      PLTE_MAX_LENGTH = 768
      PLTE_ENTRY_SIZE = 3
      TRNS_GREY_LENGTH = 2
      TRNS_RGB_LENGTH = 6
      MAX_SAMPLE_VALUE = 255

      # RFC 1950 zlib wrapper constants.
      ZLIB_HEADER_SIZE = 2
      ZLIB_TRAILER_SIZE = 4
      ZLIB_OVERHEAD = ZLIB_HEADER_SIZE + ZLIB_TRAILER_SIZE
      ZLIB_MIN_STREAM_SIZE = 6
      ZLIB_CMF_DEFLATE = 0x78
      ZLIB_FLG_DEFAULT = 0x9C
      ZLIB_METHOD_DEFLATE = 8
      ZLIB_CINFO_MAX = 7
      ZLIB_FCHECK_MODULUS = 31
      ZLIB_FDICT_MASK = 0x20
      ZLIB_CMF_METHOD_MASK = 0x0F
      ZLIB_CMF_CINFO_SHIFT = 4

      # RFC 1950 Adler-32 constants.
      ADLER_BASE = 65_521
      ADLER_CHUNK_SIZE = 5552
      ADLER_INITIAL_A = 1
      ADLER_INITIAL_B = 0
      ADLER_B_SHIFT = 16

      # Binary packing format strings.
      IHDR_PACK_FORMAT = "NNCCCCC"
      CHUNK_LENGTH_PACK_FORMAT = "N"
      CRC_PACK_FORMAT = "N"
      ADLER32_PACK_FORMAT = "N"
      TRNS_GREY_PACK_FORMAT = "n"
      TRNS_RGB_PACK_FORMAT = "nnn"
      ZLIB_HEADER_PACK_FORMAT = "CC"
      UINT8_PACK_FORMAT = "C"
      BYTE_ARRAY_PACK_FORMAT = "C*"

      # Error identifiers matching fixture taxonomy.
      ERR_INVALID_MAX_PIXELS = "invalid-max-pixels"
      ERR_INVALID_IMAGE_DIMENSIONS = "invalid-image-dimensions"
      ERR_INVALID_PIXEL_DATA_LENGTH = "invalid-pixel-data-length"
      ERR_FILE_TOO_SHORT = "file-too-short"
      ERR_INVALID_SIGNATURE = "invalid-signature"
      ERR_TRUNCATED_CHUNK = "truncated-chunk"
      ERR_INVALID_CHUNK_TYPE = "invalid-chunk-type"
      ERR_CHUNK_CRC_MISMATCH = "chunk-crc-mismatch"
      ERR_CHUNK_BEFORE_IHDR = "chunk-before-ihdr"
      ERR_DUPLICATE_IHDR = "duplicate-ihdr"
      ERR_INVALID_IHDR_LENGTH = "invalid-ihdr-length"
      ERR_INVALID_DIMENSIONS = "invalid-dimensions"
      ERR_DIMENSION_LIMIT = "dimension-limit"
      ERR_PIXEL_LIMIT = "pixel-limit"
      ERR_UNSUPPORTED_FEATURE = "unsupported-feature"
      ERR_INVALID_PLTE = "invalid-plte"
      ERR_INVALID_TRNS = "invalid-trns"
      ERR_NONCONSECUTIVE_IDAT = "nonconsecutive-idat"
      ERR_INVALID_IEND = "invalid-iend"
      ERR_TRAILING_DATA = "trailing-data"
      ERR_UNKNOWN_CRITICAL_CHUNK = "unknown-critical-chunk"
      ERR_MISSING_REQUIRED_CHUNK = "missing-required-chunk"
      ERR_INVALID_ZLIB_HEADER = "invalid-zlib-header"
      ERR_PRESET_DICTIONARY = "preset-dictionary"
      ERR_INFLATE_FAILED = "inflate-failed"
      ERR_INFLATED_LENGTH_MISMATCH = "inflated-length-mismatch"
      ERR_IDAT_CAVITY = "idat-cavity"
      ERR_ADLER_MISMATCH = "adler-mismatch"
      ERR_INVALID_FILTER = "invalid-filter"
      ERROR_UNKNOWN = "unknown-error"

      # External raw inflate error codes.
      ZIP_ERR_OUTPUT_LIMIT_EXCEEDED = "output-limit-exceeded"

      # Closed set of 29 portable error identifiers in exact fixture specification order.
      PNG_ERROR_CODES = [
        ERR_INVALID_MAX_PIXELS,
        ERR_INVALID_IMAGE_DIMENSIONS,
        ERR_INVALID_PIXEL_DATA_LENGTH,
        ERR_FILE_TOO_SHORT,
        ERR_INVALID_SIGNATURE,
        ERR_TRUNCATED_CHUNK,
        ERR_INVALID_CHUNK_TYPE,
        ERR_CHUNK_CRC_MISMATCH,
        ERR_CHUNK_BEFORE_IHDR,
        ERR_DUPLICATE_IHDR,
        ERR_INVALID_IHDR_LENGTH,
        ERR_INVALID_DIMENSIONS,
        ERR_DIMENSION_LIMIT,
        ERR_PIXEL_LIMIT,
        ERR_UNSUPPORTED_FEATURE,
        ERR_INVALID_PLTE,
        ERR_INVALID_TRNS,
        ERR_NONCONSECUTIVE_IDAT,
        ERR_INVALID_IEND,
        ERR_TRAILING_DATA,
        ERR_UNKNOWN_CRITICAL_CHUNK,
        ERR_MISSING_REQUIRED_CHUNK,
        ERR_INVALID_ZLIB_HEADER,
        ERR_PRESET_DICTIONARY,
        ERR_INFLATE_FAILED,
        ERR_INFLATED_LENGTH_MISMATCH,
        ERR_IDAT_CAVITY,
        ERR_ADLER_MISMATCH,
        ERR_INVALID_FILTER
      ].freeze
    end
  end
end
