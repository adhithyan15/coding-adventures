# frozen_string_literal: true

# leb128.rb — LEB128 variable-length integer encoding for WebAssembly
#
# ─────────────────────────────────────────────────────────────────────────────
# WHAT IS LEB128?
# ─────────────────────────────────────────────────────────────────────────────
#
# LEB128 stands for "Little-Endian Base 128." It is a variable-length encoding
# for integers. Instead of always using 4 or 8 bytes, small numbers take fewer
# bytes. WebAssembly uses LEB128 for every integer in its binary (.wasm) format:
# function indices, memory sizes, instruction immediates, section lengths, etc.
#
# Each encoded byte has two parts:
#   - Bits 0–6 (the lower 7): the payload (actual data)
#   - Bit 7 (the high bit, 0x80): the continuation flag
#
# Continuation flag:
#   bit 7 = 1  →  "more bytes follow"
#   bit 7 = 0  →  "this is the last byte"
#
# ─────────────────────────────────────────────────────────────────────────────
# WORKED EXAMPLE: Encoding 624485
# ─────────────────────────────────────────────────────────────────────────────
#
#   624485 in binary (20 bits):
#     0b0001_0011_0000_0111_0110_0101
#
#   Step 1 — split into 7-bit groups, least-significant first:
#     Group 0 (bits 0–6):   110_0101  = 0x65 = 101
#     Group 1 (bits 7–13):  000_1110  = 0x0E = 14
#     Group 2 (bits 14–19): 010_0110  = 0x26 = 38
#
#   Step 2 — set continuation bit (0x80) on every group except the last:
#     Group 0: 0x65 | 0x80 = 0xE5   (more bytes follow)
#     Group 1: 0x0E | 0x80 = 0x8E   (more bytes follow)
#     Group 2: 0x26         = 0x26   (last byte)
#
#   Encoded: [0xE5, 0x8E, 0x26]  ← 3 bytes instead of 4
#
# ─────────────────────────────────────────────────────────────────────────────
# SIGNED vs UNSIGNED
# ─────────────────────────────────────────────────────────────────────────────
#
# Unsigned LEB128 (ULEB128): treats numbers as non-negative.
# Signed LEB128 (SLEB128): uses two's complement sign extension.
#
# After decoding all 7-bit groups, if bit 6 of the last byte is 1 (for values
# that haven't consumed all 32 bits), the value is negative. We sign-extend it
# by setting all bits from the current shift position upward to 1.
#
# Example — decode [0x7E] as signed:
#   0x7E = 0b0111_1110
#   continuation bit = 0 → last byte
#   payload = 0x7E & 0x7F = 126 = 0b111_1110
#   bit 6 = 1 → sign extend: 126 - 128 = -2  ✓
#
# ─────────────────────────────────────────────────────────────────────────────
# RUBY BIGNUM NOTE
# ─────────────────────────────────────────────────────────────────────────────
#
# Unlike JavaScript, Ruby integers are arbitrary-precision (Bignum). This means
# we must explicitly clamp results to 32-bit ranges when needed, and use
# explicit mask constants rather than relying on overflow wrapping.
#
# For unsigned: results are in [0, 2^32 - 1]
# For signed:   results are in [-2^31, 2^31 - 1]
#
# ─────────────────────────────────────────────────────────────────────────────

