-- fma.lua -- Fused Multiply-Add and format conversion
--
-- === What is FMA (Fused Multiply-Add)? ===
--
-- FMA computes a * b + c with only ONE rounding step at the end. Compare:
--
--   Without FMA (separate operations):
--       temp = fp_mul(a, b)     -- round #1 (loses precision)
--       result = fp_add(temp, c)  -- round #2 (loses more precision)
--
--   With FMA:
--       result = fma(a, b, c)  -- round only once!
--
-- === Why FMA matters for ML ===
--
-- In machine learning, the dominant computation is the dot product:
--
--   result = sum(a[i] * w[i] for i in range(N))
--
-- Each multiply-add in the sum is a potential FMA. By rounding only once per
-- operation instead of twice, FMA gives more accurate gradients during training.
--
-- Every modern processor has FMA:
--   - Intel Haswell (2013): FMA3 instruction (AVX2)
--   - NVIDIA GPUs: native FMA in CUDA cores
--   - Google TPU: the MAC (Multiply-Accumulate) unit IS an FMA
--   - Apple M-series: FMA in both CPU and Neural Engine
--
-- === Algorithm ===
--
--   Step 1: Multiply a * b with FULL precision (no rounding!)
--   Step 2: Align c's mantissa to the product's exponent
--   Step 3: Add the full-precision product and aligned c
--   Step 4: Normalize and round ONCE

local formats = require("coding_adventures.fp_arithmetic.formats")
local ieee754 = require("coding_adventures.fp_arithmetic.ieee754")
local logic_gates = require("coding_adventures.logic_gates")

local FloatBits = formats.FloatBits
local int_to_bits_msb = formats.int_to_bits_msb
local bits_msb_to_int = formats.bits_msb_to_int
local bit_length = formats.bit_length
local make_nan = formats.make_nan
local make_inf = formats.make_inf
local make_zero = formats.make_zero

local float_to_bits = ieee754.float_to_bits
local bits_to_float = ieee754.bits_to_float
local is_nan = ieee754.is_nan
local is_inf = ieee754.is_inf
local is_zero = ieee754.is_zero

