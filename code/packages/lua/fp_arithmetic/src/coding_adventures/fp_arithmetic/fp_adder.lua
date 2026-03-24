-- fp_adder.lua -- Floating-point addition, subtraction, negation, abs, compare
--
-- === How FP addition works at the hardware level ===
--
-- Adding two floating-point numbers is surprisingly complex compared to integer
-- addition. The core difficulty is that the two numbers might have very different
-- exponents, so their mantissas are "misaligned" and must be shifted before they
-- can be added.
--
-- Consider adding 1.5 + 0.125 in decimal scientific notation:
--
--   1.5 x 10^0  +  1.25 x 10^-1
--
-- You can't just add 1.5 + 1.25 because they have different exponents. First,
-- you align them to the same exponent:
--
--   1.5   x 10^0
--   0.125 x 10^0   (shifted 1.25 right by 1 decimal place)
--   -------------
--   1.625 x 10^0
--
-- Binary FP addition follows the exact same principle, but with binary mantissas
-- and power-of-2 exponents.
--
-- === The five steps of FP addition ===
--
--   Step 1: Compare exponents
--           Subtract exponents to find the difference.
--           The number with the smaller exponent gets shifted.
--
--   Step 2: Align mantissas
--           Shift the smaller number's mantissa right by the exponent
--           difference. This is like converting 0.125 to line up with 1.5.
--
--   Step 3: Add or subtract mantissas
--           If signs are the same: add mantissas
--           If signs differ: subtract the smaller from the larger
--
--   Step 4: Normalize
--           The result might not be in 1.xxx form. Adjust:
--           - If overflow (10.xxx): shift right, increment exponent
--           - If underflow (0.0xxx): shift left, decrement exponent
--
--   Step 5: Round
--           The result might have more bits than the format allows.
--           Round to fit, using "round to nearest even" (banker's rounding).

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

--- Adds two floating-point numbers using the IEEE 754 algorithm.
---
--- This implements the full addition algorithm:
---  1. Handle special cases (NaN, Inf, Zero)
---  2. Compare exponents
---  3. Align mantissas
---  4. Add/subtract mantissas
---  5. Normalize result
---  6. Round to nearest even
---
--- === Worked example: 1.5 + 0.25 in FP32 ===
---
---   1.5 = 1.1 x 2^0    -> exp=127, mant=10000...0
---   0.25 = 1.0 x 2^-2   -> exp=125, mant=00000...0
---
---   Step 1: exp_diff = 127 - 125 = 2 (b has smaller exponent)
---   Step 2: Shift b's mantissa right by 2:
---           1.10000...0  (a, with implicit 1)
---           0.01000...0  (b, shifted right by 2)
---   Step 3: Add:  1.10000...0 + 0.01000...0 = 1.11000...0
---   Step 4: Already normalized (starts with 1.)
---   Step 5: No rounding needed (exact)
---   Result: 1.11 x 2^0 = 1.75 (correct!)
---
--- @param a table FloatBits operand
--- @param b table FloatBits operand
--- @return table FloatBits result
local function fp_add(a, b)
    local fmt = a.fmt

    -- ===================================================================
    -- Step 0: Handle special cases
    -- ===================================================================
    -- IEEE 754 defines strict rules for special values:
    --   NaN + anything = NaN
    --   Inf + (-Inf) = NaN
    --   Inf + x = Inf (for finite x)
    --   0 + x = x

    -- NaN propagation: any NaN input produces NaN output
    if is_nan(a) or is_nan(b) then
        return make_nan(fmt)
    end

    -- Infinity handling
    local a_inf = is_inf(a)
    local b_inf = is_inf(b)
    if a_inf and b_inf then
        if a.sign == b.sign then
            return make_inf(a.sign, fmt)
        end
        -- Inf + (-Inf) = NaN
        return make_nan(fmt)
    end
    if a_inf then return a end
    if b_inf then return b end

    -- Zero handling
    local a_zero = is_zero(a)
    local b_zero = is_zero(b)
    if a_zero and b_zero then
        -- +0 + +0 = +0, -0 + -0 = -0, +0 + -0 = +0
        local result_sign = logic_gates.AND(a.sign, b.sign)
        return make_zero(result_sign, fmt)
    end
    if a_zero then return b end
    if b_zero then return a end

    -- ===================================================================
    -- Step 1: Extract exponents and mantissas as integers
    -- ===================================================================
    --
    -- We work with extended mantissas that include the implicit leading bit.
    -- For normal numbers, this is 1; for denormals, it's 0.
    --
    -- We also add extra guard bits for rounding precision. The guard bits
    -- are: Guard (G), Round (R), and Sticky (S) -- 3 extra bits.

    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)

    -- Add implicit leading 1 for normal numbers (exponent != 0)
    if exp_a ~= 0 then
        mant_a = (1 << fmt.mantissa_bits) | mant_a
    else
        exp_a = 1  -- Denormal true exponent = 1 - bias, stored as 1 for alignment
    end
    if exp_b ~= 0 then
        mant_b = (1 << fmt.mantissa_bits) | mant_b
    else
        exp_b = 1
    end

    -- Add 3 guard bits (shift left by 3) for rounding precision
    local guard_bits = 3
    mant_a = mant_a << guard_bits
    mant_b = mant_b << guard_bits

    -- ===================================================================
    -- Step 2: Align mantissas by shifting the smaller one right
    -- ===================================================================

    local result_exp
    if exp_a >= exp_b then
        local exp_diff = exp_a - exp_b
        if exp_diff > 0 and exp_diff < (fmt.mantissa_bits + 1 + guard_bits) then
            local shifted_out = mant_b & ((1 << exp_diff) - 1)
            local sticky = 0
            if shifted_out ~= 0 then sticky = 1 end
            mant_b = mant_b >> exp_diff
            if sticky ~= 0 and exp_diff > 0 then
                mant_b = mant_b | 1
            end
        elseif exp_diff > 0 then
            local sticky = 0
            if mant_b ~= 0 then sticky = 1 end
            mant_b = mant_b >> exp_diff
            if sticky ~= 0 then
                mant_b = mant_b | 1
            end
        end
        result_exp = exp_a
    else
        local exp_diff = exp_b - exp_a
        if exp_diff > 0 and exp_diff < (fmt.mantissa_bits + 1 + guard_bits) then
            local shifted_out = mant_a & ((1 << exp_diff) - 1)
            local sticky = 0
            if shifted_out ~= 0 then sticky = 1 end
            mant_a = mant_a >> exp_diff
            if sticky ~= 0 and exp_diff > 0 then
                mant_a = mant_a | 1
            end
        elseif exp_diff > 0 then
            local sticky = 0
            if mant_a ~= 0 then sticky = 1 end
            mant_a = mant_a >> exp_diff
            if sticky ~= 0 then
                mant_a = mant_a | 1
            end
        end
        result_exp = exp_b
    end

    -- ===================================================================
    -- Step 3: Add or subtract mantissas based on signs
    -- ===================================================================

    local result_mant, result_sign
    if a.sign == b.sign then
        result_mant = mant_a + mant_b
        result_sign = a.sign
    else
        if mant_a >= mant_b then
            result_mant = mant_a - mant_b
            result_sign = a.sign
        else
            result_mant = mant_b - mant_a
            result_sign = b.sign
        end
    end

    -- ===================================================================
    -- Step 4: Handle zero result
    -- ===================================================================
    if result_mant == 0 then
        return make_zero(0, fmt)  -- +0 by convention
    end

    -- ===================================================================
    -- Step 5: Normalize the result
    -- ===================================================================
    --
    -- The result mantissa should be in the form 1.xxxx (the leading 1 in
    -- position mantissa_bits + guard_bits).

    local normal_pos = fmt.mantissa_bits + guard_bits
    local leading_pos = bit_length(result_mant) - 1

    if leading_pos > normal_pos then
        -- Overflow: shift right to normalize
        local shift_amount = leading_pos - normal_pos
        local lost_bits = result_mant & ((1 << shift_amount) - 1)
        result_mant = result_mant >> shift_amount
        if lost_bits ~= 0 then
            result_mant = result_mant | 1  -- sticky
        end
        result_exp = result_exp + shift_amount
    elseif leading_pos < normal_pos then
        -- Underflow: shift left to normalize
        local shift_amount = normal_pos - leading_pos
        if result_exp - shift_amount >= 1 then
            result_mant = result_mant << shift_amount
            result_exp = result_exp - shift_amount
        else
            -- Can't shift all the way -- result becomes denormal
            local actual_shift = result_exp - 1
            if actual_shift > 0 then
                result_mant = result_mant << actual_shift
            end
            result_exp = 0
        end
    end

    -- ===================================================================
    -- Step 6: Round to nearest even
    -- ===================================================================
    --
    -- Round to nearest even rules:
    --   - If GRS = 0xx: round down (truncate)
    --   - If GRS = 100: round to even (round up if mantissa LSB is 1)
    --   - If GRS = 101, 110, 111: round up

    local guard = (result_mant >> (guard_bits - 1)) & 1
    local round_bit = (result_mant >> (guard_bits - 2)) & 1
    local sticky_bit = result_mant & ((1 << (guard_bits - 2)) - 1)
    if sticky_bit ~= 0 then sticky_bit = 1 end

    -- Remove guard bits
    result_mant = result_mant >> guard_bits

    -- Apply rounding
    if guard == 1 then
        if round_bit == 1 or sticky_bit == 1 then
            result_mant = result_mant + 1  -- Round up
        elseif (result_mant & 1) == 1 then
            result_mant = result_mant + 1  -- Tie-breaking: round to even
        end
    end

    -- Check if rounding caused overflow
    if result_mant >= (1 << (fmt.mantissa_bits + 1)) then
        result_mant = result_mant >> 1
        result_exp = result_exp + 1
    end

    -- ===================================================================
    -- Step 7: Handle exponent overflow/underflow
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
    -- Step 8: Pack the result
    -- ===================================================================
    -- Remove the implicit leading 1 (if normal)
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

