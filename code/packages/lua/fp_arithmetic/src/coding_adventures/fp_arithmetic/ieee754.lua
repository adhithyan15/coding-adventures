-- ieee754.lua -- IEEE 754 encoding and decoding
--
-- Converting between Lua float64 values and our explicit bit-level
-- representation (FloatBits).
--
-- === How does a computer store 3.14? ===
--
-- When you write `x = 3.14` in Lua, the computer stores it as 64 bits
-- following the IEEE 754 standard. This module converts between Lua's native
-- float representation and our explicit bit-level representation (FloatBits).
--
-- === Encoding: float -> bits ===
--
-- For FP32, we use Lua 5.4's string.pack("f", value) to get the exact same
-- bit pattern the hardware uses. We then extract sign, exponent, and mantissa
-- from the raw 32-bit integer. For FP16 and BF16, we manually extract the
-- bits because Lua doesn't natively support these formats.
--
-- === Special values in IEEE 754 ===
--
-- IEEE 754 reserves certain bit patterns for special values:
--
--   Exponent      Mantissa    Meaning
--   ----------    --------    -------
--   All 1s        All 0s      +/- Infinity
--   All 1s        Non-zero    NaN (Not a Number)
--   All 0s        All 0s      +/- Zero
--   All 0s        Non-zero    Denormalized number (very small, near zero)
--   Other         Any         Normal number

local formats = require("coding_adventures.fp_arithmetic.formats")
local logic_gates = require("coding_adventures.logic_gates")

local FloatBits = formats.FloatBits
local FP32 = formats.FP32
local int_to_bits_msb = formats.int_to_bits_msb
local bits_msb_to_int = formats.bits_msb_to_int
local make_nan = formats.make_nan
local make_inf = formats.make_inf
local make_zero = formats.make_zero
local zeros_table = formats.zeros_table

-- =========================================================================
-- Encoding: Lua float64 -> FloatBits
-- =========================================================================