--- Computes a * b + c with a single rounding step (fused multiply-add).
---
--- === Worked example: fma(1.5, 2.0, 0.25) in FP32 ===
---
---   a = 1.5 = 1.1 x 2^0    (exp=127, mant=1.100...0)
---   b = 2.0 = 1.0 x 2^1    (exp=128, mant=1.000...0)
---   c = 0.25 = 1.0 x 2^-2  (exp=125, mant=1.000...0)
---
---   Step 1: Full-precision multiply
---           1.100...0 x 1.000...0 = 1.100...0 (48-bit, no rounding)
---           Product exponent: 127 + 128 - 127 = 128 (true exp = 1)
---
---   Step 2: Align c to product's exponent
---           c = 1.0 x 2^-2, product exponent = 128
---           Shift c right by 128 - 125 = 3 positions
---
---   Step 3: Add
---           1.100 x 2^1 + 0.001 x 2^1 = 1.101 x 2^1
---
---   Step 4: Normalize and round
---           Already normalized, result = 1.101 x 2^1 = 3.25
---           Check: 1.5 * 2.0 + 0.25 = 3.0 + 0.25 = 3.25 correct!
---
--- @param a table FloatBits operand (multiplied)
--- @param b table FloatBits operand (multiplied)
--- @param c table FloatBits operand (added)
--- @return table FloatBits result (a * b + c)
local function fma(a, b, c)
    local fmt = a.fmt

    -- ===================================================================
    -- Step 0: Handle special cases
    -- ===================================================================
    if is_nan(a) or is_nan(b) or is_nan(c) then
        return make_nan(fmt)
    end

    local a_inf = is_inf(a)
    local b_inf = is_inf(b)
    local c_inf = is_inf(c)
    local a_zero = is_zero(a)
    local b_zero = is_zero(b)

    -- Inf * 0 = NaN
    if (a_inf and b_zero) or (b_inf and a_zero) then
        return make_nan(fmt)
    end

    local product_sign = logic_gates.XOR(a.sign, b.sign)

    -- Inf * finite + c
    if a_inf or b_inf then
        if c_inf and product_sign ~= c.sign then
            return make_nan(fmt)  -- Inf + (-Inf) = NaN
        end
        return make_inf(product_sign, fmt)
    end

    -- a * b = 0, result is just c
    if a_zero or b_zero then
        if is_zero(c) then
            local result_sign = logic_gates.AND(product_sign, c.sign)
            return make_zero(result_sign, fmt)
        end
        return c
    end

    -- c is Inf
    if c_inf then
        return c
    end

    -- ===================================================================
    -- Step 1: Multiply a * b with full precision (no rounding!)
    -- ===================================================================
    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)

    if exp_a ~= 0 then
        mant_a = (1 << fmt.mantissa_bits) | mant_a
    else
        exp_a = 1
    end
    if exp_b ~= 0 then
        mant_b = (1 << fmt.mantissa_bits) | mant_b
    else
        exp_b = 1
    end

    -- Full-precision product: no truncation, no rounding!
    local product = mant_a * mant_b
    local product_exp = exp_a + exp_b - fmt.bias

    -- Normalize the product
    local product_leading = bit_length(product) - 1
    local normal_product_pos = 2 * fmt.mantissa_bits

    if product_leading > normal_product_pos then
        product_exp = product_exp + (product_leading - normal_product_pos)
    elseif product_leading < normal_product_pos then
        product_exp = product_exp - (normal_product_pos - product_leading)
    end

    -- ===================================================================
    -- Step 2: Align c's mantissa to the product's exponent
    -- ===================================================================
    local exp_c = bits_msb_to_int(c.exponent)
    local mant_c = bits_msb_to_int(c.mantissa)

    if exp_c ~= 0 then
        mant_c = (1 << fmt.mantissa_bits) | mant_c
    else
        exp_c = 1
    end

    local exp_diff = product_exp - exp_c

    local c_scale_shift = product_leading - fmt.mantissa_bits
    local c_aligned
    if c_scale_shift >= 0 then
        c_aligned = mant_c << c_scale_shift
    else
        c_aligned = mant_c >> (-c_scale_shift)
    end

    local result_exp
    if exp_diff >= 0 then
        c_aligned = c_aligned >> exp_diff
        result_exp = product_exp
    else
        product = product >> (-exp_diff)
        result_exp = exp_c
    end

    -- ===================================================================
    -- Step 3: Add product and c
    -- ===================================================================
    local result_mant, result_sign
    if product_sign == c.sign then
        result_mant = product + c_aligned
        result_sign = product_sign
    else
        if product >= c_aligned then
            result_mant = product - c_aligned
            result_sign = product_sign
        else
            result_mant = c_aligned - product
            result_sign = c.sign
        end
    end

    if result_mant == 0 then
        return make_zero(0, fmt)
    end

    -- ===================================================================
    -- Step 4: Normalize and round ONCE
    -- ===================================================================
    local result_leading = bit_length(result_mant) - 1
    local target_pos = product_leading
    if target_pos < fmt.mantissa_bits then
        target_pos = fmt.mantissa_bits
    end

    if result_leading > target_pos then
        local shift = result_leading - target_pos
        result_exp = result_exp + shift
    elseif result_leading < target_pos then
        local shift_needed = target_pos - result_leading
        result_exp = result_exp - shift_needed
    end

    -- Round to mantissa_bits precision
    result_leading = bit_length(result_mant) - 1
    local round_pos = result_leading - fmt.mantissa_bits

    if round_pos > 0 then
        local guard = (result_mant >> (round_pos - 1)) & 1
        local round_bit, sticky = 0, 0
        if round_pos >= 2 then
            round_bit = (result_mant >> (round_pos - 2)) & 1
            local mask = (1 << (round_pos - 2)) - 1
            if (result_mant & mask) ~= 0 then
                sticky = 1
            end
        end

        result_mant = result_mant >> round_pos

        -- Round to nearest even
        if guard == 1 then
            if round_bit == 1 or sticky == 1 then
                result_mant = result_mant + 1
            elseif (result_mant & 1) == 1 then
                result_mant = result_mant + 1
            end
        end

        if result_mant >= (1 << (fmt.mantissa_bits + 1)) then
            result_mant = result_mant >> 1
            result_exp = result_exp + 1
        end
    elseif round_pos < 0 then
        result_mant = result_mant << (-round_pos)
    end

    -- Handle exponent overflow/underflow
    local max_exp = (1 << fmt.exponent_bits) - 1

    if result_exp >= max_exp then
        return make_inf(result_sign, fmt)
    end

    if result_exp <= 0 then
        if result_exp < -(fmt.mantissa_bits) then
            return make_zero(result_sign, fmt)
        end
        local shift = 1 - result_exp
        result_mant = result_mant >> shift
        result_exp = 0
    end

    -- Remove implicit leading 1
    if result_exp > 0 then
        result_mant = result_mant & ((1 << fmt.mantissa_bits) - 1)
    end

    return FloatBits.new(
        result_sign,
        int_to_bits_msb(result_exp, fmt.exponent_bits),
        int_to_bits_msb(result_mant, fmt.mantissa_bits),
        fmt
    )
end

--- Converts a floating-point number from one format to another.
---
--- === Why format conversion matters ===
---
--- In ML pipelines, data frequently changes precision:
---   - Training starts in FP32 (full precision)
---   - Forward pass uses FP16 or BF16 (faster, less memory)
---   - Gradients accumulated in FP32 (need precision)
---   - Weights stored as BF16 on TPU
---
--- === FP32 -> BF16 conversion (trivially simple!) ===
---
--- BF16 was designed so that conversion from FP32 is dead simple:
--- just truncate the lower 16 bits! Both formats use the same 8-bit
--- exponent with bias 127, so no exponent adjustment is needed.
---
--- @param bits table FloatBits source value
--- @param target_fmt table Target FloatFormat
--- @return table FloatBits in the target format
local function fp_convert(bits, target_fmt)
    -- Same format: no conversion needed
    if bits.fmt == target_fmt then
        return bits
    end

    -- Strategy: decode to Lua float64, then re-encode in target format.
    -- This handles all edge cases correctly.
    local value = bits_to_float(bits)
    return float_to_bits(value, target_fmt)
end

-- =========================================================================
-- Module exports
-- =========================================================================

return {
    fma = fma,
    fp_convert = fp_convert,
}