--- Subtracts two floating-point numbers: a - b.
---
--- === Why subtraction is trivial once you have addition ===
---
--- In IEEE 754, a - b = a + (-b). To negate b, we just flip its sign bit.
--- This is a single XOR gate in hardware -- the cheapest possible operation.
---
--- @param a table FloatBits operand
--- @param b table FloatBits operand
--- @return table FloatBits result (a - b)
local function fp_sub(a, b)
    local neg_b = FloatBits.new(
        logic_gates.XOR(b.sign, 1),
        b.exponent,
        b.mantissa,
        b.fmt
    )
    return fp_add(a, neg_b)
end

--- Negates a floating-point number: return -a.
---
--- This is the simplest floating-point operation: just flip the sign bit.
--- In hardware, it's literally one NOT gate (or XOR with 1).
---
--- Note: neg(+0) = -0 and neg(-0) = +0. Both are valid IEEE 754 zeros.
---
--- @param a table FloatBits operand
--- @return table FloatBits result (-a)
local function fp_neg(a)
    return FloatBits.new(
        logic_gates.XOR(a.sign, 1),
        a.exponent,
        a.mantissa,
        a.fmt
    )
end

--- Returns the absolute value of a floating-point number.
---
--- Even simpler than negation: just force the sign bit to 0.
--- In hardware, this is done by AND-ing the sign bit with 0.
---
--- Note: abs(NaN) is still NaN (with sign=0). This is the IEEE 754 behavior.
---
--- @param a table FloatBits operand
--- @return table FloatBits result (|a|)
local function fp_abs(a)
    return FloatBits.new(0, a.exponent, a.mantissa, a.fmt)
