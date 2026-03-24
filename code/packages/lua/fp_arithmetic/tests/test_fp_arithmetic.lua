-- Tests for fp-arithmetic -- comprehensive busted test suite.
--
-- These tests cover all modules: formats, ieee754 encoding/decoding,
-- special value detection, arithmetic (add/sub/mul/neg/abs/compare),
-- FMA, format conversion, and pipelined units.

-- Add dependency paths so we can require the package and its dependencies.
package.path = "../src/?.lua;" .. "../src/?/init.lua;"
    .. "../../logic_gates/src/?.lua;" .. "../../logic_gates/src/?/init.lua;"
    .. "../../clock/src/?.lua;" .. "../../clock/src/?/init.lua;"
    .. package.path

local fp = require("coding_adventures.fp_arithmetic")
local clock_mod = require("coding_adventures.clock")

-- =========================================================================
-- Helper: round-trip test (encode then decode, check result)
-- =========================================================================

--- Encodes a Lua number into FloatBits and decodes it back, then asserts
--- that the decoded value matches the original within the format's precision.
local function round_trip(value, fmt, tolerance)
    tolerance = tolerance or 0.0001
    local bits = fp.float_to_bits(value, fmt)
    local result = fp.bits_to_float(bits)
    if value ~= value then
        -- NaN: check that result is also NaN
        assert.is_true(result ~= result, "expected NaN but got " .. tostring(result))
    elseif value == math.huge or value == -math.huge then
        assert.are.equal(value, result)
    elseif value == 0 then
        assert.are.equal(0, result)
    else
        local rel_err = math.abs((result - value) / value)
        assert.is_true(rel_err <= tolerance,
            string.format("round_trip(%g, %s): got %g, rel_err=%g > %g",
                value, fmt.name, result, rel_err, tolerance))
    end
    return bits
end

--- Helper to compute FP result and compare against expected Lua value.
local function check_fp_op(op_func, a_val, b_val, expected, fmt, tolerance)
    fmt = fmt or fp.FP32
    tolerance = tolerance or 0.001
    local a = fp.float_to_bits(a_val, fmt)
    local b = fp.float_to_bits(b_val, fmt)
    local result_bits = op_func(a, b)
    local result = fp.bits_to_float(result_bits)

    if expected ~= expected then
        assert.is_true(result ~= result, "expected NaN")
    elseif expected == math.huge or expected == -math.huge then
        assert.are.equal(expected, result)
    elseif expected == 0 then
        assert.are.equal(0, math.abs(result))
    else
        local rel_err = math.abs((result - expected) / expected)
        assert.is_true(rel_err < tolerance,
            string.format("op(%g, %g): expected %g, got %g, rel_err=%g",
                a_val, b_val, expected, result, rel_err))
    end
end

-- =========================================================================
-- Tests begin
-- =========================================================================

