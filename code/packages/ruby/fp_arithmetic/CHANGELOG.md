# Changelog

All notable changes to `coding_adventures_fp_arithmetic` will be documented here.

## [0.1.0] - 2026-03-18

### Added
- `FloatFormat` and `FloatBits` immutable value objects using `Data.define`
- Format constants: `FP32`, `FP16`, `BF16`
- `float_to_bits` / `bits_to_float` encoding and decoding for all three formats
- Special value detection: `nan?`, `inf?`, `zero?`, `denormalized?`
- Bit helper functions: `int_to_bits_msb`, `bits_msb_to_int`, `all_ones?`, `all_zeros?`
- `fp_add` -- IEEE 754 floating-point addition with round-to-nearest-even
- `fp_sub` -- subtraction via sign-flip + addition
- `fp_neg` -- negation (flip sign bit)
- `fp_abs` -- absolute value (clear sign bit)
- `fp_compare` -- three-way comparison (-1, 0, 1)
- `fp_mul` -- IEEE 754 floating-point multiplication
- `fp_fma` -- fused multiply-add with single rounding
- `fp_convert` -- format conversion between FP32, FP16, BF16
- `PipelinedFPAdder` -- 5-stage clock-driven pipelined adder
- `PipelinedFPMultiplier` -- 4-stage clock-driven pipelined multiplier
- `PipelinedFMA` -- 6-stage clock-driven pipelined FMA unit
- `FPUnit` -- complete floating-point unit combining all three pipelines
