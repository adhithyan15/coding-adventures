-- fp-arithmetic -- IEEE 754 floating-point arithmetic from logic gates
--
-- This package implements the complete floating-point arithmetic stack:
-- encoding/decoding, addition, subtraction, multiplication, fused multiply-add,
-- format conversion, and pipelined hardware simulation -- all built on top of
-- logic gates, just like real hardware.
--
-- === Supported formats ===
--
--   Format  Total  Exp  Mantissa  Bias   Used by
--   ------  -----  ---  --------  ----   -------
--   FP32     32     8     23      127    CPU, GPU (default precision)
--   FP16     16     5     10       15    GPU training (mixed precision)
--   BF16     16     8      7      127    TPU (native), ML training
--
-- === The computing stack ===
--
-- This package is part of the coding-adventures monorepo, a ground-up
-- implementation of the computing stack from transistors to operating systems.
-- Layer 9 in the computing stack.
--
-- Dependencies:
--   - logic-gates (Layer 10): AND, OR, XOR gates for bit-level operations
--   - clock (Layer 8): Clock signal for pipeline simulation

local formats = require("coding_adventures.fp_arithmetic.formats")
local ieee754 = require("coding_adventures.fp_arithmetic.ieee754")
local fp_adder = require("coding_adventures.fp_arithmetic.fp_adder")
local fp_multiplier = require("coding_adventures.fp_arithmetic.fp_multiplier")
local fma_mod = require("coding_adventures.fp_arithmetic.fma")
local pipeline = require("coding_adventures.fp_arithmetic.pipeline")

return {
    VERSION = "0.1.0",

    -- Format types and constants
    FloatFormat = formats.FloatFormat,
    FloatBits = formats.FloatBits,
    FP32 = formats.FP32,
    FP16 = formats.FP16,
    BF16 = formats.BF16,

    -- Bit utilities
    int_to_bits_msb = formats.int_to_bits_msb,
    bits_msb_to_int = formats.bits_msb_to_int,
    bit_length = formats.bit_length,

    -- Special value constructors
    make_nan = formats.make_nan,
    make_inf = formats.make_inf,
    make_zero = formats.make_zero,

    -- Encoding / decoding
    float_to_bits = ieee754.float_to_bits,
    bits_to_float = ieee754.bits_to_float,

    -- Special value detection
    is_nan = ieee754.is_nan,
    is_inf = ieee754.is_inf,
    is_zero = ieee754.is_zero,
    is_denormalized = ieee754.is_denormalized,

    -- Arithmetic operations
    fp_add = fp_adder.fp_add,
    fp_sub = fp_adder.fp_sub,
    fp_neg = fp_adder.fp_neg,
    fp_abs = fp_adder.fp_abs,
    fp_compare = fp_adder.fp_compare,
    fp_mul = fp_multiplier.fp_mul,
    fma = fma_mod.fma,
    fp_convert = fma_mod.fp_convert,

    -- Pipelined units
    PipelinedFPAdder = pipeline.PipelinedFPAdder,
    PipelinedFPMultiplier = pipeline.PipelinedFPMultiplier,
    PipelinedFMA = pipeline.PipelinedFMA,
    FPUnit = pipeline.FPUnit,
}
