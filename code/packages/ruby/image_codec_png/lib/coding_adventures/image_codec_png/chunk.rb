# frozen_string_literal: true

require "coding_adventures_zip"
require_relative "constants"

module CodingAdventures
  module ImageCodecPng
    # PNG chunk framing and CRC-32 operations reusing CodingAdventures::Zip.
    module Chunk
      # Format a complete PNG chunk: length (BE u32), type (4 bytes), payload, CRC-32 (BE u32).
      def self.format(type, *payload_parts)
        payload = payload_parts.join.b
        length = payload.bytesize

        crc = CodingAdventures::Zip.crc32(type)
        crc = CodingAdventures::Zip.crc32(payload, initial: crc) unless payload.empty?

        [length].pack(Constants::CHUNK_LENGTH_PACK_FORMAT).b +
          type.b +
          payload +
          [crc].pack(Constants::CRC_PACK_FORMAT).b
      end

      # Validate whether a 4-byte chunk type adheres to RFC 2083 Section 5.4.
      # All 4 bytes must be ASCII letters, and the 3rd byte (reserved bit) must be uppercase.
      def self.valid_type?(type_bytes)
        return false unless type_bytes.bytesize == Constants::CHUNK_TYPE_SIZE

        b0 = type_bytes.getbyte(0)
        b1 = type_bytes.getbyte(1)
        b2 = type_bytes.getbyte(2)
        b3 = type_bytes.getbyte(3)

        return false if (b2 & Constants::CHUNK_RESERVED_BIT) != 0

        [b0, b1, b2, b3].all? do |b|
          b.between?(Constants::ASCII_UPPER_A, Constants::ASCII_UPPER_Z) ||
            b.between?(Constants::ASCII_LOWER_A, Constants::ASCII_LOWER_Z)
        end
      end

      # Checks whether a chunk type is critical (uppercase first letter).
      def self.critical?(type_bytes)
        (type_bytes.getbyte(0) & Constants::CHUNK_ANCILLARY_BIT) == 0
      end
    end
  end
end