--- Converts a Lua number to its IEEE 754 bit representation.
---
--- === How FP32 encoding works ===
---
--- For FP32, we use string.pack("f", value) which gives us the raw 4 bytes
--- of the IEEE 754 single-precision representation. We then unpack as an
--- unsigned 32-bit integer and extract the three fields using bit shifts and
--- masks.
---
--- === How FP16/BF16 encoding works (manual) ===
---
--- For FP16 and BF16, Lua doesn't have native support, so we:
---  1. First encode as FP32 (which we know is exact for the hardware)
---  2. Extract the sign, exponent, and mantissa from the FP32 encoding
---  3. Re-encode into the target format, adjusting exponent bias and
---     truncating the mantissa
---
--- === Worked example: encoding 3.14 as FP32 ===
---
---   3.14 in binary: 11.00100011110101110000101...
---   Normalized:     1.100100011110101110000101... x 2^1
---
---   Sign:     0 (positive)
---   Exponent: 1 + 127 (bias) = 128 = 10000000 in binary
---   Mantissa: 10010001111010111000010 (23 bits after the implicit 1)
---                                      ^-- note: the leading 1 is NOT stored
---
--- @param value number Lua number (float64)
--- @param fmt table A FloatFormat instance
--- @return table A FloatBits instance
local function float_to_bits(value, fmt)
    -- --- Handle NaN specially ---
    -- Lua's math has NaN, and IEEE 754 defines NaN as exponent=all-1s,
    -- mantissa=non-zero. We use a "quiet NaN" with the MSB of mantissa set.
    if value ~= value then  -- NaN is the only value not equal to itself
        return make_nan(fmt)
    end

    -- --- Handle Infinity ---
    -- +Inf and -Inf: exponent=all-1s, mantissa=all-0s.
    if value == math.huge then
        return make_inf(0, fmt)
    end
    if value == -math.huge then
        return make_inf(1, fmt)
    end

    -- --- FP32: use string.pack for hardware-exact encoding ---
    if fmt == FP32 then
        -- string.pack("f", v) gives us the raw IEEE 754 FP32 bytes.
        -- string.unpack("I4", ...) interprets those bytes as a 32-bit unsigned int.
        local raw_bytes = string.pack("f", value)
        local int_bits = string.unpack("I4", raw_bytes)

        -- Extract the three fields using bit shifts and masks:
        --   Bit 31:     sign
        --   Bits 30-23: exponent (8 bits)
        --   Bits 22-0:  mantissa (23 bits)
        local sign = (int_bits >> 31) & 1
        local exp_int = (int_bits >> 23) & 0xFF
        local mant_int = int_bits & 0x7FFFFF

        return FloatBits.new(
            sign,
            int_to_bits_msb(exp_int, 8),
            int_to_bits_msb(mant_int, 23),
            FP32
        )
    end

    -- --- FP16 and BF16: manual conversion from FP32 ---
    --
    -- Strategy: encode as FP32 first, then convert.
    -- This handles all the tricky cases (denormals, rounding) correctly.
    local fp32_bits = float_to_bits(value, FP32)
    local fp32_exp = bits_msb_to_int(fp32_bits.exponent)
    local fp32_mant = bits_msb_to_int(fp32_bits.mantissa)
    local sign = fp32_bits.sign

    -- --- Handle zero ---
    if fp32_exp == 0 and fp32_mant == 0 then
        return make_zero(sign, fmt)
    end

    -- --- Compute the true (unbiased) exponent ---
    local true_exp, full_mantissa
    if fp32_exp == 0 then
        -- Denormal in FP32: true exponent is -126, implicit bit is 0
        true_exp = 1 - FP32.bias  -- = -126
        full_mantissa = fp32_mant
    else
        true_exp = fp32_exp - FP32.bias
        -- Normal: full mantissa includes the implicit leading 1
        full_mantissa = (1 << FP32.mantissa_bits) | fp32_mant
    end

    -- --- Map to target format ---
    local target_exp = true_exp + fmt.bias
    local max_exp = (1 << fmt.exponent_bits) - 1

    -- --- Overflow: exponent too large for target format -> Infinity ---
    if target_exp >= max_exp then
        return make_inf(sign, fmt)
    end

    -- --- Normal case: exponent fits in target format ---
    if target_exp > 0 then
        local truncated
        if fmt.mantissa_bits < FP32.mantissa_bits then
            local shift = FP32.mantissa_bits - fmt.mantissa_bits
            truncated = fp32_mant >> shift
            -- Round-to-nearest-even
            local round_bit = (fp32_mant >> (shift - 1)) & 1
            local sticky = fp32_mant & ((1 << (shift - 1)) - 1)
            if round_bit ~= 0 and (sticky ~= 0 or (truncated & 1) ~= 0) then
                truncated = truncated + 1
                -- Rounding overflow
                if truncated >= (1 << fmt.mantissa_bits) then
                    truncated = 0
                    target_exp = target_exp + 1
                    if target_exp >= max_exp then
                        return make_inf(sign, fmt)
                    end
                end
            end
        else
            truncated = fp32_mant << (fmt.mantissa_bits - FP32.mantissa_bits)
        end

        return FloatBits.new(
            sign,
            int_to_bits_msb(target_exp, fmt.exponent_bits),
            int_to_bits_msb(truncated, fmt.mantissa_bits),
            fmt
        )
    end

    -- --- Underflow: number is too small for normal representation ---
    -- It might still be representable as a denormal in the target format.
    local denorm_shift = 1 - target_exp

    if denorm_shift > fmt.mantissa_bits then
        -- Too small even for denormal -> flush to zero
        return make_zero(sign, fmt)
    end

    -- Shift the full mantissa right to create a denormal
    local denorm_mant = full_mantissa >> (denorm_shift + FP32.mantissa_bits - fmt.mantissa_bits)

    return FloatBits.new(
        sign,
        zeros_table(fmt.exponent_bits),
        int_to_bits_msb(denorm_mant & ((1 << fmt.mantissa_bits) - 1), fmt.mantissa_bits),
        fmt
    )
end

-- =========================================================================
-- Decoding: FloatBits -> Lua float64
-- =========================================================================

--- Converts an IEEE 754 bit representation back to a Lua number.
---
--- === How decoding works ===
---
--- For FP32, we reconstruct the 32-bit integer and use string.unpack to get
--- the exact Lua float. For FP16/BF16, we manually compute the value using:
---
---   value = (-1)^sign x 2^(exponent - bias) x 1.mantissa
---
--- @param bits table A FloatBits instance
--- @return number Lua float64 value
local function bits_to_float(bits)
    local exp_int = bits_msb_to_int(bits.exponent)
    local mant_int = bits_msb_to_int(bits.mantissa)
    local max_exp = (1 << bits.fmt.exponent_bits) - 1

    -- --- Special values ---

    -- NaN: exponent all 1s, mantissa non-zero
    if exp_int == max_exp and mant_int ~= 0 then
        return 0/0  -- Lua NaN
    end

    -- Infinity: exponent all 1s, mantissa all zeros
    if exp_int == max_exp and mant_int == 0 then
        if bits.sign == 1 then
            return -math.huge
        end
        return math.huge
    end

    -- Zero: exponent all 0s, mantissa all zeros
    if exp_int == 0 and mant_int == 0 then
        if bits.sign == 1 then
            -- Lua doesn't have a literal -0, but we can create one via division
            return -0.0
        end
        return 0.0
    end

    -- --- For FP32, use string.pack/unpack for exact conversion ---
    if bits.fmt == FP32 then
        local int_bits = (bits.sign << 31) | (exp_int << 23) | mant_int
        local raw_bytes = string.pack("I4", int_bits)
        return string.unpack("f", raw_bytes)
    end

    -- --- For FP16/BF16, compute the float value manually ---
    local true_exp, mantissa_value

    if exp_int == 0 then
        -- Denormalized: value = (-1)^sign x 2^(1-bias) x 0.mantissa
        true_exp = 1 - bits.fmt.bias
        mantissa_value = mant_int / (1 << bits.fmt.mantissa_bits)
    else
        -- Normal: implicit leading 1
        true_exp = exp_int - bits.fmt.bias
        mantissa_value = 1.0 + mant_int / (1 << bits.fmt.mantissa_bits)
    end

    local result = mantissa_value * (2.0 ^ true_exp)
    if bits.sign == 1 then
        result = -result
    end

    return result