describe("fp-arithmetic", function()

    -- =====================================================================
    -- Module version
    -- =====================================================================
    it("has a version", function()
        assert.are.equal("0.1.0", fp.VERSION)
    end)

    -- =====================================================================
    -- FloatFormat
    -- =====================================================================
    describe("FloatFormat", function()
        it("creates FP32 format with correct fields", function()
            assert.are.equal("fp32", fp.FP32.name)
            assert.are.equal(32, fp.FP32.total_bits)
            assert.are.equal(8, fp.FP32.exponent_bits)
            assert.are.equal(23, fp.FP32.mantissa_bits)
            assert.are.equal(127, fp.FP32.bias)
        end)

        it("creates FP16 format with correct fields", function()
            assert.are.equal("fp16", fp.FP16.name)
            assert.are.equal(16, fp.FP16.total_bits)
            assert.are.equal(5, fp.FP16.exponent_bits)
            assert.are.equal(10, fp.FP16.mantissa_bits)
            assert.are.equal(15, fp.FP16.bias)
        end)

        it("creates BF16 format with correct fields", function()
            assert.are.equal("bf16", fp.BF16.name)
            assert.are.equal(16, fp.BF16.total_bits)
            assert.are.equal(8, fp.BF16.exponent_bits)
            assert.are.equal(7, fp.BF16.mantissa_bits)
            assert.are.equal(127, fp.BF16.bias)
        end)

        it("compares formats for equality", function()
            assert.is_true(fp.FP32 == fp.FP32)
            assert.is_false(fp.FP32 == fp.FP16)
            assert.is_false(fp.FP16 == fp.BF16)
        end)
    end)

    -- =====================================================================
    -- Bit utilities
    -- =====================================================================
    describe("bit utilities", function()
        it("converts integer to bits MSB first", function()
            local bits = fp.int_to_bits_msb(5, 8)
            assert.are.same({0, 0, 0, 0, 0, 1, 0, 1}, bits)
        end)

        it("converts zero to all-zero bits", function()
            local bits = fp.int_to_bits_msb(0, 4)
            assert.are.same({0, 0, 0, 0}, bits)
        end)

        it("converts bits back to integer", function()
            assert.are.equal(5, fp.bits_msb_to_int({0, 0, 0, 0, 0, 1, 0, 1}))
            assert.are.equal(0, fp.bits_msb_to_int({0, 0, 0, 0}))
            assert.are.equal(255, fp.bits_msb_to_int({1, 1, 1, 1, 1, 1, 1, 1}))
        end)

        it("round-trips integer through bits", function()
            for _, v in ipairs({0, 1, 127, 255, 1023, 8388607}) do
                local width = math.max(1, fp.bit_length(v))
                assert.are.equal(v, fp.bits_msb_to_int(fp.int_to_bits_msb(v, width)))
            end
        end)

        it("computes bit_length correctly", function()
            assert.are.equal(0, fp.bit_length(0))
            assert.are.equal(1, fp.bit_length(1))
            assert.are.equal(3, fp.bit_length(5))
            assert.are.equal(8, fp.bit_length(255))
            assert.are.equal(24, fp.bit_length(8388608))
        end)
    end)

    -- =====================================================================
    -- IEEE 754 encoding/decoding (FP32)
    -- =====================================================================
    describe("FP32 encoding/decoding", function()
        it("round-trips positive integers", function()
            for _, v in ipairs({1.0, 2.0, 3.0, 42.0, 1000.0}) do
                round_trip(v, fp.FP32, 0)
            end
        end)

        it("round-trips negative numbers", function()
            for _, v in ipairs({-1.0, -2.5, -100.0}) do
                round_trip(v, fp.FP32, 0)
            end
        end)

        it("round-trips fractional numbers", function()
            for _, v in ipairs({0.5, 0.25, 0.125, 1.5, 3.14}) do
                round_trip(v, fp.FP32, 1e-6)
            end
        end)

        it("round-trips powers of 2", function()
            for e = -10, 10 do
                round_trip(2.0 ^ e, fp.FP32, 0)
            end
        end)

        it("encodes +0 correctly", function()
            local bits = fp.float_to_bits(0.0, fp.FP32)
            assert.are.equal(0, bits.sign)
            assert.is_true(fp.is_zero(bits))
            assert.are.equal(0.0, fp.bits_to_float(bits))
        end)

        it("encodes -0 correctly", function()
            local bits = fp.float_to_bits(-0.0, fp.FP32)
            assert.are.equal(1, bits.sign)
            assert.is_true(fp.is_zero(bits))
        end)

        it("encodes +Inf correctly", function()
            local bits = fp.float_to_bits(math.huge, fp.FP32)
            assert.are.equal(0, bits.sign)
            assert.is_true(fp.is_inf(bits))
            assert.are.equal(math.huge, fp.bits_to_float(bits))
        end)

        it("encodes -Inf correctly", function()
            local bits = fp.float_to_bits(-math.huge, fp.FP32)
            assert.are.equal(1, bits.sign)
            assert.is_true(fp.is_inf(bits))
            assert.are.equal(-math.huge, fp.bits_to_float(bits))
        end)

        it("encodes NaN correctly", function()
            local bits = fp.float_to_bits(0/0, fp.FP32)
            assert.is_true(fp.is_nan(bits))
            local decoded = fp.bits_to_float(bits)
            assert.is_true(decoded ~= decoded)  -- NaN != NaN
        end)

        it("encodes 1.0 with exponent 127", function()
            local bits = fp.float_to_bits(1.0, fp.FP32)
            assert.are.equal(0, bits.sign)
            assert.are.equal(127, fp.bits_msb_to_int(bits.exponent))
            assert.are.equal(0, fp.bits_msb_to_int(bits.mantissa))
        end)

        it("encodes 2.0 with exponent 128", function()
            local bits = fp.float_to_bits(2.0, fp.FP32)
            assert.are.equal(128, fp.bits_msb_to_int(bits.exponent))
            assert.are.equal(0, fp.bits_msb_to_int(bits.mantissa))
        end)

        it("encodes 1.5 correctly", function()
            local bits = fp.float_to_bits(1.5, fp.FP32)
            assert.are.equal(0, bits.sign)
            assert.are.equal(127, fp.bits_msb_to_int(bits.exponent))
            -- 1.5 = 1.1 binary, mantissa = 100...0 = 2^22
            assert.are.equal(1 << 22, fp.bits_msb_to_int(bits.mantissa))
        end)
    end)

    -- =====================================================================
    -- IEEE 754 encoding/decoding (FP16)
    -- =====================================================================
    describe("FP16 encoding/decoding", function()
        it("round-trips simple values", function()
            for _, v in ipairs({1.0, 2.0, 0.5, -1.0, 0.25}) do
                round_trip(v, fp.FP16, 0)
            end
        end)

        it("round-trips fractional values with FP16 tolerance", function()
            round_trip(3.14, fp.FP16, 0.002)
            round_trip(0.1, fp.FP16, 0.01)
        end)

        it("handles FP16 overflow to infinity", function()
            -- FP16 max is ~65504, larger values overflow
            local bits = fp.float_to_bits(100000.0, fp.FP16)
            assert.is_true(fp.is_inf(bits))
        end)

        it("encodes special values in FP16", function()
            assert.is_true(fp.is_nan(fp.float_to_bits(0/0, fp.FP16)))
            assert.is_true(fp.is_inf(fp.float_to_bits(math.huge, fp.FP16)))
            assert.is_true(fp.is_inf(fp.float_to_bits(-math.huge, fp.FP16)))
            assert.is_true(fp.is_zero(fp.float_to_bits(0.0, fp.FP16)))
        end)
    end)

    -- =====================================================================
    -- IEEE 754 encoding/decoding (BF16)
    -- =====================================================================
    describe("BF16 encoding/decoding", function()
        it("round-trips simple values", function()
            for _, v in ipairs({1.0, 2.0, 0.5, -1.0, 4.0}) do
                round_trip(v, fp.BF16, 0)
            end
        end)

        it("round-trips with BF16 precision tolerance", function()
            round_trip(3.14, fp.BF16, 0.02)
            round_trip(100.0, fp.BF16, 0.01)
        end)

        it("handles large numbers (BF16 has FP32 range)", function()
            round_trip(1e30, fp.BF16, 0.02)
        end)

        it("encodes special values in BF16", function()
            assert.is_true(fp.is_nan(fp.float_to_bits(0/0, fp.BF16)))
            assert.is_true(fp.is_inf(fp.float_to_bits(math.huge, fp.BF16)))
            assert.is_true(fp.is_zero(fp.float_to_bits(0.0, fp.BF16)))
        end)
    end)

    -- =====================================================================
    -- Special value detection
    -- =====================================================================
    describe("special value detection", function()
        it("detects NaN", function()
            local nan = fp.make_nan(fp.FP32)
            assert.is_true(fp.is_nan(nan))
            assert.is_false(fp.is_inf(nan))
            assert.is_false(fp.is_zero(nan))
            assert.is_false(fp.is_denormalized(nan))
        end)

        it("detects +Inf", function()
            local inf = fp.make_inf(0, fp.FP32)
            assert.is_true(fp.is_inf(inf))
            assert.is_false(fp.is_nan(inf))
            assert.is_false(fp.is_zero(inf))
        end)

        it("detects -Inf", function()
            local neg_inf = fp.make_inf(1, fp.FP32)
            assert.is_true(fp.is_inf(neg_inf))
            assert.are.equal(1, neg_inf.sign)
        end)

        it("detects +0", function()
            local zero = fp.make_zero(0, fp.FP32)
            assert.is_true(fp.is_zero(zero))
            assert.is_false(fp.is_nan(zero))
            assert.is_false(fp.is_inf(zero))
        end)

        it("detects -0", function()
            local neg_zero = fp.make_zero(1, fp.FP32)
            assert.is_true(fp.is_zero(neg_zero))
            assert.are.equal(1, neg_zero.sign)
        end)

        it("detects denormalized numbers", function()
            -- Create a denormalized number: exponent=0, mantissa!=0
            local denorm = fp.FloatBits.new(0,
                fp.int_to_bits_msb(0, 8),
                fp.int_to_bits_msb(1, 23),
                fp.FP32)
            assert.is_true(fp.is_denormalized(denorm))
            assert.is_false(fp.is_zero(denorm))
            assert.is_false(fp.is_nan(denorm))
        end)

        it("normal numbers are not special", function()
            local one = fp.float_to_bits(1.0, fp.FP32)
            assert.is_false(fp.is_nan(one))
            assert.is_false(fp.is_inf(one))
            assert.is_false(fp.is_zero(one))
            assert.is_false(fp.is_denormalized(one))
        end)

        it("detects specials in FP16", function()
            assert.is_true(fp.is_nan(fp.make_nan(fp.FP16)))
            assert.is_true(fp.is_inf(fp.make_inf(0, fp.FP16)))
            assert.is_true(fp.is_zero(fp.make_zero(0, fp.FP16)))
        end)

        it("detects specials in BF16", function()
            assert.is_true(fp.is_nan(fp.make_nan(fp.BF16)))
            assert.is_true(fp.is_inf(fp.make_inf(1, fp.BF16)))
            assert.is_true(fp.is_zero(fp.make_zero(1, fp.BF16)))
        end)
    end)

    -- =====================================================================
    -- FP Addition
    -- =====================================================================
    describe("fp_add", function()
        it("adds simple positive numbers", function()
            check_fp_op(fp.fp_add, 1.0, 2.0, 3.0)
            check_fp_op(fp.fp_add, 1.5, 0.25, 1.75)
            check_fp_op(fp.fp_add, 100.0, 200.0, 300.0)
        end)

        it("adds negative numbers", function()
            check_fp_op(fp.fp_add, -1.0, -2.0, -3.0)
            check_fp_op(fp.fp_add, -5.0, 3.0, -2.0)
            check_fp_op(fp.fp_add, 5.0, -3.0, 2.0)
        end)

        it("adds numbers with large exponent difference", function()
            check_fp_op(fp.fp_add, 1.0, 0.00001, 1.00001, fp.FP32, 0.001)
        end)

        it("cancellation: a + (-a) = 0", function()
            local a = fp.float_to_bits(3.14, fp.FP32)
            local neg_a = fp.fp_neg(a)
            local result = fp.fp_add(a, neg_a)
            assert.is_true(fp.is_zero(result))
        end)

        it("NaN + anything = NaN", function()
            local nan = fp.make_nan(fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            assert.is_true(fp.is_nan(fp.fp_add(nan, one)))
            assert.is_true(fp.is_nan(fp.fp_add(one, nan)))
            assert.is_true(fp.is_nan(fp.fp_add(nan, nan)))
        end)

        it("Inf + finite = Inf", function()
            local inf = fp.make_inf(0, fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            local result = fp.fp_add(inf, one)
            assert.is_true(fp.is_inf(result))
            assert.are.equal(0, result.sign)
        end)

        it("Inf + (-Inf) = NaN", function()
            local pos_inf = fp.make_inf(0, fp.FP32)
            local neg_inf = fp.make_inf(1, fp.FP32)
            assert.is_true(fp.is_nan(fp.fp_add(pos_inf, neg_inf)))
        end)

        it("Inf + Inf = Inf", function()
            local inf = fp.make_inf(0, fp.FP32)
            local result = fp.fp_add(inf, inf)
            assert.is_true(fp.is_inf(result))
            assert.are.equal(0, result.sign)
        end)

        it("0 + x = x", function()
            local zero = fp.make_zero(0, fp.FP32)
            local val = fp.float_to_bits(42.0, fp.FP32)
            local result = fp.bits_to_float(fp.fp_add(zero, val))
            assert.are.equal(42.0, result)
        end)

        it("+0 + -0 = +0", function()
            local pz = fp.make_zero(0, fp.FP32)
            local nz = fp.make_zero(1, fp.FP32)
            local result = fp.fp_add(pz, nz)
            assert.is_true(fp.is_zero(result))
            assert.are.equal(0, result.sign)
        end)

        it("-0 + -0 = -0", function()
            local nz = fp.make_zero(1, fp.FP32)
            local result = fp.fp_add(nz, nz)
            assert.is_true(fp.is_zero(result))
            assert.are.equal(1, result.sign)
        end)

        it("works in FP16", function()
            check_fp_op(fp.fp_add, 1.0, 2.0, 3.0, fp.FP16)
            check_fp_op(fp.fp_add, 1.5, 0.5, 2.0, fp.FP16)
        end)

        it("works in BF16", function()
            check_fp_op(fp.fp_add, 1.0, 2.0, 3.0, fp.BF16)
            check_fp_op(fp.fp_add, 10.0, 20.0, 30.0, fp.BF16, 0.02)
        end)
    end)

    -- =====================================================================
    -- FP Subtraction
    -- =====================================================================
    describe("fp_sub", function()
        it("subtracts simple numbers", function()
            check_fp_op(fp.fp_sub, 5.0, 3.0, 2.0)
            check_fp_op(fp.fp_sub, 1.0, 1.0, 0.0)
            check_fp_op(fp.fp_sub, 3.0, 5.0, -2.0)
        end)

        it("handles negative operands", function()
            check_fp_op(fp.fp_sub, -3.0, -5.0, 2.0)
            check_fp_op(fp.fp_sub, -3.0, 5.0, -8.0)
        end)

        it("Inf - Inf = NaN", function()
            local inf = fp.make_inf(0, fp.FP32)
            assert.is_true(fp.is_nan(fp.fp_sub(inf, inf)))
        end)
    end)

    -- =====================================================================
    -- FP Negation
    -- =====================================================================
    describe("fp_neg", function()
        it("negates positive to negative", function()
            local pos = fp.float_to_bits(3.14, fp.FP32)
            local neg = fp.fp_neg(pos)
            assert.are.equal(1, neg.sign)
            local val = fp.bits_to_float(neg)
            assert.is_true(math.abs(val + 3.14) < 0.001)
        end)

        it("negates negative to positive", function()
            local neg = fp.float_to_bits(-2.5, fp.FP32)
            local pos = fp.fp_neg(neg)
            assert.are.equal(0, pos.sign)
            assert.are.equal(2.5, fp.bits_to_float(pos))
        end)

        it("negates +0 to -0", function()
            local pz = fp.make_zero(0, fp.FP32)
            local nz = fp.fp_neg(pz)
            assert.is_true(fp.is_zero(nz))
            assert.are.equal(1, nz.sign)
        end)

        it("negates -0 to +0", function()
            local nz = fp.make_zero(1, fp.FP32)
            local pz = fp.fp_neg(nz)
            assert.is_true(fp.is_zero(pz))
            assert.are.equal(0, pz.sign)
        end)

        it("double negation is identity", function()
            local original = fp.float_to_bits(7.0, fp.FP32)
            local double_neg = fp.fp_neg(fp.fp_neg(original))
            assert.are.equal(7.0, fp.bits_to_float(double_neg))
        end)
    end)

    -- =====================================================================
    -- FP Absolute Value
    -- =====================================================================
    describe("fp_abs", function()
        it("abs of positive is positive", function()
            local pos = fp.float_to_bits(5.0, fp.FP32)
            local result = fp.fp_abs(pos)
            assert.are.equal(0, result.sign)
            assert.are.equal(5.0, fp.bits_to_float(result))
        end)

        it("abs of negative is positive", function()
            local neg = fp.float_to_bits(-5.0, fp.FP32)
            local result = fp.fp_abs(neg)
            assert.are.equal(0, result.sign)
            assert.are.equal(5.0, fp.bits_to_float(result))
        end)

        it("abs of -0 is +0", function()
            local nz = fp.make_zero(1, fp.FP32)
            local result = fp.fp_abs(nz)
            assert.is_true(fp.is_zero(result))
            assert.are.equal(0, result.sign)
        end)

        it("abs of NaN is NaN with sign=0", function()
            local nan = fp.make_nan(fp.FP32)
            local result = fp.fp_abs(nan)
            assert.is_true(fp.is_nan(result))
            assert.are.equal(0, result.sign)
        end)
    end)

    -- =====================================================================
    -- FP Compare
    -- =====================================================================
    describe("fp_compare", function()
        it("compares equal values", function()
            local a = fp.float_to_bits(3.0, fp.FP32)
            local b = fp.float_to_bits(3.0, fp.FP32)
            assert.are.equal(0, fp.fp_compare(a, b))
        end)

        it("compares a < b", function()
            local a = fp.float_to_bits(1.0, fp.FP32)
            local b = fp.float_to_bits(2.0, fp.FP32)
            assert.are.equal(-1, fp.fp_compare(a, b))
        end)

        it("compares a > b", function()
            local a = fp.float_to_bits(5.0, fp.FP32)
            local b = fp.float_to_bits(2.0, fp.FP32)
            assert.are.equal(1, fp.fp_compare(a, b))
        end)

        it("positive > negative", function()
            local pos = fp.float_to_bits(1.0, fp.FP32)
            local neg = fp.float_to_bits(-1.0, fp.FP32)
            assert.are.equal(1, fp.fp_compare(pos, neg))
            assert.are.equal(-1, fp.fp_compare(neg, pos))
        end)

        it("compares negative numbers (reversed)", function()
            local a = fp.float_to_bits(-1.0, fp.FP32)
            local b = fp.float_to_bits(-5.0, fp.FP32)
            assert.are.equal(1, fp.fp_compare(a, b))   -- -1 > -5
            assert.are.equal(-1, fp.fp_compare(b, a))   -- -5 < -1
        end)

        it("+0 == -0", function()
            local pz = fp.make_zero(0, fp.FP32)
            local nz = fp.make_zero(1, fp.FP32)
            assert.are.equal(0, fp.fp_compare(pz, nz))
        end)

        it("NaN comparisons return 0 (unordered)", function()
            local nan = fp.make_nan(fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            assert.are.equal(0, fp.fp_compare(nan, one))
            assert.are.equal(0, fp.fp_compare(one, nan))
            assert.are.equal(0, fp.fp_compare(nan, nan))
        end)

        it("compares with zero correctly", function()
            local zero = fp.make_zero(0, fp.FP32)
            local pos = fp.float_to_bits(1.0, fp.FP32)
            local neg = fp.float_to_bits(-1.0, fp.FP32)
            assert.are.equal(-1, fp.fp_compare(zero, pos))
            assert.are.equal(1, fp.fp_compare(zero, neg))
        end)
    end)

    -- =====================================================================
    -- FP Multiplication
    -- =====================================================================
    describe("fp_mul", function()
        it("multiplies simple positive numbers", function()
            check_fp_op(fp.fp_mul, 2.0, 3.0, 6.0)
            check_fp_op(fp.fp_mul, 1.5, 2.0, 3.0)
            check_fp_op(fp.fp_mul, 4.0, 0.25, 1.0)
        end)

        it("multiplies with negative numbers", function()
            check_fp_op(fp.fp_mul, -2.0, 3.0, -6.0)
            check_fp_op(fp.fp_mul, -2.0, -3.0, 6.0)
            check_fp_op(fp.fp_mul, 2.0, -3.0, -6.0)
        end)

        it("multiplies by zero", function()
            check_fp_op(fp.fp_mul, 5.0, 0.0, 0.0)
            check_fp_op(fp.fp_mul, 0.0, 5.0, 0.0)
        end)

        it("multiplies by one (identity)", function()
            check_fp_op(fp.fp_mul, 3.14, 1.0, 3.14, fp.FP32, 1e-6)
        end)

        it("NaN * anything = NaN", function()
            local nan = fp.make_nan(fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            assert.is_true(fp.is_nan(fp.fp_mul(nan, one)))
            assert.is_true(fp.is_nan(fp.fp_mul(one, nan)))
        end)

        it("Inf * 0 = NaN", function()
            local inf = fp.make_inf(0, fp.FP32)
            local zero = fp.make_zero(0, fp.FP32)
            assert.is_true(fp.is_nan(fp.fp_mul(inf, zero)))
        end)

        it("Inf * finite = Inf", function()
            local inf = fp.make_inf(0, fp.FP32)
            local two = fp.float_to_bits(2.0, fp.FP32)
            local result = fp.fp_mul(inf, two)
            assert.is_true(fp.is_inf(result))
            assert.are.equal(0, result.sign)
        end)

        it("Inf * (-finite) = -Inf", function()
            local inf = fp.make_inf(0, fp.FP32)
            local neg_two = fp.float_to_bits(-2.0, fp.FP32)
            local result = fp.fp_mul(inf, neg_two)
            assert.is_true(fp.is_inf(result))
            assert.are.equal(1, result.sign)
        end)

        it("overflow produces infinity", function()
            check_fp_op(fp.fp_mul, 1e30, 1e30, math.huge)
        end)

        it("works in FP16", function()
            check_fp_op(fp.fp_mul, 2.0, 3.0, 6.0, fp.FP16)
        end)

        it("works in BF16", function()
            check_fp_op(fp.fp_mul, 2.0, 3.0, 6.0, fp.BF16)
        end)
    end)

    -- =====================================================================
    -- FMA (Fused Multiply-Add)
    -- =====================================================================
    describe("fma", function()
        it("computes a*b+c correctly", function()
            local a = fp.float_to_bits(1.5, fp.FP32)
            local b = fp.float_to_bits(2.0, fp.FP32)
            local c = fp.float_to_bits(0.25, fp.FP32)
            local result = fp.bits_to_float(fp.fma(a, b, c))
            assert.is_true(math.abs(result - 3.25) < 0.001)
        end)

        it("computes a*b+c with negative c", function()
            local a = fp.float_to_bits(3.0, fp.FP32)
            local b = fp.float_to_bits(4.0, fp.FP32)
            local c = fp.float_to_bits(-2.0, fp.FP32)
            local result = fp.bits_to_float(fp.fma(a, b, c))
            assert.is_true(math.abs(result - 10.0) < 0.001)
        end)

        it("computes a*b+0 = a*b", function()
            local a = fp.float_to_bits(2.0, fp.FP32)
            local b = fp.float_to_bits(3.0, fp.FP32)
            local c = fp.make_zero(0, fp.FP32)
            local result = fp.bits_to_float(fp.fma(a, b, c))
            assert.is_true(math.abs(result - 6.0) < 0.001)
        end)

        it("NaN propagation", function()
            local nan = fp.make_nan(fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            assert.is_true(fp.is_nan(fp.fma(nan, one, one)))
            assert.is_true(fp.is_nan(fp.fma(one, nan, one)))
            assert.is_true(fp.is_nan(fp.fma(one, one, nan)))
        end)

        it("Inf * 0 + c = NaN", function()
            local inf = fp.make_inf(0, fp.FP32)
            local zero = fp.make_zero(0, fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            assert.is_true(fp.is_nan(fp.fma(inf, zero, one)))
        end)

        it("0 * 0 + c = c", function()
            local zero = fp.make_zero(0, fp.FP32)
            local five = fp.float_to_bits(5.0, fp.FP32)
            local result = fp.bits_to_float(fp.fma(zero, zero, five))
            assert.are.equal(5.0, result)
        end)

        it("Inf * finite + c = Inf", function()
            local inf = fp.make_inf(0, fp.FP32)
            local two = fp.float_to_bits(2.0, fp.FP32)
            local one = fp.float_to_bits(1.0, fp.FP32)
            local result = fp.fma(inf, two, one)
            assert.is_true(fp.is_inf(result))
        end)

        it("Inf * finite + (-Inf) = NaN", function()
            local inf = fp.make_inf(0, fp.FP32)
            local two = fp.float_to_bits(2.0, fp.FP32)
            local neg_inf = fp.make_inf(1, fp.FP32)
            assert.is_true(fp.is_nan(fp.fma(inf, two, neg_inf)))
        end)

        it("0 * 0 + 0 = 0", function()
            local z = fp.make_zero(0, fp.FP32)
            local result = fp.fma(z, z, z)
            assert.is_true(fp.is_zero(result))
        end)

        it("finite * finite + Inf = Inf", function()
            local two = fp.float_to_bits(2.0, fp.FP32)
            local inf = fp.make_inf(0, fp.FP32)
            local result = fp.fma(two, two, inf)
            assert.is_true(fp.is_inf(result))
        end)

        it("works in FP16", function()
            local a = fp.float_to_bits(2.0, fp.FP16)
            local b = fp.float_to_bits(3.0, fp.FP16)
            local c = fp.float_to_bits(1.0, fp.FP16)
            local result = fp.bits_to_float(fp.fma(a, b, c))
            assert.is_true(math.abs(result - 7.0) < 0.1)
        end)
    end)

    -- =====================================================================
    -- Format conversion
    -- =====================================================================
    describe("fp_convert", function()
        it("same format returns same value", function()
            local bits = fp.float_to_bits(3.14, fp.FP32)
            local converted = fp.fp_convert(bits, fp.FP32)
            assert.are.equal(fp.bits_to_float(bits), fp.bits_to_float(converted))
        end)

        it("FP32 -> FP16 preserves value approximately", function()
            local fp32 = fp.float_to_bits(1.5, fp.FP32)
            local fp16 = fp.fp_convert(fp32, fp.FP16)
            assert.are.equal("fp16", fp16.fmt.name)
            assert.are.equal(1.5, fp.bits_to_float(fp16))
        end)

        it("FP32 -> BF16 preserves value approximately", function()
            local fp32 = fp.float_to_bits(2.0, fp.FP32)
            local bf16 = fp.fp_convert(fp32, fp.BF16)
            assert.are.equal("bf16", bf16.fmt.name)
            assert.are.equal(2.0, fp.bits_to_float(bf16))
        end)

        it("FP16 -> FP32 is lossless for representable values", function()
            local fp16 = fp.float_to_bits(1.0, fp.FP16)
            local fp32 = fp.fp_convert(fp16, fp.FP32)
            assert.are.equal("fp32", fp32.fmt.name)
            assert.are.equal(1.0, fp.bits_to_float(fp32))
        end)

        it("BF16 -> FP32 preserves value", function()
            local bf16 = fp.float_to_bits(4.0, fp.BF16)
            local fp32 = fp.fp_convert(bf16, fp.FP32)
            assert.are.equal(4.0, fp.bits_to_float(fp32))
        end)

        it("FP16 -> BF16 converts correctly", function()
            local fp16 = fp.float_to_bits(2.0, fp.FP16)
            local bf16 = fp.fp_convert(fp16, fp.BF16)
            assert.are.equal(2.0, fp.bits_to_float(bf16))
        end)

        it("preserves infinity across formats", function()
            local inf32 = fp.make_inf(0, fp.FP32)
            local inf16 = fp.fp_convert(inf32, fp.FP16)
            assert.is_true(fp.is_inf(inf16))
        end)

        it("preserves NaN across formats", function()
            local nan32 = fp.make_nan(fp.FP32)
            local nan16 = fp.fp_convert(nan32, fp.FP16)
            assert.is_true(fp.is_nan(nan16))
        end)

        it("preserves zero across formats", function()
            local z32 = fp.make_zero(0, fp.FP32)
            local z16 = fp.fp_convert(z32, fp.FP16)
            assert.is_true(fp.is_zero(z16))
        end)
    end)

    -- =====================================================================
    -- Pipelined FP Adder
    -- =====================================================================
    describe("PipelinedFPAdder", function()
        it("adds two numbers after pipeline fills", function()
            local clk = clock_mod.Clock.new(1000000)
            local adder = fp.PipelinedFPAdder.new(clk, fp.FP32)

            local a = fp.float_to_bits(1.5, fp.FP32)
            local b = fp.float_to_bits(2.5, fp.FP32)
            adder:submit(a, b)

            -- Run enough cycles for the pipeline to flush (5 stages + 1)
            for _ = 1, 6 do
                clk:full_cycle()
            end

            assert.is_true(#adder.results > 0)
            local result = fp.bits_to_float(adder.results[1])
            assert.is_true(math.abs(result - 4.0) < 0.001)
        end)

        it("handles multiple submissions (pipeline throughput)", function()
            local clk = clock_mod.Clock.new(1000000)
            local adder = fp.PipelinedFPAdder.new(clk, fp.FP32)

            -- Submit 3 additions
            adder:submit(fp.float_to_bits(1.0, fp.FP32), fp.float_to_bits(2.0, fp.FP32))
            adder:submit(fp.float_to_bits(3.0, fp.FP32), fp.float_to_bits(4.0, fp.FP32))
            adder:submit(fp.float_to_bits(5.0, fp.FP32), fp.float_to_bits(6.0, fp.FP32))

            -- Run enough cycles
            for _ = 1, 10 do
                clk:full_cycle()
            end

            assert.are.equal(3, #adder.results)
        end)

        it("handles special values in pipeline", function()
            local clk = clock_mod.Clock.new(1000000)
            local adder = fp.PipelinedFPAdder.new(clk, fp.FP32)

            -- NaN input
            adder:submit(fp.make_nan(fp.FP32), fp.float_to_bits(1.0, fp.FP32))

            for _ = 1, 6 do
                clk:full_cycle()
            end

            assert.is_true(#adder.results > 0)
            assert.is_true(fp.is_nan(adder.results[1]))
        end)

        it("tracks cycle count", function()
            local clk = clock_mod.Clock.new(1000000)
            local adder = fp.PipelinedFPAdder.new(clk, fp.FP32)
            clk:full_cycle()
            clk:full_cycle()
            assert.are.equal(2, adder.cycle_count)
        end)
    end)

    -- =====================================================================
    -- Pipelined FP Multiplier
    -- =====================================================================
    describe("PipelinedFPMultiplier", function()
        it("multiplies two numbers after pipeline fills", function()
            local clk = clock_mod.Clock.new(1000000)
            local mul = fp.PipelinedFPMultiplier.new(clk, fp.FP32)

            local a = fp.float_to_bits(3.0, fp.FP32)
            local b = fp.float_to_bits(4.0, fp.FP32)
            mul:submit(a, b)

            for _ = 1, 5 do
                clk:full_cycle()
            end

            assert.is_true(#mul.results > 0)
            local result = fp.bits_to_float(mul.results[1])
            assert.is_true(math.abs(result - 12.0) < 0.001)
        end)

        it("handles multiple submissions", function()
            local clk = clock_mod.Clock.new(1000000)
            local mul = fp.PipelinedFPMultiplier.new(clk, fp.FP32)

            mul:submit(fp.float_to_bits(2.0, fp.FP32), fp.float_to_bits(3.0, fp.FP32))
            mul:submit(fp.float_to_bits(4.0, fp.FP32), fp.float_to_bits(5.0, fp.FP32))

            for _ = 1, 8 do
                clk:full_cycle()
            end

            assert.are.equal(2, #mul.results)
        end)

        it("handles Inf * 0 = NaN in pipeline", function()
            local clk = clock_mod.Clock.new(1000000)
            local mul = fp.PipelinedFPMultiplier.new(clk, fp.FP32)

            mul:submit(fp.make_inf(0, fp.FP32), fp.make_zero(0, fp.FP32))

            for _ = 1, 5 do
                clk:full_cycle()
            end

            assert.is_true(#mul.results > 0)
            assert.is_true(fp.is_nan(mul.results[1]))
        end)
    end)

    -- =====================================================================
    -- Pipelined FMA
    -- =====================================================================
    describe("PipelinedFMA", function()
        it("computes a*b+c after pipeline fills", function()
            local clk = clock_mod.Clock.new(1000000)
            local fma_unit = fp.PipelinedFMA.new(clk, fp.FP32)

            local a = fp.float_to_bits(2.0, fp.FP32)
            local b = fp.float_to_bits(3.0, fp.FP32)
            local c = fp.float_to_bits(1.0, fp.FP32)
            fma_unit:submit(a, b, c)

            for _ = 1, 7 do
                clk:full_cycle()
            end

            assert.is_true(#fma_unit.results > 0)
            local result = fp.bits_to_float(fma_unit.results[1])
            assert.is_true(math.abs(result - 7.0) < 0.001)
        end)

        it("handles NaN in pipeline", function()
            local clk = clock_mod.Clock.new(1000000)
            local fma_unit = fp.PipelinedFMA.new(clk, fp.FP32)

            fma_unit:submit(
                fp.make_nan(fp.FP32),
                fp.float_to_bits(1.0, fp.FP32),
                fp.float_to_bits(1.0, fp.FP32)
            )

            for _ = 1, 7 do
                clk:full_cycle()
            end

            assert.is_true(#fma_unit.results > 0)
            assert.is_true(fp.is_nan(fma_unit.results[1]))
        end)

        it("handles multiple submissions", function()
            local clk = clock_mod.Clock.new(1000000)
            local fma_unit = fp.PipelinedFMA.new(clk, fp.FP32)

            fma_unit:submit(
                fp.float_to_bits(1.0, fp.FP32),
                fp.float_to_bits(2.0, fp.FP32),
                fp.float_to_bits(3.0, fp.FP32)
            )
            fma_unit:submit(
                fp.float_to_bits(4.0, fp.FP32),
                fp.float_to_bits(5.0, fp.FP32),
                fp.float_to_bits(6.0, fp.FP32)
            )

            for _ = 1, 10 do
                clk:full_cycle()
            end

            assert.are.equal(2, #fma_unit.results)
        end)
    end)

    -- =====================================================================
    -- FPUnit (complete unit)
    -- =====================================================================
    describe("FPUnit", function()
        it("creates all three pipelines", function()
            local clk = clock_mod.Clock.new(1000000)
            local unit = fp.FPUnit.new(clk, fp.FP32)

            assert.is_not_nil(unit.adder)
            assert.is_not_nil(unit.multiplier)
            assert.is_not_nil(unit.fma)
        end)

        it("tick advances all pipelines", function()
            local clk = clock_mod.Clock.new(1000000)
            local unit = fp.FPUnit.new(clk, fp.FP32)

            unit.adder:submit(
                fp.float_to_bits(1.0, fp.FP32),
                fp.float_to_bits(2.0, fp.FP32)
            )
            unit.multiplier:submit(
                fp.float_to_bits(3.0, fp.FP32),
                fp.float_to_bits(4.0, fp.FP32)
            )

            unit:tick(8)

            assert.is_true(#unit.adder.results > 0)
            assert.is_true(#unit.multiplier.results > 0)

            local add_result = fp.bits_to_float(unit.adder.results[1])
            local mul_result = fp.bits_to_float(unit.multiplier.results[1])

            assert.is_true(math.abs(add_result - 3.0) < 0.001)
            assert.is_true(math.abs(mul_result - 12.0) < 0.001)
        end)
    end)

    -- =====================================================================
    -- Cross-format arithmetic consistency
    -- =====================================================================
    describe("cross-format consistency", function()
        it("FP32 and FP16 addition agree for small values", function()
            local a32 = fp.float_to_bits(1.0, fp.FP32)
            local b32 = fp.float_to_bits(2.0, fp.FP32)
            local r32 = fp.bits_to_float(fp.fp_add(a32, b32))

            local a16 = fp.float_to_bits(1.0, fp.FP16)
            local b16 = fp.float_to_bits(2.0, fp.FP16)
            local r16 = fp.bits_to_float(fp.fp_add(a16, b16))

            assert.are.equal(r32, r16)
        end)

        it("FP32 and BF16 multiplication agree for powers of 2", function()
            local a32 = fp.float_to_bits(2.0, fp.FP32)
            local b32 = fp.float_to_bits(4.0, fp.FP32)
            local r32 = fp.bits_to_float(fp.fp_mul(a32, b32))

            local abf = fp.float_to_bits(2.0, fp.BF16)
            local bbf = fp.float_to_bits(4.0, fp.BF16)
            local rbf = fp.bits_to_float(fp.fp_mul(abf, bbf))

            assert.are.equal(r32, rbf)
        end)
    end)

    -- =====================================================================
    -- Edge cases and stress tests
    -- =====================================================================
    describe("edge cases", function()
        it("very small number + very small number", function()
            local tiny = fp.float_to_bits(1e-38, fp.FP32)
            local result = fp.fp_add(tiny, tiny)
            local val = fp.bits_to_float(result)
            assert.is_true(val > 0)
            assert.is_true(math.abs(val - 2e-38) / 2e-38 < 0.01)
        end)

        it("large + small doesn't lose the large", function()
            local big = fp.float_to_bits(1e10, fp.FP32)
            local small = fp.float_to_bits(1.0, fp.FP32)
            local result = fp.bits_to_float(fp.fp_add(big, small))
            assert.is_true(result >= 1e10)
        end)

        it("multiply produces denormalized result near underflow", function()
            local tiny = fp.float_to_bits(1e-20, fp.FP32)
            local result = fp.fp_mul(tiny, tiny)
            -- 1e-40 underflows in FP32 to zero or denormal
            local val = fp.bits_to_float(result)
            assert.is_true(val >= 0)
        end)

        it("sign bit XOR logic for multiplication", function()
            -- (+) * (+) = (+)
            local pp = fp.fp_mul(
                fp.float_to_bits(2.0, fp.FP32),
                fp.float_to_bits(3.0, fp.FP32))
            assert.are.equal(0, pp.sign)

            -- (+) * (-) = (-)
            local pn = fp.fp_mul(
                fp.float_to_bits(2.0, fp.FP32),
                fp.float_to_bits(-3.0, fp.FP32))
            assert.are.equal(1, pn.sign)

            -- (-) * (-) = (+)
            local nn = fp.fp_mul(
                fp.float_to_bits(-2.0, fp.FP32),
                fp.float_to_bits(-3.0, fp.FP32))
            assert.are.equal(0, nn.sign)
        end)

        it("addition is commutative", function()
            local a = fp.float_to_bits(3.14, fp.FP32)
            local b = fp.float_to_bits(2.72, fp.FP32)
            local ab = fp.bits_to_float(fp.fp_add(a, b))
            local ba = fp.bits_to_float(fp.fp_add(b, a))
            assert.are.equal(ab, ba)
        end)

        it("multiplication is commutative", function()
            local a = fp.float_to_bits(3.14, fp.FP32)
            local b = fp.float_to_bits(2.72, fp.FP32)
            local ab = fp.bits_to_float(fp.fp_mul(a, b))
            local ba = fp.bits_to_float(fp.fp_mul(b, a))
            assert.are.equal(ab, ba)
        end)
    end)
end)
