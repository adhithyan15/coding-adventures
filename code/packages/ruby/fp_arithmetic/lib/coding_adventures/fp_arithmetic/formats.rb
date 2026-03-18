# frozen_string_literal: true

# ---------------------------------------------------------------------------
# IEEE 754 floating-point formats -- the bit-level anatomy of a float.
# ---------------------------------------------------------------------------
#
# === What is a floating-point format? ===
#
# Floating-point is how computers represent real numbers (like 3.14 or -0.001).
# It works like scientific notation, but in binary:
#
#     Scientific notation:   -6.022 x 10^23
#     IEEE 754 (binary):     (-1)^sign x 1.mantissa x 2^(exponent - bias)
#
# A floating-point number is stored as three bit fields packed into a
# fixed-width binary word:
#
#     FP32 (32 bits):  [sign(1)] [exponent(8)] [mantissa(23)]
#                       ^         ^              ^
#                       |         |              |
#                       |         |              +-- fractional part (after the "1.")
#                       |         +-- power of 2 (biased: stored value - 127)
#                       +-- 0 = positive, 1 = negative
#
# === The three formats we support ===
#
#     Format  Total  Exp  Mantissa  Bias   Used by
#     ------  -----  ---  --------  ----   -------
#     FP32     32     8     23      127    CPU, GPU (default precision)
#     FP16     16     5     10       15    GPU training (mixed precision)
#     BF16     16     8      7      127    TPU (native), ML training
#
# === Why BF16 exists ===
#
# BF16 (Brain Float 16) was invented by Google for TPU hardware. It keeps
# the same exponent range as FP32 (8-bit exponent, bias 127) but truncates
# the mantissa from 23 bits to just 7. This means:
#
#   - Same range as FP32 (can represent very large and very small numbers)
#   - Much less precision (~2-3 decimal digits vs ~7 for FP32)
#   - Perfect for ML: gradients can be huge or tiny (need range), but
#     don't need to be super precise (need less precision)
#   - Trivial conversion from FP32: just truncate the lower 16 bits!
#
# === The implicit leading 1 ===
#
# For normal (non-zero, non-denormal) numbers, the mantissa has an implicit
# leading 1 that is not stored. So a stored mantissa of [1, 0, 1, ...] actually
# represents 1.101... in binary. This trick gives us one extra bit of precision
# for free.
#
#     Stored bits:   [1, 0, 1, 0, 0, ...]
#     Actual value:  1.10100...  (the "1." is implicit)
#
# The only exception is denormalized numbers (exponent = all zeros), where the
# implicit bit is 0 instead of 1, allowing representation of very small numbers
# near zero.
#
# === Ruby Implementation Notes ===
#
# We use Data.define (Ruby 3.2+) to create immutable value objects for both
# FloatFormat and FloatBits. This mirrors Python's frozen dataclasses and
# ensures that format constants and bit patterns cannot be accidentally mutated.

module CodingAdventures
  module FpArithmetic
    # FloatFormat -- describes the shape of a floating-point format.
    #
    # This is an immutable value object (Data.define). Once created, you
    # cannot change its fields. This prevents bugs where code accidentally
    # mutates a format constant.
    #
    # Members:
    #   name           - Human-readable name ("fp32", "fp16", "bf16").
    #   total_bits     - Total width of the format in bits.
    #   exponent_bits  - Number of bits in the exponent field.
    #   mantissa_bits  - Number of explicit mantissa bits (without the implicit
    #                    leading 1). Actual precision is mantissa_bits + 1.
    #   bias           - The exponent bias. True exponent = stored_exponent - bias.
    #
    # Example:
    #   FP32.total_bits     # => 32
    #   FP32.exponent_bits  # => 8
    #   FP32.mantissa_bits  # => 23
    #   FP32.bias           # => 127
    FloatFormat = Data.define(:name, :total_bits, :exponent_bits, :mantissa_bits, :bias)

    # -----------------------------------------------------------------------
    # Standard format constants
    # -----------------------------------------------------------------------
    # These are module-level singletons. All code that works with floating-point
    # should reference these constants rather than constructing FloatFormat manually.

    # FP32 (single precision) -- the workhorse of computing.
    #
    #   [sign(1)] [exponent(8)] [mantissa(23)]
    #    bit 31    bits 30-23    bits 22-0
    #
    # Used by CPU FPUs, GPU CUDA cores, and as the default for most computation.
    # Range: ~1.18e-38 to ~3.40e38, precision: ~7 decimal digits.
    FP32 = FloatFormat.new(name: "fp32", total_bits: 32, exponent_bits: 8, mantissa_bits: 23, bias: 127)

    # FP16 (half precision) -- GPU mixed-precision training.
    #
    #   [sign(1)] [exponent(5)] [mantissa(10)]
    #    bit 15    bits 14-10    bits 9-0
    #
    # Used for GPU training in mixed precision and inference.
    # Range: ~5.96e-8 to ~65504, precision: ~3-4 decimal digits.
    FP16 = FloatFormat.new(name: "fp16", total_bits: 16, exponent_bits: 5, mantissa_bits: 10, bias: 15)

    # BF16 (brain float) -- Google's TPU native format.
    #
    #   [sign(1)] [exponent(8)] [mantissa(7)]
    #    bit 15    bits 14-7     bits 6-0
    #
    # Same exponent range as FP32, but with only 7 mantissa bits (vs 23).
    # Range: same as FP32, precision: ~2-3 decimal digits.
    BF16 = FloatFormat.new(name: "bf16", total_bits: 16, exponent_bits: 8, mantissa_bits: 7, bias: 127)

    # FloatBits -- the actual bit pattern of a floating-point number.
    #
    # This stores the actual 0s and 1s that make up the number, decomposed
    # into the three fields (sign, exponent, mantissa). All bit lists are
    # stored MSB-first (index 0 = most significant bit).
    #
    # Members:
    #   sign      - 0 for positive, 1 for negative.
    #   exponent  - Array of exponent bits, MSB first. Length = fmt.exponent_bits.
    #   mantissa  - Array of mantissa bits, MSB first. Length = fmt.mantissa_bits.
    #              These are the explicit bits only (no implicit leading 1).
    #   fmt       - The FloatFormat this number is encoded in.
    #
    # Example:
    #   bits = FloatBits.new(
    #     sign: 0,
    #     exponent: [0,1,1,1,1,1,1,1],
    #     mantissa: [0]*23,
    #     fmt: FP32
    #   )
    #   # This is +1.0 in FP32: exponent 127-127=0, mantissa 1.000...
    FloatBits = Data.define(:sign, :exponent, :mantissa, :fmt)
  end
end