end

--- Compares two floating-point numbers.
---
--- Returns:
---   -1 if a < b
---    0 if a == b
---    1 if a > b
---
--- NaN comparisons always return 0 (unordered).
---
--- === How FP comparison works in hardware ===
---
--- For two positive normal numbers:
---   - Compare exponents first (larger exponent = larger number)
---   - If exponents equal, compare mantissas
---
--- For mixed signs: positive > negative (always).
--- For two negative numbers: comparison is reversed.
---
--- @param a table FloatBits operand
--- @param b table FloatBits operand
--- @return number -1, 0, or 1
local function fp_compare(a, b)
    -- NaN is unordered
    if is_nan(a) or is_nan(b) then
        return 0
    end

    -- Handle zeros: +0 == -0
    if is_zero(a) and is_zero(b) then
        return 0
    end

    -- Different signs: positive > negative
    if a.sign ~= b.sign then
        if is_zero(a) then
            if b.sign == 1 then return 1 end
            return -1
        end
        if is_zero(b) then
            if a.sign == 1 then return -1 end
            return 1
        end
        if a.sign == 1 then return -1 end
        return 1
    end

    -- Same sign: compare exponent, then mantissa
    local exp_a = bits_msb_to_int(a.exponent)
    local exp_b = bits_msb_to_int(b.exponent)
    local mant_a = bits_msb_to_int(a.mantissa)
    local mant_b = bits_msb_to_int(b.mantissa)

    if exp_a ~= exp_b then
        if a.sign == 0 then
            if exp_a > exp_b then return 1 end
            return -1
        end
        if exp_a > exp_b then return -1 end
        return 1
    end

    if mant_a ~= mant_b then
        if a.sign == 0 then
            if mant_a > mant_b then return 1 end
            return -1
        end
        if mant_a > mant_b then return -1 end
        return 1
    end

    return 0
end

-- =========================================================================
-- Module exports
-- =========================================================================

return {
    fp_add = fp_add,
    fp_sub = fp_sub,
    fp_neg = fp_neg,
    fp_abs = fp_abs,
    fp_compare = fp_compare,
}
