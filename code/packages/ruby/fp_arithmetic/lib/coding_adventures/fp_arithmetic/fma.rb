# frozen_string_literal: true

# ---------------------------------------------------------------------------
# Fused Multiply-Add and format conversion.
# ---------------------------------------------------------------------------
#
# === What is FMA (Fused Multiply-Add)? ===
#
# FMA computes a * b + c with only ONE rounding step at the end. Compare:
#
#     Without FMA (separate operations):
#         temp = fp_mul(a, b)     # round #1 (loses precision)
#         result = fp_add(temp, c)  # round #2 (loses more precision)
#
#     With FMA:
#         result = fp_fma(a, b, c)  # round only once!
#
# === Why FMA matters for ML ===
#
# In machine learning, the dominant computation is the dot product:
#     result = sum(a[i] * w[i] for i in 0...N)
#
# Each multiply-add in the sum is a potential FMA. By rounding only once
# per operation instead of twice, FMA gives more accurate gradients.
#
# Every modern processor has FMA:
# - Intel Haswell (2013): FMA3 instruction (AVX2)
# - NVIDIA GPUs: native FMA in CUDA cores
# - Google TPU: the MAC (Multiply-Accumulate) unit IS an FMA
# - Apple M-series: FMA in both CPU and Neural Engine

