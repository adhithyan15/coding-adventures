# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial TypeScript port from Python fp-arithmetic package
- IEEE 754 encoding/decoding (`floatToBits`, `bitsToFloat`) using DataView for FP32 and manual conversion for FP16/BF16
- Special value detection: `isNaN`, `isInf`, `isZero`, `isDenormalized`
- Format definitions: `FP32`, `FP16`, `BF16` with `FloatFormat` and `FloatBits` interfaces
- Floating-point addition (`fpAdd`) with full IEEE 754 compliance including denormals, rounding, and special values
- Floating-point subtraction (`fpSub`), negation (`fpNeg`), absolute value (`fpAbs`), comparison (`fpCompare`)
- Floating-point multiplication (`fpMul`) with shift-and-add algorithm
- Fused multiply-add (`fpFma`) with single rounding step for ML accuracy
- Format conversion (`fpConvert`) between FP32, FP16, and BF16
- Pipelined FP units: `PipelinedFPAdder` (5-stage), `PipelinedFPMultiplier` (4-stage), `PipelinedFMA` (6-stage)
- Complete `FPUnit` class combining all three pipelines
- Uses `BigInt` for all bit-level manipulation to avoid JavaScript precision limits
- All literate programming comments preserved from Python source
- Comprehensive test suite with 200+ test cases covering edge cases, special values, and denormals
