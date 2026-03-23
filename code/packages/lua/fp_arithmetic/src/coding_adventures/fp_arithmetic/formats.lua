-- formats.lua -- IEEE 754 floating-point format definitions and data structures
--
-- === What is a floating-point format? ===
--
-- Floating-point is how computers represent real numbers (like 3.14 or -0.001).
-- It works like scientific notation, but in binary:
--
--   Scientific notation:   -6.022 x 10^23
--   IEEE 754 (binary):     (-1)^sign x 1.mantissa x 2^(exponent - bias)
--
-- A floating-point number is stored as three bit fields packed into a fixed-width
-- binary word:
--
--   FP32 (32 bits):  [sign(1)] [exponent(8)] [mantissa(23)]
--                     ^         ^              ^
--                     |         |              |
--                     |         |              +-- fractional part (after the "1.")
--                     |         +-- power of 2 (biased: stored value - 127)
--                     +-- 0 = positive, 1 = negative
--
-- === The three formats we support ===
--
--   Format  Total  Exp  Mantissa  Bias   Used by
--   ------  -----  ---  --------  ----   -------
--   FP32     32     8     23      127    CPU, GPU (default precision)
--   FP16     16     5     10       15    GPU training (mixed precision)
--   BF16     16     8      7      127    TPU (native), ML training
--
-- === Why BF16 exists ===
--
-- BF16 (Brain Float 16) was invented by Google for TPU hardware. It keeps the
-- same exponent range as FP32 (8-bit exponent, bias 127) but truncates the
-- mantissa from 23 bits to just 7. This means:
--
--   - Same range as FP32 (can represent very large and very small numbers)
--   - Much less precision (~2-3 decimal digits vs ~7 for FP32)
--   - Perfect for ML: gradients can be huge or tiny (need range), but don't
--     need to be super precise (need less precision)
--   - Trivial conversion from FP32: just truncate the lower 16 bits!
--
-- === The implicit leading 1 ===
--
-- For normal (non-zero, non-denormal) numbers, the mantissa has an implicit
-- leading 1 that is not stored. So a stored mantissa of {1, 0, 1, ...} actually
-- represents 1.101... in binary. This trick gives us one extra bit of precision
-- for free.
--
--   Stored bits:   {1, 0, 1, 0, 0, ...}
--   Actual value:  1.10100...  (the "1." is implicit)
--
-- The only exception is denormalized numbers (exponent = all zeros), where the
-- implicit bit is 0 instead of 1, allowing representation of very small numbers
-- near zero.

-- =========================================================================
-- FloatFormat -- describes the shape of a floating-point format
-- =========================================================================

--- FloatFormat describes the bit layout of an IEEE 754 floating-point format.
---
--- Fields:
---   - name: Human-readable name ("fp32", "fp16", "bf16").
---   - total_bits: Total width of the format in bits.
---   - exponent_bits: Number of bits in the exponent field.
---   - mantissa_bits: Number of explicit mantissa bits (without the implicit
---     leading 1). The actual precision is mantissa_bits + 1.
---   - bias: The exponent bias. The true exponent is (stored_exponent - bias).
---     For FP32: bias=127, so stored exponent 127 means true exponent 0,
---     stored exponent 128 means true exponent 1, etc.
local FloatFormat = {}
FloatFormat.__index = FloatFormat

--- Creates a new FloatFormat descriptor.
---
--- @param name string Human-readable name
--- @param total_bits number Total width in bits
--- @param exponent_bits number Number of exponent bits
--- @param mantissa_bits number Number of explicit mantissa bits
--- @param bias number Exponent bias
--- @return table A FloatFormat instance
function FloatFormat.new(name, total_bits, exponent_bits, mantissa_bits, bias)
    local self = setmetatable({}, FloatFormat)
    self.name = name
    self.total_bits = total_bits
    self.exponent_bits = exponent_bits
    self.mantissa_bits = mantissa_bits
    self.bias = bias
    return self
end

--- Equality check for FloatFormat instances.
---
--- Two formats are equal if all their fields match. This is used in format
--- conversion to detect when no conversion is needed.
function FloatFormat.__eq(a, b)
    return a.name == b.name
        and a.total_bits == b.total_bits
        and a.exponent_bits == b.exponent_bits
        and a.mantissa_bits == b.mantissa_bits
        and a.bias == b.bias
