# Changelog

All notable changes to the `fp-arithmetic` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- **formats**: `FloatFormat` struct and `FloatBits` struct with `FP32`, `FP16`, `BF16` constants.
  Helper constructors `make_nan`, `make_inf`, `make_zero`.
- **ieee754**: `float_to_bits` and `bits_to_float` for encoding/decoding between Rust `f64`
  and IEEE 754 bit-level representation. Special value detectors: `is_nan`, `is_inf`,
  `is_zero`, `is_denormalized`. Bit conversion utilities: `int_to_bits_msb`, `bits_msb_to_int`.
- **fp_adder**: `fp_add` (full IEEE 754 addition with guard/round/sticky bits),
  `fp_sub`, `fp_neg`, `fp_abs`, `fp_compare`.
- **fp_multiplier**: `fp_mul` (IEEE 754 multiplication with shift-and-add, round-to-nearest-even).
- **fma**: `fp_fma` (fused multiply-add with single rounding step), `fp_convert` (format conversion
  between FP32, FP16, BF16).
- **pipeline**: `PipelinedFPAdder` (5-stage), `PipelinedFPMultiplier` (4-stage),
  `PipelinedFMA` (6-stage), and `FPUnit` (complete floating-point unit with all three pipelines).
- 61 unit tests and 3 doc-tests covering all modules.

### Notes

- Ported from the Go reference implementation (`code/packages/go/fp-arithmetic/`).
- Pipeline implementation uses `Vec`-based stage queues with explicit `tick()` method
  (Rust adaptation of Go's goroutine/channel approach with clock listeners).
- Shift overflow guards added for FMA alignment stage to prevent panics when
  exponent differences exceed 63 bits.
