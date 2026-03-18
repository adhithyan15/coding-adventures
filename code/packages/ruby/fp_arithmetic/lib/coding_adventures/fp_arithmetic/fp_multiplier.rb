# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Floating-point multiplication -- built from logic gates.
# ---------------------------------------------------------------------------
#
# === How FP multiplication works ===
#
# Floating-point multiplication is actually simpler than addition! That's
# because you don't need to align mantissas -- the exponents just add.
#
# In scientific notation:
#     (1.5 x 10^3) x (2.0 x 10^2) = (1.5 x 2.0) x 10^(3+2) = 3.0 x 10^5
#
# The same principle applies in binary:
#     (-1)^s1 x 1.m1 x 2^e1  *  (-1)^s2 x 1.m2 x 2^e2
#     = (-1)^(s1 XOR s2) x (1.m1 x 1.m2) x 2^(e1 + e2)
#
# === The four steps of FP multiplication ===
#
#     Step 1: Result sign = XOR of input signs
#     Step 2: Result exponent = exp_a + exp_b - bias
#     Step 3: Multiply mantissas using shift-and-add
#     Step 4: Normalize and round

module CodingAdventures
  module FpArithmetic
    # Multiply two floating-point numbers using logic gates.
    #
    # Implements the IEEE 754 multiplication algorithm:
    # 1. Handle special cases (NaN, Inf, Zero)
    # 2. XOR signs
    # 3. Add exponents, subtract bias
    # 4. Multiply mantissas (shift-and-add)
    # 5. Normalize and round
    #
    # @param a [FloatBits] First operand.
    # @param b [FloatBits] Second operand. Must use same FloatFormat as a.
    # @return [FloatBits] The product in the same format.
    def self.fp_mul(a, b)
      fmt = a.fmt

      # ===================================================================
      # Step 0: Handle special cases
      # ===================================================================
      result_sign = LogicGates.xor_gate(a.sign, b.sign)

      # NaN propagation
      if nan?(a) || nan?(b)
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
          fmt: fmt
        )
      end

      a_inf = inf?(a)
      b_inf = inf?(b)
      a_zero = zero?(a)
      b_zero = zero?(b)

      # Inf x 0 = NaN
      if (a_inf && b_zero) || (b_inf && a_zero)
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
          fmt: fmt
        )
      end

      # Inf x anything = Inf
      if a_inf || b_inf
        return FloatBits.new(
          sign: result_sign,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # Zero x anything = Zero
      if a_zero || b_zero
        return FloatBits.new(
          sign: result_sign,
          exponent: Array.new(fmt.exponent_bits, 0),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # ===================================================================
      # Step 1: Extract exponents and mantissas
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

      # ===================================================================
      # Step 2: Add exponents, subtract bias
      # ===================================================================
      result_exp = exp_a + exp_b - fmt.bias

      # ===================================================================
      # Step 3: Multiply mantissas (shift-and-add)
      # ===================================================================
      product = mant_a * mant_b

      # ===================================================================
      # Step 4: Normalize
      # ===================================================================
      leading_pos = product.bit_length - 1
      normal_pos = 2 * fmt.mantissa_bits

      if leading_pos > normal_pos
        extra = leading_pos - normal_pos
        result_exp += extra
      elsif leading_pos < normal_pos
        deficit = normal_pos - leading_pos
        result_exp -= deficit
      end

      # ===================================================================
      # Step 5: Round to nearest even
      # ===================================================================
      round_pos = leading_pos - fmt.mantissa_bits

      if round_pos > 0
        guard = (product >> (round_pos - 1)) & 1
        if round_pos >= 2
          round_bit = (product >> (round_pos - 2)) & 1
          sticky = (product & ((1 << (round_pos - 2)) - 1)) != 0 ? 1 : 0
        else
          round_bit = 0
          sticky = 0
        end

        result_mant = product >> round_pos

        if guard == 1
          if round_bit == 1 || sticky == 1
            result_mant += 1
          elsif (result_mant & 1) == 1
            result_mant += 1
          end
        end

        # Check if rounding caused mantissa overflow
        if result_mant >= (1 << (fmt.mantissa_bits + 1))
          result_mant >>= 1
          result_exp += 1
        end
      elsif round_pos == 0
        result_mant = product
      else
        result_mant = product << (-round_pos)
      end

      # ===================================================================
      # Step 6: Handle exponent overflow/underflow
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
      # Step 7: Pack the result
      # ===================================================================
      result_mant &= (1 << fmt.mantissa_bits) - 1 if result_exp > 0

      FloatBits.new(
        sign: result_sign,
        exponent: int_to_bits_msb(result_exp, fmt.exponent_bits),
        mantissa: int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt: fmt
      )
    end
  end
end