end

-- =========================================================================
-- Standard format constants
-- =========================================================================

--- FP32 (single precision) -- the workhorse of computing.
---
---   [sign(1)] [exponent(8)] [mantissa(23)]
---    bit 31    bits 30-23    bits 22-0
---
--- Used by CPU FPUs, GPU CUDA cores, and as the default for most computation.
--- Range: ~1.18e-38 to ~3.40e38, precision: ~7 decimal digits.
local FP32 = FloatFormat.new("fp32", 32, 8, 23, 127)

--- FP16 (half precision) -- GPU mixed-precision training.
---
---   [sign(1)] [exponent(5)] [mantissa(10)]
---    bit 15    bits 14-10    bits 9-0
---
--- Used for GPU training in mixed precision and inference. Saves memory and
--- bandwidth at the cost of range and precision.
--- Range: ~5.96e-8 to ~65504, precision: ~3-4 decimal digits.
local FP16 = FloatFormat.new("fp16", 16, 5, 10, 15)

--- BF16 (brain float) -- Google's TPU native format.
---
---   [sign(1)] [exponent(8)] [mantissa(7)]
---    bit 15    bits 14-7     bits 6-0
---
--- Same exponent range as FP32, but with only 7 mantissa bits (vs 23).
--- Converting FP32 -> BF16 is trivial: just drop the lower 16 bits.
--- Range: same as FP32, precision: ~2-3 decimal digits.
local BF16 = FloatFormat.new("bf16", 16, 8, 7, 127)

-- =========================================================================
-- FloatBits -- the actual bit pattern of a floating-point number
-- =========================================================================

--- FloatBits is the bit-level representation of an IEEE 754 floating-point number.
---
--- This stores the actual 0s and 1s that make up the number, decomposed into
--- the three fields (sign, exponent, mantissa). All bit tables are stored
--- MSB-first (index 1 = most significant bit).
---
--- === Bit layout (FP32 example) ===
---
--- Consider the number 3.14:
---
---   Binary: 1.10010001111010111000011 x 2^1
---   Sign: 0 (positive)
---   Exponent: 128 (= 1 + 127 bias) = {1,0,0,0,0,0,0,0}
---   Mantissa: {1,0,0,1,0,0,0,1,1,1,1,0,1,0,1,1,1,0,0,0,0,1,1}
---
--- Fields:
---   - sign: 0 for positive, 1 for negative.
---   - exponent: Table of exponent bits, MSB first. Length = fmt.exponent_bits.
---   - mantissa: Table of mantissa bits, MSB first. Length = fmt.mantissa_bits.
---     These are the explicit bits only (no implicit leading 1).
---   - fmt: The FloatFormat this number is encoded in.
local FloatBits = {}
FloatBits.__index = FloatBits

--- Creates a new FloatBits instance.
---
--- @param sign number 0 or 1
--- @param exponent table Array of exponent bits (MSB first)
--- @param mantissa table Array of mantissa bits (MSB first)
--- @param fmt table A FloatFormat instance
--- @return table A FloatBits instance
function FloatBits.new(sign, exponent, mantissa, fmt)
    local self = setmetatable({}, FloatBits)
    self.sign = sign
    self.exponent = exponent
    self.mantissa = mantissa
    self.fmt = fmt
    return self
end

-- =========================================================================
-- Utility functions for bit tables
-- =========================================================================

--- Creates a table of n zeros.
---
--- @param n number Length of the table
--- @return table Array of zeros
local function zeros_table(n)
    local t = {}
    for i = 1, n do
        t[i] = 0
    end
    return t
end

--- Creates a table of n ones.
---
--- @param n number Length of the table
--- @return table Array of ones
local function ones_table(n)
    local t = {}
    for i = 1, n do
        t[i] = 1
    end
    return t
end

-- =========================================================================
-- Helper constructors for common special values
-- =========================================================================

