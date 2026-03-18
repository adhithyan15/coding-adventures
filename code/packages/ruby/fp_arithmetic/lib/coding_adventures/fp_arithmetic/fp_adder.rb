# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Floating-point addition and subtraction -- built from logic gates.
# ---------------------------------------------------------------------------
#
# === How FP addition works at the hardware level ===
#
# Adding two floating-point numbers is surprisingly complex compared to
# integer addition. The core difficulty is that the two numbers might have
# very different exponents, so their mantissas are "misaligned" and must be
# shifted before they can be added.
#
# Consider adding 1.5 + 0.125 in decimal scientific notation:
#     1.5 x 10^0  +  1.25 x 10^-1
#
# You can't just add 1.5 + 1.25 because they have different exponents.
# First, you align them to the same exponent:
#     1.5   x 10^0
#     0.125 x 10^0   (shifted 1.25 right by 1 decimal place)
#     ─────────────
#     1.625 x 10^0
#
# Binary FP addition follows the exact same principle, but with binary
# mantissas and power-of-2 exponents.
#
# === The five steps of FP addition ===
#
#     Step 1: Compare exponents
#     Step 2: Align mantissas
#     Step 3: Add or subtract mantissas
#     Step 4: Normalize
#     Step 5: Round

module CodingAdventures
  module FpArithmetic
    # Add two floating-point numbers using logic gates.
    #
    # This implements the full IEEE 754 addition algorithm:
    # 1. Handle special cases (NaN, Inf, Zero)
    # 2. Compare exponents
    # 3. Align mantissas
    # 4. Add/subtract mantissas
    # 5. Normalize result
    # 6. Round to nearest even
    #
    # @param a [FloatBits] First operand.
    # @param b [FloatBits] Second operand. Must use same FloatFormat as a.
    # @return [FloatBits] The sum in the same format.
    def self.fp_add(a, b)
      fmt = a.fmt

      # ===================================================================
      # Step 0: Handle special cases
      # ===================================================================
      # IEEE 754 defines strict rules for special values:
      #   NaN + anything = NaN
      #   Inf + (-Inf) = NaN
      #   Inf + x = Inf (for finite x)
      #   0 + x = x

      # NaN propagation
      if nan?(a) || nan?(b)
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
          fmt: fmt
        )
      end

      # Infinity handling
      a_inf = inf?(a)
      b_inf = inf?(b)
      if a_inf && b_inf
        if a.sign == b.sign
          return FloatBits.new(
            sign: a.sign,
            exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: Array.new(fmt.mantissa_bits, 0),
            fmt: fmt
          )
        else
          # Inf + (-Inf) = NaN
          return FloatBits.new(
            sign: 0,
            exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
            fmt: fmt
          )
        end
      end
      return a if a_inf
      return b if b_inf

      # Zero handling
      a_zero = zero?(a)
      b_zero = zero?(b)
      if a_zero && b_zero
        result_sign = LogicGates.and_gate(a.sign, b.sign)
        return FloatBits.new(
          sign: result_sign,
          exponent: Array.new(fmt.exponent_bits, 0),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end
      return b if a_zero
      return a if b_zero

      # ===================================================================
      # Step 1: Extract exponents and mantissas as integers
      # ===================================================================
      exp_a = bits_msb_to_int(a.exponent)
      exp_b = bits_msb_to_int(b.exponent)
      mant_a = bits_msb_to_int(a.mantissa)
      mant_b = bits_msb_to_int(b.mantissa)

      # Add implicit leading 1 for normal numbers
      if exp_a != 0
        mant_a = (1 << fmt.mantissa_bits) | mant_a
      else
        exp_a = 1
      end
      if exp_b != 0
        mant_b = (1 << fmt.mantissa_bits) | mant_b
      else
        exp_b = 1
      end

      # Add 3 guard bits for rounding precision
      guard_bits = 3
      mant_a <<= guard_bits
      mant_b <<= guard_bits

      # ===================================================================
      # Step 2: Align mantissas by shifting the smaller one right
      # ===================================================================
      if exp_a >= exp_b
        exp_diff = exp_a - exp_b
        if exp_diff > 0 && exp_diff < (fmt.mantissa_bits + 1 + guard_bits)
          shifted_out = mant_b & ((1 << exp_diff) - 1)
          sticky = shifted_out != 0 ? 1 : 0
        else
          sticky = (mant_b != 0 && exp_diff > 0) ? 1 : 0
        end
        mant_b >>= exp_diff
        mant_b |= 1 if sticky == 1 && exp_diff > 0
        result_exp = exp_a
      else
        exp_diff = exp_b - exp_a
        if exp_diff > 0 && exp_diff < (fmt.mantissa_bits + 1 + guard_bits)
          shifted_out = mant_a & ((1 << exp_diff) - 1)
          sticky = shifted_out != 0 ? 1 : 0
        else
          sticky = (mant_a != 0 && exp_diff > 0) ? 1 : 0
        end
        mant_a >>= exp_diff
        mant_a |= 1 if sticky == 1 && exp_diff > 0
        result_exp = exp_b
      end

      # ===================================================================
      # Step 3: Add or subtract mantissas based on signs
      # ===================================================================
      if a.sign == b.sign
        result_mant = mant_a + mant_b
        result_sign = a.sign
      else
        if mant_a >= mant_b
          result_mant = mant_a - mant_b
          result_sign = a.sign
        else
          result_mant = mant_b - mant_a
          result_sign = b.sign
        end
      end

      # ===================================================================
      # Step 4: Handle zero result
      # ===================================================================
      if result_mant == 0
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 0),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # ===================================================================
      # Step 5: Normalize the result
      # ===================================================================
      normal_pos = fmt.mantissa_bits + guard_bits
      leading_pos = result_mant.bit_length - 1

      if leading_pos > normal_pos
        shift_amount = leading_pos - normal_pos
        lost_bits = result_mant & ((1 << shift_amount) - 1)
        result_mant >>= shift_amount
        result_mant |= 1 if lost_bits != 0
        result_exp += shift_amount
      elsif leading_pos < normal_pos
        shift_amount = normal_pos - leading_pos
        if result_exp - shift_amount >= 1
          result_mant <<= shift_amount
          result_exp -= shift_amount
        else
          actual_shift = result_exp - 1
          result_mant <<= actual_shift if actual_shift > 0
          result_exp = 0
        end
      end

      # ===================================================================
      # Step 6: Round to nearest even
      # ===================================================================
      guard = (result_mant >> (guard_bits - 1)) & 1
      round_bit = (result_mant >> (guard_bits - 2)) & 1
      sticky_bit = result_mant & ((1 << (guard_bits - 2)) - 1)
      sticky_bit = sticky_bit != 0 ? 1 : 0

      # Remove guard bits
      result_mant >>= guard_bits

      # Apply rounding
      if guard == 1
        if round_bit == 1 || sticky_bit == 1
          result_mant += 1
        elsif (result_mant & 1) == 1
          result_mant += 1
        end
      end

      # Check if rounding caused overflow
      if result_mant >= (1 << (fmt.mantissa_bits + 1))
        result_mant >>= 1
        result_exp += 1
      end

      # ===================================================================
      # Step 7: Handle exponent overflow/underflow
      # ===================================================================
      max_exp = (1 << fmt.exponent_bits) - 1

      if result_exp >= max_exp
        return FloatBits.new(
          sign: result_sign,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      if result_exp <= 0
        if result_exp < -(fmt.mantissa_bits)
          return FloatBits.new(
            sign: result_sign,
            exponent: Array.new(fmt.exponent_bits, 0),
            mantissa: Array.new(fmt.mantissa_bits, 0),
            fmt: fmt
          )
        end
        shift = 1 - result_exp
        result_mant >>= shift
        result_exp = 0
      end

      # ===================================================================
      # Step 8: Pack the result
      # ===================================================================
      result_mant &= (1 << fmt.mantissa_bits) - 1 if result_exp > 0

      FloatBits.new(
        sign: result_sign,
        exponent: int_to_bits_msb(result_exp, fmt.exponent_bits),
        mantissa: int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt: fmt
      )
    end

    # Subtract two floating-point numbers: a - b.
    #
    # In IEEE 754, a - b = a + (-b). To negate b, we just flip its sign bit.
    # This is a single XOR gate in hardware.
    #
    # @param a [FloatBits] The minuend.
    # @param b [FloatBits] The subtrahend.
    # @return [FloatBits] a - b as FloatBits.
    def self.fp_sub(a, b)
      neg_b = FloatBits.new(
        sign: LogicGates.xor_gate(b.sign, 1),
        exponent: b.exponent,
        mantissa: b.mantissa,
        fmt: b.fmt
      )
      fp_add(a, neg_b)
    end

    # Negate a floating-point number: return -a.
    #
    # The simplest FP operation: just flip the sign bit.
    # In hardware, it's literally one XOR gate.
    #
    # @param a [FloatBits] The number to negate.
    # @return [FloatBits] -a as FloatBits.
    def self.fp_neg(a)
      FloatBits.new(
        sign: LogicGates.xor_gate(a.sign, 1),
        exponent: a.exponent,
        mantissa: a.mantissa,
        fmt: a.fmt
      )
    end

    # Return the absolute value of a floating-point number.
    #
    # Even simpler than negation: just force the sign bit to 0.
    #
    # @param a [FloatBits] The input number.
    # @return [FloatBits] |a| as FloatBits.
    def self.fp_abs(a)
      FloatBits.new(
        sign: 0,
        exponent: a.exponent,
        mantissa: a.mantissa,
        fmt: a.fmt
      )
    end

    # Compare two floating-point numbers.
    #
    # Returns:
    #   -1 if a < b
    #    0 if a == b
    #    1 if a > b
    #
    # NaN comparisons always return 0 (unordered).
    #
    # @param a [FloatBits] First operand.
    # @param b [FloatBits] Second operand.
    # @return [Integer] -1, 0, or 1.
    def self.fp_compare(a, b)
      # NaN is unordered
      return 0 if nan?(a) || nan?(b)

      # Handle zeros: +0 == -0
      return 0 if zero?(a) && zero?(b)

      # Different signs: positive > negative
      if a.sign != b.sign
        return (b.sign == 1 ? 1 : -1) if zero?(a)
        return (a.sign == 1 ? -1 : 1) if zero?(b)
        return a.sign == 1 ? -1 : 1
      end

      # Same sign: compare exponent, then mantissa
      exp_a = bits_msb_to_int(a.exponent)
      exp_b = bits_msb_to_int(b.exponent)
      mant_a = bits_msb_to_int(a.mantissa)
      mant_b = bits_msb_to_int(b.mantissa)

      if exp_a != exp_b
        if a.sign == 0
          return exp_a > exp_b ? 1 : -1
        else
          return exp_a > exp_b ? -1 : 1
        end
      end

      if mant_a != mant_b
        if a.sign == 0
          return mant_a > mant_b ? 1 : -1
        else
          return mant_a > mant_b ? -1 : 1
        end
      end

      0
    end
  end
end