module CodingAdventures
  module FpArithmetic
    # Fused multiply-add: compute a * b + c with single rounding.
    #
    # @param a [FloatBits] First multiplicand.
    # @param b [FloatBits] Second multiplicand.
    # @param c [FloatBits] Addend.
    # @return [FloatBits] a * b + c with only one rounding step.
    def self.fp_fma(a, b, c)
      fmt = a.fmt

      # ===================================================================
      # Step 0: Handle special cases
      # ===================================================================
      if nan?(a) || nan?(b) || nan?(c)
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
          fmt: fmt
        )
      end

      a_inf = inf?(a)
      b_inf = inf?(b)
      c_inf = inf?(c)
      a_zero = zero?(a)
      b_zero = zero?(b)

      # Inf * 0 = NaN
      if (a_inf && b_zero) || (b_inf && a_zero)
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
          fmt: fmt
        )
      end

      product_sign = LogicGates.xor_gate(a.sign, b.sign)

      # Inf * finite + c
      if a_inf || b_inf
        if c_inf && product_sign != c.sign
          # Inf + (-Inf) = NaN
          return FloatBits.new(
            sign: 0,
            exponent: Array.new(fmt.exponent_bits, 1),
            mantissa: [1] + Array.new(fmt.mantissa_bits - 1, 0),
            fmt: fmt
          )
        end
        return FloatBits.new(
          sign: product_sign,
          exponent: Array.new(fmt.exponent_bits, 1),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # a * b = 0, result is just c
      if a_zero || b_zero
        if zero?(c)
          result_sign = LogicGates.and_gate(product_sign, c.sign)
          return FloatBits.new(
            sign: result_sign,
            exponent: Array.new(fmt.exponent_bits, 0),
            mantissa: Array.new(fmt.mantissa_bits, 0),
            fmt: fmt
          )
        end
        return c
      end

      # c is Inf
      return c if c_inf

      # ===================================================================
      # Step 1: Multiply a * b with full precision (no rounding!)
      # ===================================================================
      exp_a = bits_msb_to_int(a.exponent)
      exp_b = bits_msb_to_int(b.exponent)
      mant_a = bits_msb_to_int(a.mantissa)
      mant_b = bits_msb_to_int(b.mantissa)

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

      # Full-precision product: no truncation, no rounding!
      product = mant_a * mant_b
      product_exp = exp_a + exp_b - fmt.bias

      # Normalize the product position
      product_leading = product.bit_length - 1
      normal_product_pos = 2 * fmt.mantissa_bits

      if product_leading > normal_product_pos
        product_exp += product_leading - normal_product_pos
      elsif product_leading < normal_product_pos
        product_exp -= normal_product_pos - product_leading
      end

      # ===================================================================
      # Step 2: Align c's mantissa to the product's exponent
      # ===================================================================
      exp_c = bits_msb_to_int(c.exponent)
      mant_c = bits_msb_to_int(c.mantissa)

      if exp_c != 0
        mant_c = (1 << fmt.mantissa_bits) | mant_c
      else
        exp_c = 1
      end

      exp_diff = product_exp - exp_c

      c_scale_shift = product_leading - fmt.mantissa_bits
      c_aligned = if c_scale_shift >= 0
        mant_c << c_scale_shift
      else
        mant_c >> (-c_scale_shift)
      end

      if exp_diff >= 0
        c_aligned >>= exp_diff
        result_exp = product_exp
      else
        product >>= (-exp_diff)
        result_exp = exp_c
      end

      # ===================================================================
      # Step 3: Add product and c
      # ===================================================================
      if product_sign == c.sign
        result_mant = product + c_aligned
        result_sign = product_sign
      else
        if product >= c_aligned
          result_mant = product - c_aligned
          result_sign = product_sign
        else
          result_mant = c_aligned - product
          result_sign = c.sign
        end
      end

      # Handle zero result
      if result_mant == 0
        return FloatBits.new(
          sign: 0,
          exponent: Array.new(fmt.exponent_bits, 0),
          mantissa: Array.new(fmt.mantissa_bits, 0),
          fmt: fmt
        )
      end

      # ===================================================================
      # Step 4: Normalize and round ONCE
      # ===================================================================
      result_leading = result_mant.bit_length - 1
      target_pos = product_leading > fmt.mantissa_bits ? product_leading : fmt.mantissa_bits

      if result_leading > target_pos
        shift = result_leading - target_pos
        result_exp += shift
      elsif result_leading < target_pos
        shift_needed = target_pos - result_leading
        result_exp -= shift_needed
      end

      # Round to mantissa_bits precision
      result_leading = result_mant.bit_length - 1
      round_pos = result_leading - fmt.mantissa_bits

      if round_pos > 0
        guard = (result_mant >> (round_pos - 1)) & 1
        if round_pos >= 2
          round_bit = (result_mant >> (round_pos - 2)) & 1
          sticky = (result_mant & ((1 << (round_pos - 2)) - 1)) != 0 ? 1 : 0
        else
          round_bit = 0
          sticky = 0
        end

        result_mant >>= round_pos

        # Round to nearest even
        if guard == 1
          if round_bit == 1 || sticky == 1
            result_mant += 1
          elsif (result_mant & 1) == 1
            result_mant += 1
          end
        end

        # Check rounding overflow
        if result_mant >= (1 << (fmt.mantissa_bits + 1))
          result_mant >>= 1
          result_exp += 1
        end
      elsif round_pos < 0
        result_mant <<= (-round_pos)
      end

      # Handle exponent overflow/underflow
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

      # Remove implicit leading 1
      result_mant &= (1 << fmt.mantissa_bits) - 1 if result_exp > 0

      FloatBits.new(
        sign: result_sign,
        exponent: int_to_bits_msb(result_exp, fmt.exponent_bits),
        mantissa: int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt: fmt
      )
    end

    # -----------------------------------------------------------------------
    # Format conversion: FP32 <-> FP16 <-> BF16
    # -----------------------------------------------------------------------

    # Convert a floating-point number from one format to another.
    #
    # In ML pipelines, data frequently changes precision:
    # - Training starts in FP32 (full precision)
    # - Forward pass uses FP16 or BF16 (faster, less memory)
    # - Gradients accumulated in FP32 (need precision)
    # - Weights stored as BF16 on TPU
    #
    # @param bits [FloatBits] The source FloatBits.
    # @param target_fmt [FloatFormat] The target FloatFormat.
    # @return [FloatBits] The value in the target format.
    def self.fp_convert(bits, target_fmt)
      return bits if bits.fmt == target_fmt

      # Strategy: decode to Ruby float, then re-encode in target format.
      value = bits_to_float(bits)
      float_to_bits(value, target_fmt)
    end
  end
end