--- Creates a quiet NaN in the given format.
---
--- NaN (Not a Number) is represented by exponent = all 1s and mantissa != 0.
--- The MSB of the mantissa being 1 makes it a "quiet" NaN (as opposed to a
--- "signaling" NaN with MSB 0).
---
--- @param fmt table A FloatFormat instance
--- @return table A FloatBits representing NaN
local function make_nan(fmt)
    local mant = zeros_table(fmt.mantissa_bits)
    mant[1] = 1  -- MSB set = quiet NaN
    return FloatBits.new(0, ones_table(fmt.exponent_bits), mant, fmt)
end

--- Creates positive or negative infinity in the given format.
---
--- Infinity is represented by exponent = all 1s and mantissa = all 0s.
---
--- @param sign number 0 for +Inf, 1 for -Inf
--- @param fmt table A FloatFormat instance
--- @return table A FloatBits representing infinity
local function make_inf(sign, fmt)
    return FloatBits.new(sign, ones_table(fmt.exponent_bits), zeros_table(fmt.mantissa_bits), fmt)
end

--- Creates positive or negative zero in the given format.
---
--- Zero is represented by exponent = all 0s and mantissa = all 0s.
--- IEEE 754 has both +0 and -0 -- they compare equal but have different bits.
---
--- @param sign number 0 for +0, 1 for -0
--- @param fmt table A FloatFormat instance
--- @return table A FloatBits representing zero
local function make_zero(sign, fmt)
    return FloatBits.new(sign, zeros_table(fmt.exponent_bits), zeros_table(fmt.mantissa_bits), fmt)
end

-- =========================================================================
-- Helper: integer <-> bit table conversions
-- =========================================================================

--- Converts a non-negative integer to a table of bits, MSB first.
---
--- This is the fundamental conversion between Lua integers and our bit-level
--- representation.
---
--- Example:
---   int_to_bits_msb(5, 8) => {0, 0, 0, 0, 0, 1, 0, 1}
---                            128 64 32 16  8  4  2  1
---                                              4     1  = 5
---
--- How it works: we check each bit position from MSB to LSB. For each
--- position i (counting from width-1 down to 0), we check if that bit is
--- set using a right-shift and AND with 1.
---
--- @param value number Non-negative integer
--- @param width number Number of bits in the output
--- @return table Array of bits, MSB first
local function int_to_bits_msb(value, width)
    local bits = {}
    for i = 1, width do
        bits[i] = (value >> (width - i)) & 1
    end
    return bits
end

--- Converts a table of bits (MSB first) back to a non-negative integer.
---
--- This is the inverse of int_to_bits_msb.
---
--- Example:
---   bits_msb_to_int({0, 0, 0, 0, 0, 1, 0, 1}) => 5
---   Each bit contributes: bit_value * 2^position
---   0*128 + 0*64 + 0*32 + 0*16 + 0*8 + 1*4 + 0*2 + 1*1 = 5
---
--- @param bits table Array of bits, MSB first
--- @return number The integer value
local function bits_msb_to_int(bits)
    local result = 0
    for _, bit in ipairs(bits) do
        result = (result << 1) | bit
    end
    return result
end

--- Returns the position of the highest set bit + 1, like Python's int.bit_length().
---
--- For example: bit_length(5) = 3, bit_length(1) = 1, bit_length(0) = 0.
--- This is essential for normalization: we need to know where the leading 1 is.
---
--- @param v number Non-negative integer
--- @return number Bit length
local function bit_length(v)
    if v == 0 then return 0 end
    local n = 0
    while v > 0 do
        n = n + 1
        v = v >> 1
    end
    return n
end

-- =========================================================================
-- Module exports
-- =========================================================================

return {
    -- Types
    FloatFormat = FloatFormat,
    FloatBits = FloatBits,

    -- Format constants
    FP32 = FP32,
    FP16 = FP16,
    BF16 = BF16,

    -- Special value constructors
    make_nan = make_nan,
    make_inf = make_inf,
    make_zero = make_zero,

    -- Bit utilities
    int_to_bits_msb = int_to_bits_msb,
    bits_msb_to_int = bits_msb_to_int,
    bit_length = bit_length,
    zeros_table = zeros_table,
    ones_table = ones_table,
}