end

-- =========================================================================
-- Special value detection -- using logic gates
-- =========================================================================
--
-- These functions detect special IEEE 754 values by examining the bit pattern.
-- We use AND and OR from logic_gates to check bit fields, staying true to the
-- "built from gates" philosophy.

--- Checks if all bits in a table are 1, using AND gates.
---
--- In hardware, this would be a wide AND gate:
---
---   all_ones = AND(bit[1], AND(bit[2], AND(bit[3], ...)))
---
--- If ALL bits are 1, the final AND output is 1. If ANY bit is 0, it collapses to 0.
---
--- @param bits table Array of bits
--- @return boolean True if all bits are 1
local function all_ones(bits)
    local result = bits[1]
    for i = 2, #bits do
        result = logic_gates.AND(result, bits[i])
    end
    return result == 1
end

--- Checks if all bits in a table are 0, using OR gates then NOT.
---
--- In hardware: NOR across all bits.
---
---   any_one = OR(bit[1], OR(bit[2], OR(bit[3], ...)))
---   all_zeros = NOT(any_one)
---
--- If ANY bit is 1, the OR chain produces 1, and we return false.
--- If ALL bits are 0, the OR chain produces 0, and we return true.
---
--- @param bits table Array of bits
--- @return boolean True if all bits are 0
local function all_zeros(bits)
    local result = bits[1]
    for i = 2, #bits do
        result = logic_gates.OR(result, bits[i])
    end
    return result == 0
end

--- Checks if a FloatBits represents NaN (Not a Number).
---
--- NaN is defined as: exponent = all 1s AND mantissa != all 0s.
---
--- In IEEE 754, NaN is the result of undefined operations like:
---
---   0 / 0, Inf - Inf, sqrt(-1)
---
--- @param bits table A FloatBits instance
--- @return boolean True if bits represents NaN
local function is_nan(bits)
    return all_ones(bits.exponent) and not all_zeros(bits.mantissa)
end

--- Checks if a FloatBits represents Infinity (+Inf or -Inf).
---
--- Infinity is defined as: exponent = all 1s AND mantissa = all 0s.
---
--- @param bits table A FloatBits instance
--- @return boolean True if bits represents infinity
local function is_inf(bits)
    return all_ones(bits.exponent) and all_zeros(bits.mantissa)
end

--- Checks if a FloatBits represents zero (+0 or -0).
---
--- Zero is defined as: exponent = all 0s AND mantissa = all 0s.
---
--- @param bits table A FloatBits instance
--- @return boolean True if bits represents zero
local function is_zero(bits)
    return all_zeros(bits.exponent) and all_zeros(bits.mantissa)
end

--- Checks if a FloatBits represents a denormalized (subnormal) number.
---
--- Denormalized is defined as: exponent = all 0s AND mantissa != all 0s.
---
--- === What are denormalized numbers? ===
---
--- Normal IEEE 754 numbers have an implicit leading 1: the value is 1.mantissa.
--- But what about very small numbers close to zero? The smallest normal FP32
--- number is about 1.18e-38. Without denormals, the next smaller value would
--- be 0 -- a sudden jump called "the underflow gap."
---
--- Denormalized numbers fill this gap. When the exponent is all zeros, the
--- implicit bit becomes 0 instead of 1, and the true exponent is fixed at
--- (1 - bias).
---
--- @param bits table A FloatBits instance
--- @return boolean True if bits represents a denormalized number
local function is_denormalized(bits)
    return all_zeros(bits.exponent) and not all_zeros(bits.mantissa)
end

-- =========================================================================
-- Module exports
-- =========================================================================

return {
    float_to_bits = float_to_bits,
    bits_to_float = bits_to_float,
    is_nan = is_nan,
    is_inf = is_inf,
    is_zero = is_zero,
    is_denormalized = is_denormalized,
    all_ones = all_ones,
    all_zeros = all_zeros,
}
