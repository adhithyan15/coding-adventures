-- fp_multiplier.lua -- Floating-point multiplication built from logic gates
--
-- === How FP multiplication works ===
--
-- Floating-point multiplication is actually simpler than addition! That's because
-- you don't need to align mantissas -- the exponents just add together.
--
-- In scientific notation:
--
--   (1.5 x 10^3) x (2.0 x 10^2) = (1.5 x 2.0) x 10^(3+2) = 3.0 x 10^5
--
-- The same principle applies in binary:
--
--   (-1)^s1 x 1.m1 x 2^e1  *  (-1)^s2 x 1.m2 x 2^e2
--   = (-1)^(s1 XOR s2) x (1.m1 x 1.m2) x 2^(e1 + e2)
--
-- === The four steps of FP multiplication ===
--
--   Step 1: Result sign = XOR of input signs
--           Positive x Positive = Positive (0 XOR 0 = 0)
--           Positive x Negative = Negative (0 XOR 1 = 1)
--           Negative x Negative = Positive (1 XOR 1 = 0)
--
--   Step 2: Result exponent = exp_a + exp_b - bias
--           We subtract the bias once because both exponents include it.
--
--   Step 3: Multiply mantissas using shift-and-add
--           The result is double-width (e.g., 48 bits for FP32's 24-bit mantissas).
--
--   Step 4: Normalize and round (same as addition)

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

local is_nan = ieee754.is_nan
local is_inf = ieee754.is_inf
local is_zero = ieee754.is_zero

--- Multiplies two floating-point numbers using the IEEE 754 algorithm.
---
--- === Worked example: 1.5 x 2.0 in FP32 ===
---
---   1.5 = 1.1 x 2^0    -> sign=0, exp=127, mant=100...0
---   2.0 = 1.0 x 2^1    -> sign=0, exp=128, mant=000...0
---
---   Step 1: result_sign = 0 XOR 0 = 0 (positive)
---   Step 2: result_exp = 127 + 128 - 127 = 128 (true exp = 1)
---   Step 3: mantissa product:
---           1.100...0 x 1.000...0 = 1.100...0 (trivial case)
---   Step 4: Already normalized
---   Result: 1.1 x 2^1 = 3.0 (correct!)
---
--- @param a table FloatBits operand
--- @param b table FloatBits operand
--- @return table FloatBits result (a * b)
local function fp_mul(a, b)
    local fmt = a.fmt

    -- ===================================================================
    -- Step 0: Handle special cases
    -- ===================================================================
    -- IEEE 754 rules for multiplication:
    --   NaN x anything = NaN
    --   Inf x 0 = NaN
    --   Inf x finite = Inf (with appropriate sign)
    --   0 x finite = 0

    -- Result sign: always XOR of input signs (even for special cases)
    local result_sign = logic_gates.XOR(a.sign, b.sign)

    -- NaN propagation
    if is_nan(a) or is_nan(b) then
        return make_nan(fmt)
    end

    local a_inf = is_inf(a)
    local b_inf = is_inf(b)
    local a_zero = is_zero(a)
    local b_zero = is_zero(b)

    -- Inf x 0 = NaN (undefined)
    if (a_inf and b_zero) or (b_inf and a_zero) then
        return make_nan(fmt)
    end

    -- Inf x anything = Inf
    if a_inf or b_inf then
        return make_inf(result_sign, fmt)
    end

    -- Zero x anything = Zero
    if a_zero or b_zero then
        return make_zero(result_sign, fmt)
    end

    -- ===================================================================
    -- Step 1: Extract exponents and mantissas
    -- ===================================================================
    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)

    -- Add implicit leading 1 for normal numbers
    if exp_a ~= 0 then
        mant_a = (1 << fmt.mantissa_bits) | mant_a
    else
        exp_a = 1  -- Denormal: true exponent = 1 - bias
    end

    if exp_b ~= 0 then
        mant_b = (1 << fmt.mantissa_bits) | mant_b
    else
        exp_b = 1
    end

    -- ===================================================================
    -- Step 2: Add exponents, subtract bias
    -- ===================================================================
    local result_exp = exp_a + exp_b - fmt.bias

    -- ===================================================================
    -- Step 3: Multiply mantissas (shift-and-add)
    -- ===================================================================
    --
    -- The mantissa product of two (mantissa_bits+1)-bit numbers produces
    -- a (2*(mantissa_bits+1))-bit result. We use Lua integer multiplication.
    local product = mant_a * mant_b

    -- ===================================================================
    -- Step 4: Normalize
    -- ===================================================================
    local leading_pos = bit_length(product) - 1
    local normal_pos = 2 * fmt.mantissa_bits

    if leading_pos > normal_pos then
        local extra = leading_pos - normal_pos
        result_exp = result_exp + extra
    elseif leading_pos < normal_pos then
        local deficit = normal_pos - leading_pos
        result_exp = result_exp - deficit
    end

    -- ===================================================================
    -- Step 5: Round to nearest even
    -- ===================================================================
    local round_pos = leading_pos - fmt.mantissa_bits

    local result_mant
    if round_pos > 0 then
        local guard = (product >> (round_pos - 1)) & 1
        local round_bit, sticky = 0, 0
        if round_pos >= 2 then
            round_bit = (product >> (round_pos - 2)) & 1
            local mask = (1 << (round_pos - 2)) - 1
            if (product & mask) ~= 0 then
                sticky = 1
            end
        end

        result_mant = product >> round_pos

        -- Apply rounding
        if guard == 1 then
            if round_bit == 1 or sticky == 1 then
                result_mant = result_mant + 1
            elseif (result_mant & 1) == 1 then
                result_mant = result_mant + 1
            end
        end

        -- Check if rounding caused mantissa overflow
        if result_mant >= (1 << (fmt.mantissa_bits + 1)) then
            result_mant = result_mant >> 1
            result_exp = result_exp + 1
        end
    elseif round_pos == 0 then
        result_mant = product
    else
        result_mant = product << (-round_pos)
    end

    -- ===================================================================
    -- Step 6: Handle exponent overflow/underflow
    -- ===================================================================
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

    -- ===================================================================
    -- Step 7: Pack the result
    -- ===================================================================
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

-- =========================================================================
-- Module exports
-- =========================================================================

return {
    fp_mul = fp_mul,
}
