# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-18

### Added
- `FloatFormat` dataclass and constants: `FP32`, `FP16`, `BF16`
- `FloatBits` dataclass for representing IEEE 754 bit patterns
- `float_to_bits()` / `bits_to_float()` encoding and decoding
- Special value detection: `is_nan()`, `is_inf()`, `is_zero()`, `is_denormalized()`
- `fp_add()` / `fp_sub()` floating-point addition and subtraction from gates
- `fp_neg()` / `fp_abs()` sign manipulation
- `fp_compare()` floating-point comparison
- `fp_mul()` floating-point multiplication from gates
- `fp_fma()` fused multiply-add with single rounding
- `fp_convert()` format conversion (FP32/FP16/BF16)
- Internal `_gates` module with standalone AND, OR, NOT, XOR, ripple_carry_adder

### Changed
- Made package fully self-contained with no external dependencies
  (previously depended on coding-adventures-logic-gates and coding-adventures-arithmetic)
- Raised test coverage threshold from 80% to 90% (targeting 95%+)
- Expanded test suite with comprehensive edge case coverage:
  denormals, overflow, underflow, NaN propagation, signed zeros,
  infinity arithmetic, catastrophic cancellation, format conversion
