# Changelog

All notable changes to the fp-arithmetic Go package will be documented in this file.

## [0.1.0] - 2026-03-18

### Added

- **formats.go**: `FloatFormat` and `FloatBits` structs with `FP32`, `FP16`, `BF16` format constants.
  Helper constructors `makeNaN`, `makeInf`, `makeZero` for special IEEE 754 values.
- **ieee754.go**: `FloatToBits` and `BitsToFloat` for encoding/decoding between Go float64 and
  bit-level IEEE 754 representation. Supports FP32, FP16, and BF16 formats. Special value detectors
  `IsNaN`, `IsInf`, `IsZero`, `IsDenormalized` built from logic gates (AND/OR chains).
  Utility functions `IntToBitsMSB` and `BitsMSBToInt` for bit-integer conversion.
- **fp_adder.go**: `FPAdd` (full IEEE 754 addition with alignment, normalization, rounding),
  `FPSub` (subtraction via sign flip + addition), `FPNeg` (sign negation via XOR),
  `FPAbs` (absolute value), `FPCompare` (three-way comparison returning -1/0/1).
- **fp_multiplier.go**: `FPMul` implementing IEEE 754 multiplication with shift-and-add mantissa
  multiply, exponent addition, normalization, and round-to-nearest-even.
- **fma.go**: `FMA` (fused multiply-add with single rounding step) and `FPConvert` for format
  conversion between FP32, FP16, and BF16.
- **pipeline.go**: Clock-driven pipelined FP units using Go's sync.Mutex for thread safety:
  - `PipelinedFPAdder` (5-stage): unpack, align, add/sub, normalize, round/pack
  - `PipelinedFPMultiplier` (4-stage): unpack+exp, multiply, normalize, round/pack
  - `PipelinedFMA` (6-stage): unpack, multiply, align, add, normalize, round/pack
  - `FPUnit`: composite unit with all three pipelines sharing a clock
- Full test suite with 77 tests achieving 85.6% coverage.
