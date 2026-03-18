# frozen_string_literal: true

# ---------------------------------------------------------------------------
# IEEE 754 encoding and decoding -- converting between Ruby floats and bits.
# ---------------------------------------------------------------------------
#
# === How does a computer store 3.14? ===
#
# When you write x = 3.14 in Ruby, the computer stores it as 64 bits
# following the IEEE 754 standard (Ruby uses FP64 internally). This module
# converts between Ruby's native float representation and our explicit
# bit-level representation (FloatBits).
#
# === Encoding: float -> bits ===
#
# For FP32, we use Ruby's Array#pack / String#unpack to get the exact
# same bit pattern that the hardware uses. For FP16 and BF16, we manually
# extract the bits because Ruby doesn't natively support these formats.
#
# === Special values in IEEE 754 ===
#
# IEEE 754 reserves certain bit patterns for special values:
#
#     Exponent      Mantissa    Meaning
#     ----------    --------    -------
#     All 1s        All 0s      +/- Infinity
#     All 1s        Non-zero    NaN (Not a Number)
#     All 0s        All 0s      +/- Zero
#     All 0s        Non-zero    Denormalized number (very small, near zero)
#     Other         Any         Normal number

module CodingAdventures
  module FpArithmetic
    # -----------------------------------------------------------------------
    # Helper: integer <-> bit list conversions
    # -----------------------------------------------------------------------

    # Convert a non-negative integer to a list of bits, MSB first.
    #
    # This is the fundamental conversion between Ruby's arbitrary-precision
    # integers and our bit-level representation.
    #
    # Example:
    #   int_to_bits_msb(5, 8)
    #   # => [0, 0, 0, 0, 0, 1, 0, 1]
    #   #     128 64 32 16  8  4  2  1
    #   #                      4     1  = 5
    #
    # How it works:
    #   We check each bit position from the most significant (leftmost) to
    #   the least significant (rightmost). For each position i (counting from
    #   width-1 down to 0), we check if that bit is set using a right-shift
    #   and AND with 1.
    #
    # @param value [Integer] The integer to convert (must be >= 0).
    # @param width [Integer] The number of bits in the output list.
    # @return [Array<Integer>] A list of 0s and 1s, MSB first.
    def self.int_to_bits_msb(value, width)
      Array.new(width) { |i| (value >> (width - 1 - i)) & 1 }
    end

    # Convert a list of bits (MSB first) back to a non-negative integer.
    #
    # This is the inverse of int_to_bits_msb.
    #
    # Example:
    #   bits_msb_to_int([0, 0, 0, 0, 0, 1, 0, 1])
    #   # => 5
    #
    # How it works:
    #   We iterate through the bits from MSB to LSB. For each bit, we shift
    #   the accumulator left by 1 (multiply by 2) and OR in the new bit.
    #
    # @param bits [Array<Integer>] List of 0s and 1s, MSB first.
    # @return [Integer] The integer value represented by the bits.
    def self.bits_msb_to_int(bits)
      result = 0
      bits.each do |bit|
        result = (result << 1) | bit
      end
      result
    end

    # -----------------------------------------------------------------------
    # Encoding: Ruby float -> FloatBits
    # -----------------------------------------------------------------------

    # Convert a Ruby float to its IEEE 754 bit representation.
    #
    # === How FP32 encoding works (using pack/unpack) ===
    #
    # For FP32, we leverage Ruby's pack/unpack which gives us access to the
    # exact bit pattern that the hardware uses:
    #
    #     [3.14].pack("g")  -> "\x40\x48\xF5\xC3"
    #
    # The "g" format means: big-endian single-precision float.
    # We then unpack those 4 bytes as a 32-bit unsigned integer to get the
    # raw bits.
    #
    # === How FP16/BF16 encoding works (manual) ===
    #
    # For FP16 and BF16, Ruby doesn't have native support, so we:
    # 1. First encode as FP32 (which we know is exact for the hardware)
    # 2. Extract the sign, exponent, and mantissa from the FP32 encoding
    # 3. Re-encode into the target format, adjusting exponent bias and
    #    truncating the mantissa
    #
    # @param value [Float] The Ruby float to encode.
    # @param fmt [FloatFormat] The target format (FP32, FP16, or BF16).
    # @return [FloatBits] The bit-level representation.
    def self.float_to_bits(value, fmt = FP32)
      # --- Handle NaN specially ---
      # Ruby has Float::NAN, and IEEE 754 defines NaN as exponent=all-1s,
      # mantissa=non-zero. We use a "quiet NaN" with the MSB of mantissa set.
      if value.nan?
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
          fmt: fmt
        )
      end

      # --- Handle Infinity ---
      # +Inf and -Inf: exponent=all-1s, mantissa=all-0s.
      if value.infinite?
        sign = value > 0 ? 0 : 1
        return FloatBits.new(
          sign: sign,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # --- FP32: use pack/unpack for hardware-exact encoding ---
      if fmt == FP32
        # Pack as big-endian float, unpack as big-endian unsigned 32-bit int.
        packed = [value].pack("g")
        int_bits = packed.unpack1("N")

        # Extract the three fields using bit shifts and masks:
        #   Bit 31:     sign
        #   Bits 30-23: exponent (8 bits)
        #   Bits 22-0:  mantissa (23 bits)
        sign = (int_bits >> 31) & 1
        exp_int = (int_bits >> 23) & 0xFF
        mant_int = int_bits & 0x7FFFFF

        return FloatBits.new(
          sign: sign,
          exponent: int_to_bits_msb(exp_int, 8),
          mantissa: int_to_bits_msb(mant_int, 23),
          fmt: FP32
        )
      end

      # --- FP16 and BF16: manual conversion from FP32 ---
      #
      # Strategy: encode as FP32 first, then convert.
      fp32_bits = float_to_bits(value, FP32)
      fp32_exp = bits_msb_to_int(fp32_bits.exponent)
      fp32_mant = bits_msb_to_int(fp32_bits.mantissa)
      sign = fp32_bits.sign

      # --- Handle zero ---
      if fp32_exp == 0 && fp32_mant == 0
        return FloatBits.new(
          sign: sign,
          exponent: Array.new(fmt.exponent_bits, 0),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # --- Compute the true (unbiased) exponent ---
      if fp32_exp == 0
        # Denormal in FP32: true exponent is -126, implicit bit is 0
        true_exp = 1 - FP32.bias # = -126
        full_mantissa = fp32_mant
      else
        true_exp = fp32_exp - FP32.bias
        # Normal: full mantissa includes the implicit leading 1
        full_mantissa = (1 << FP32.mantissa_bits) | fp32_mant
      end

      # --- Map to target format ---
      target_exp = true_exp + fmt.bias
      max_exp = (1 << fmt.exponent_bits) - 1

      # --- Overflow: exponent too large for target format -> Infinity ---
      if target_exp >= max_exp
        return FloatBits.new(
          sign: sign,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # --- Normal case: exponent fits in target format ---
      if target_exp > 0
        if fmt.mantissa_bits < FP32.mantissa_bits
          shift = FP32.mantissa_bits - fmt.mantissa_bits
          truncated = fp32_mant >> shift
          # Round-to-nearest-even
          round_bit = (fp32_mant >> (shift - 1)) & 1
          sticky = fp32_mant & ((1 << (shift - 1)) - 1)
          if round_bit == 1 && (sticky != 0 || (truncated & 1) == 1)
            truncated += 1
            # Rounding overflow
            if truncated >= (1 << fmt.mantissa_bits)
              truncated = 0
              target_exp += 1
              if target_exp >= max_exp
                return FloatBits.new(
                  sign: sign,
                  exponent: Array.new(fmt.exponent_bits, 1),
                  mantissa: Array.new(fmt.mantissa_bits, 0),
                  fmt: fmt
                )
              end
            end
          end
        else
          truncated = fp32_mant << (fmt.mantissa_bits - FP32.mantissa_bits)
        end

        return FloatBits.new(
          sign: sign,
          exponent: int_to_bits_msb(target_exp, fmt.exponent_bits),
          mantissa: int_to_bits_msb(truncated, fmt.mantissa_bits),
          fmt: fmt
        )
      end

      # --- Underflow: number is too small for normal representation ---
      # It might still be representable as a denormal in the target format.
      denorm_shift = 1 - target_exp

      if denorm_shift > fmt.mantissa_bits
        # Too small even for denormal -> flush to zero
        return FloatBits.new(
          sign: sign,
          exponent: Array.new(fmt.exponent_bits, 0),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # Shift the full mantissa right to create a denormal
      denorm_mant = full_mantissa >> (denorm_shift + FP32.mantissa_bits - fmt.mantissa_bits)

      FloatBits.new(
        sign: sign,
        exponent: Array.new(fmt.exponent_bits, 0),
        mantissa: int_to_bits_msb(denorm_mant & ((1 << fmt.mantissa_bits) - 1), fmt.mantissa_bits),
        fmt: fmt
      )
    end

    # -----------------------------------------------------------------------
    # Decoding: FloatBits -> Ruby float
    # -----------------------------------------------------------------------

    # Convert an IEEE 754 bit representation back to a Ruby float.
    #
    # For FP32, we reconstruct the 32-bit integer and use unpack to get
    # the exact Ruby float. For FP16/BF16, we manually compute the value
    # using the formula:
    #
    #     value = (-1)^sign x 2^(exponent - bias) x 1.mantissa
    #
    # @param bits [FloatBits] The FloatBits to decode.
    # @return [Float] The Ruby float value.
    def self.bits_to_float(bits)
      exp_int = bits_msb_to_int(bits.exponent)
      mant_int = bits_msb_to_int(bits.mantissa)
      max_exp = (1 << bits.fmt.exponent_bits) - 1

      # --- Special values ---

      # NaN: exponent all 1s, mantissa non-zero
      if exp_int == max_exp && mant_int != 0
        return Float::NAN
      end

      # Infinity: exponent all 1s, mantissa all zeros
      if exp_int == max_exp && mant_int == 0
        return bits.sign == 1 ? -Float::INFINITY : Float::INFINITY
      end

      # Zero: exponent all 0s, mantissa all zeros
      if exp_int == 0 && mant_int == 0
        return bits.sign == 1 ? -0.0 : 0.0
      end

      # --- For FP32, use pack/unpack for exact conversion ---
      if bits.fmt == FP32
        int_bits = (bits.sign << 31) | (exp_int << 23) | mant_int
        packed = [int_bits].pack("N")
        return packed.unpack1("g")
      end

      # --- For FP16/BF16, compute the float value manually ---

      if exp_int == 0
        # Denormalized: exponent=0, implicit bit is 0
        true_exp = 1 - bits.fmt.bias
        mantissa_value = mant_int.to_f / (1 << bits.fmt.mantissa_bits)
      else
        # Normal: implicit leading 1
        true_exp = exp_int - bits.fmt.bias
        mantissa_value = 1.0 + mant_int.to_f / (1 << bits.fmt.mantissa_bits)
      end

      value = mantissa_value * (2.0**true_exp)
      value = -value if bits.sign == 1
      value
    end

    # -----------------------------------------------------------------------
    # Special value detection -- using logic gates
    # -----------------------------------------------------------------------
    # These functions detect special IEEE 754 values by examining the bit
    # pattern. We use AND and OR from logic_gates to check bit fields,
    # staying true to the "built from gates" philosophy.

    # Check if all bits in a list are 1, using AND gates.
    #
    # In hardware, this would be a wide AND gate:
    #     all_ones = AND(bit[0], AND(bit[1], AND(bit[2], ...)))
    #
    # @param bits [Array<Integer>] List of bits.
    # @return [Boolean] true if all bits are 1.
    def self.all_ones?(bits)
      result = bits[0]
      (1...bits.length).each do |i|
        result = LogicGates.and_gate(result, bits[i])
      end
      result == 1
    end

    # Check if all bits in a list are 0, using OR gates then NOT.
    #
    # In hardware: NOR across all bits.
    #
    # @param bits [Array<Integer>] List of bits.
    # @return [Boolean] true if all bits are 0.
    def self.all_zeros?(bits)
      result = bits[0]
      (1...bits.length).each do |i|
        result = LogicGates.or_gate(result, bits[i])
      end
      result == 0
    end

    # Check if a FloatBits represents NaN (Not a Number).
    #
    # NaN is defined as: exponent = all 1s AND mantissa != all 0s.
    #
    # @param bits [FloatBits] The FloatBits to check.
    # @return [Boolean] true if the value is NaN.
    def self.nan?(bits)
      all_ones?(bits.exponent) && !all_zeros?(bits.mantissa)
    end

    # Check if a FloatBits represents Infinity (+Inf or -Inf).
    #
    # Infinity is defined as: exponent = all 1s AND mantissa = all 0s.
    #
    # @param bits [FloatBits] The FloatBits to check.
    # @return [Boolean] true if the value is +Inf or -Inf.
    def self.inf?(bits)
      all_ones?(bits.exponent) && all_zeros?(bits.mantissa)
    end

    # Check if a FloatBits represents zero (+0 or -0).
    #
    # Zero is defined as: exponent = all 0s AND mantissa = all 0s.
    #
    # @param bits [FloatBits] The FloatBits to check.
    # @return [Boolean] true if the value is +0 or -0.
    def self.zero?(bits)
      all_zeros?(bits.exponent) && all_zeros?(bits.mantissa)
    end

    # Check if a FloatBits represents a denormalized (subnormal) number.
    #
    # Denormalized is defined as: exponent = all 0s AND mantissa != all 0s.
    #
    # Denormalized numbers fill the "underflow gap" near zero. When the
    # exponent is all zeros, the implicit bit becomes 0 instead of 1,
    # and the true exponent is fixed at (1 - bias). This allows gradual
    # underflow rather than a sudden jump to zero.
    #
    # @param bits [FloatBits] The FloatBits to check.
    # @return [Boolean] true if the value is denormalized.
    def self.denormalized?(bits)
      all_zeros?(bits.exponent) && !all_zeros?(bits.mantissa)
    end
  end
end
