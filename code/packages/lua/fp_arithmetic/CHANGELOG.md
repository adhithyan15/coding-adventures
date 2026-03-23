# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-23

### Added

- **formats.lua**: FloatFormat and FloatBits types with metatable OOP; FP32, FP16, BF16 format constants; bit-level utility functions (int_to_bits_msb, bits_msb_to_int, bit_length)
- **ieee754.lua**: IEEE 754 encode (float_to_bits) and decode (bits_to_float) using string.pack/unpack for FP32 hardware-exact conversion, manual conversion for FP16/BF16; special value detection (is_nan, is_inf, is_zero, is_denormalized) built from logic gates
- **fp_adder.lua**: Full FP addition with 8-step algorithm (unpack, align, add/sub, normalize, round-to-nearest-even); subtraction via sign-flip; negation, absolute value, comparison
- **fp_multiplier.lua**: FP multiplication with sign XOR, exponent addition, mantissa product, normalization, and rounding
- **fma.lua**: Fused multiply-add (a*b+c with single rounding); format conversion between FP32/FP16/BF16
- **pipeline.lua**: Clock-driven pipelined hardware simulation -- 5-stage FP adder, 4-stage FP multiplier, 6-stage FMA unit, and FPUnit combining all three
- **init.lua**: Unified module exporting all types, constants, operations, and pipeline classes
- Comprehensive busted test suite with 123 tests covering all modules, special values, edge cases, cross-format consistency, and pipeline behavior
