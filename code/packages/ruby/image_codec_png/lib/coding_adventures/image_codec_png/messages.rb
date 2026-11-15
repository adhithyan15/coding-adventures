# frozen_string_literal: true

require_relative "constants"

module CodingAdventures
  module ImageCodecPng
    # Standing Rule #20: Payload-blind diagnostic strings and redaction placeholders.
    # All error messages and diagnostics are centralized here to guarantee zero raw
    # input leakage and strict separation of presentation from logic.
    module Messages
      REDACTED_PLACEHOLDER = "<redacted>"
      INVALID_PLACEHOLDER = "<invalid>"
      UNKNOWN_ERROR_CODE = Constants::ERROR_UNKNOWN
      UNKNOWN_ERROR_MESSAGE = "image-codec-png: unknown error code"

      ERROR_MESSAGES = {
        Constants::ERR_INVALID_MAX_PIXELS => "image-codec-png: max_pixels must be a positive integer <= 33554432",
        Constants::ERR_INVALID_IMAGE_DIMENSIONS => "image-codec-png: image dimensions must be positive integers within limit",
        Constants::ERR_INVALID_PIXEL_DATA_LENGTH => "image-codec-png: pixel buffer length does not match width * height * 4",
        Constants::ERR_FILE_TOO_SHORT => "image-codec-png: input too short to contain PNG signature",
        Constants::ERR_INVALID_SIGNATURE => "image-codec-png: invalid PNG signature",
        Constants::ERR_TRUNCATED_CHUNK => "image-codec-png: chunk header or payload truncated",
        Constants::ERR_INVALID_CHUNK_TYPE => "image-codec-png: invalid chunk type name",
        Constants::ERR_CHUNK_CRC_MISMATCH => "image-codec-png: chunk CRC mismatch",
        Constants::ERR_CHUNK_BEFORE_IHDR => "image-codec-png: chunk found before IHDR",
        Constants::ERR_DUPLICATE_IHDR => "image-codec-png: duplicate IHDR chunk",
        Constants::ERR_INVALID_IHDR_LENGTH => "image-codec-png: IHDR length must be exactly 13 bytes",
        Constants::ERR_INVALID_DIMENSIONS => "image-codec-png: image dimensions must be non-zero",
        Constants::ERR_DIMENSION_LIMIT => "image-codec-png: image dimensions exceed limit",
        Constants::ERR_PIXEL_LIMIT => "image-codec-png: image pixel count exceeds limit",
        Constants::ERR_UNSUPPORTED_FEATURE => "image-codec-png: unsupported PNG feature or APNG chunk",
        Constants::ERR_INVALID_PLTE => "image-codec-png: invalid, duplicate, or misplaced PLTE chunk",
        Constants::ERR_INVALID_TRNS => "image-codec-png: invalid, duplicate, or misplaced tRNS chunk",
        Constants::ERR_NONCONSECUTIVE_IDAT => "image-codec-png: non-consecutive IDAT chunks",
        Constants::ERR_INVALID_IEND => "image-codec-png: IEND chunk must be empty",
        Constants::ERR_TRAILING_DATA => "image-codec-png: trailing data found after IEND chunk",
        Constants::ERR_UNKNOWN_CRITICAL_CHUNK => "image-codec-png: unknown critical chunk",
        Constants::ERR_MISSING_REQUIRED_CHUNK => "image-codec-png: missing required chunk (IHDR, IDAT, or IEND)",
        Constants::ERR_INVALID_ZLIB_HEADER => "image-codec-png: invalid zlib header in IDAT",
        Constants::ERR_PRESET_DICTIONARY => "image-codec-png: preset dictionary is unsupported",
        Constants::ERR_INFLATE_FAILED => "image-codec-png: DEFLATE decompression failed",
        Constants::ERR_INFLATED_LENGTH_MISMATCH => "image-codec-png: decompressed length does not match expected scanline size",
        Constants::ERR_IDAT_CAVITY => "image-codec-png: unconsumed bytes between compressed data and Adler-32",
        Constants::ERR_ADLER_MISMATCH => "image-codec-png: Adler-32 checksum mismatch",
        Constants::ERR_INVALID_FILTER => "image-codec-png: invalid scanline filter type"
      }.freeze

      def self.message_for(code)
        ERROR_MESSAGES.fetch(code, UNKNOWN_ERROR_MESSAGE)
      end
    end
  end
end