module CodingAdventures
  module WasmLeb128
    # LEB128Error is raised when decoding encounters invalid input.
    #
    # Common causes:
    #   - Unterminated sequence (all bytes have continuation bit = 1)
    #   - Value exceeds the 32-bit range
    class LEB128Error < StandardError; end

    # ─────────────────────────────────────────────────────────────────────────
    # Constants
    # ─────────────────────────────────────────────────────────────────────────

    # The continuation flag occupies bit 7 of each encoded byte.
    # 0x80 = 0b1000_0000
    CONTINUATION_BIT = 0x80

    # The payload mask extracts the lower 7 bits.
    # 0x7F = 0b0111_1111
    PAYLOAD_MASK = 0x7F

    # Maximum bytes for a 32-bit LEB128 value.
    # ceil(32 / 7) = 5.  Five 7-bit groups cover 35 bits, enough for 32.
    MAX_LEB128_BYTES_32 = 5

    # 32-bit masks for sign extension and unsigned clamping
    MASK_32  = 0xFFFFFFFF   # used to truncate to u32
    SIGN_32  = 0x80000000   # the sign bit of a 32-bit integer
    MAX_U32  = 0xFFFFFFFF
    MIN_I32  = -2147483648
    MAX_I32  =  2147483647

    # ─────────────────────────────────────────────────────────────────────────
    # decode_unsigned
    # ─────────────────────────────────────────────────────────────────────────

    # Decode a ULEB128-encoded unsigned integer from a byte string.
    #
    # Algorithm:
    #   result = 0
    #   shift  = 0
    #   for each byte b starting at `offset`:
    #     payload = b & 0x7F           # lower 7 bits
    #     result |= payload << shift   # place at correct bit position
    #     shift += 7
    #     break unless b & 0x80 != 0   # stop if no continuation bit
    #
    # @param data   [String] binary-encoded string (use String#b or pack/unpack)
    #               OR an Array of integers in 0..255
    # @param offset [Integer] byte position to start reading (default 0)
    # @return       [Array(Integer, Integer)] [value, bytes_consumed]
    # @raise        LEB128Error if the sequence is unterminated or too long
    #
    # Example:
    #   decode_unsigned("\xE5\x8E\x26".b)   # => [624485, 3]
    #   decode_unsigned([0xE5, 0x8E, 0x26]) # => [624485, 3]
    def self.decode_unsigned(data, offset = 0)
      result = 0
      shift = 0
      bytes_consumed = 0

      # Normalize: accept both Array of ints and binary String
      bytes = data.is_a?(Array) ? data : data.bytes

      i = offset
      loop do
        # Guard: 32-bit LEB128 must not exceed 5 bytes
        if bytes_consumed >= MAX_LEB128_BYTES_32
          raise LEB128Error, "LEB128 sequence exceeds maximum #{MAX_LEB128_BYTES_32} bytes for a 32-bit value"
        end

        # Guard: don't read past end of array
        if i >= bytes.length
          raise LEB128Error,
            "LEB128 sequence is unterminated: reached end of data at offset #{i} " \
            "without finding a byte with continuation bit = 0"
        end

        byte = bytes[i]
        payload = byte & PAYLOAD_MASK  # extract lower 7 bits

        # Place the 7 payload bits at their correct position in result.
        # After byte 0: bits 0–6. After byte 1: bits 7–13. Etc.
        result |= (payload << shift)
        shift += 7
        bytes_consumed += 1
        i += 1

        # If the continuation bit is NOT set, this is the last byte.
        if (byte & CONTINUATION_BIT) == 0
          # Truncate to 32 bits for unsigned correctness
          return [result & MASK_32, bytes_consumed]
        end
      end
    end

    # ─────────────────────────────────────────────────────────────────────────
    # decode_signed
    # ─────────────────────────────────────────────────────────────────────────

    # Decode a SLEB128-encoded signed integer (two's complement).
    #
    # Identical to decode_unsigned, but after the loop we check for sign
    # extension. If bit 6 of the last byte is 1 AND we haven't consumed all
    # 32 bits, the value is negative and needs to be sign-extended.
    #
    # Sign extension example for [0x7E] → -2:
    #   payload = 0x7E & 0x7F = 126 = 0b111_1110
    #   result = 126, shift = 7
    #   bit 6 of 0x7E = 1 → sign extend
    #   result = 126 - (1 << 7) = 126 - 128 = -2  ✓
    #
    # In code: result |= -(1 << shift)
    # The negative literal fills all bits from `shift` upward with 1s.
    #
    # @param data   [String, Array] binary data
    # @param offset [Integer] starting byte position (default 0)
    # @return       [Array(Integer, Integer)] [signed_value, bytes_consumed]
    # @raise        LEB128Error if unterminated or too long
    def self.decode_signed(data, offset = 0)
      result = 0
      shift = 0
      bytes_consumed = 0
      last_byte = 0

      bytes = data.is_a?(Array) ? data : data.bytes

      i = offset
      loop do
        if bytes_consumed >= MAX_LEB128_BYTES_32
          raise LEB128Error, "LEB128 sequence exceeds maximum #{MAX_LEB128_BYTES_32} bytes for a 32-bit value"
        end

        if i >= bytes.length
          raise LEB128Error,
            "LEB128 sequence is unterminated: reached end of data at offset #{i} " \
            "without finding a byte with continuation bit = 0"
        end

        byte = bytes[i]
        last_byte = byte
        payload = byte & PAYLOAD_MASK

        result |= (payload << shift)
        shift += 7
        bytes_consumed += 1
        i += 1

        if (byte & CONTINUATION_BIT) == 0
          # Sign extension:
          # - `shift < 32`: we haven't consumed all 32 bits yet
          # - `last_byte & 0x40 != 0`: bit 6 (MSB of payload) is 1 → negative
          #
          #   0x40 = 0b0100_0000  ← bit 6 of the byte (MSB of 7-bit payload)
          #
          # `-(1 << shift)` in Ruby creates a Bignum with all bits from
          # position `shift` upward set to 1, effectively sign-extending.
          if shift < 32 && (last_byte & 0x40) != 0
            result |= -(1 << shift)
          end

          # Clamp to signed 32-bit range:
          # Ruby integers don't overflow, so we use modular arithmetic.
          # Trick: interpret as signed 32-bit by checking the sign bit.
          result = result & MASK_32  # truncate to 32 bits
          result -= (1 << 32) if result >= SIGN_32  # convert to signed

          return [result, bytes_consumed]
        end
      end
    end

    # ─────────────────────────────────────────────────────────────────────────
    # encode_unsigned
    # ─────────────────────────────────────────────────────────────────────────

    # Encode a non-negative integer as ULEB128.
    #
    # Algorithm:
    #   do:
    #     byte = value & 0x7F       # take lowest 7 bits
    #     value >>= 7               # shift off those 7 bits
    #     byte |= 0x80 if value > 0 # set continuation bit if more to come
    #     emit byte
    #   while value > 0
    #
    # The do/begin-end-while ensures at least one byte is emitted (for value=0).
    #
    # @param value [Integer] non-negative integer in [0, 2^32 - 1]
    # @return      [String] binary string of ULEB128-encoded bytes
    #
    # Example:
    #   encode_unsigned(624485)   # => "\xE5\x8E\x26"
    def self.encode_unsigned(value)
      # Treat as unsigned 32-bit (mask off any higher bits)
      remaining = value & MASK_32
      bytes = []

      loop do
        # Extract the lowest 7 bits as this byte's payload
        byte = remaining & PAYLOAD_MASK

        # Shift off the 7 bits we just consumed (unsigned, so always positive)
        remaining >>= 7

        # Set the continuation bit if there are more bytes to follow
        byte |= CONTINUATION_BIT if remaining != 0

        bytes << byte

        break if remaining == 0
      end

      # Pack as a binary string (C* = unsigned chars)
      bytes.pack("C*")
    end

    # ─────────────────────────────────────────────────────────────────────────
    # encode_signed
    # ─────────────────────────────────────────────────────────────────────────

    # Encode a signed integer as SLEB128.
    #
    # Similar to encode_unsigned, but the termination condition handles both
    # positive and negative values:
    #
    #   Positive: stop when remaining = 0 AND bit 6 of last byte = 0
    #             (decoder will NOT sign-extend → correct for positive)
    #   Negative: stop when remaining = -1 AND bit 6 of last byte = 1
    #             (decoder WILL sign-extend → correct for negative)
    #
    # Combined condition:
    #   done = (remaining == 0 && byte & 0x40 == 0)
    #       || (remaining == -1 && byte & 0x40 != 0)
    #
    # Example: encode -2
    #   -2 in Ruby: integer, no overflow
    #   byte 0: -2 & 0x7F = 0x7E = 126 = 0b111_1110
    #           -2 >> 7 = -1 (arithmetic shift in Ruby preserves sign)
    #           remaining = -1, byte & 0x40 = 0x40 ≠ 0 → done
    #   Emit: [0x7E]  ✓
    #
    # @param value [Integer] signed integer in [-2^31, 2^31 - 1]
    # @return      [String] binary string of SLEB128-encoded bytes
    def self.encode_signed(value)
      # Coerce to 32-bit signed range via modular arithmetic
      remaining = value | 0  # Ruby's integers don't overflow; coerce via sign
      # Actually, Ruby's >> is arithmetic (sign-preserving), so we just use value directly
      remaining = value
      bytes = []
      done = false

      until done
        # Extract lowest 7 bits
        byte = remaining & PAYLOAD_MASK

        # Arithmetic right-shift (Ruby's >> preserves sign for negative numbers)
        # This is unlike JavaScript's >>, which is also arithmetic, but Ruby
        # integers are arbitrary precision, so -1 >> 7 = -1 (infinite sign extension).
        remaining >>= 7

        # Termination check:
        #   Positive numbers: remaining has dropped to 0 and the MSB of this
        #     7-bit group is 0, so the decoder won't sign-extend.
        #   Negative numbers: remaining has dropped to -1 and the MSB of this
        #     7-bit group is 1, so the decoder will sign-extend correctly.
        if (remaining == 0 && (byte & 0x40) == 0) ||
           (remaining == -1 && (byte & 0x40) != 0)
          done = true
        else
          byte |= CONTINUATION_BIT
        end

        bytes << byte
      end

      bytes.pack("C*")
    end
  end
end
