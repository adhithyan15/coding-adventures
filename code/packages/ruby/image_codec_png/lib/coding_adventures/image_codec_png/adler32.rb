# frozen_string_literal: true

require_relative "constants"

module CodingAdventures
  module ImageCodecPng
    # RFC 1950 Adler-32 checksum algorithm.
    module Adler32
      # Compute the 32-bit Adler-32 checksum of data.
      #
      # @param data [String, Array<Integer>] binary data or array of byte values
      # @return [Integer] 32-bit unsigned Adler-32 checksum
      def self.compute(data)
        bytes = data.is_a?(String) ? data.b : data.pack(Constants::BYTE_ARRAY_PACK_FORMAT).b
        len = bytes.bytesize
        a = Constants::ADLER_INITIAL_A
        b = Constants::ADLER_INITIAL_B
        pos = 0

        while pos < len
          chunk_len = [Constants::ADLER_CHUNK_SIZE, len - pos].min
          chunk_end = pos + chunk_len
          while pos < chunk_end
            a += bytes.getbyte(pos)
            b += a
            pos += 1
          end
          a %= Constants::ADLER_BASE
          b %= Constants::ADLER_BASE
        end

        ((b << Constants::ADLER_B_SHIFT) | a) & Constants::UINT32_MASK
      end
    end
  end
end
