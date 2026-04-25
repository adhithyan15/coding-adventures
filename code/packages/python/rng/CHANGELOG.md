# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `LCG` class: Linear Congruential Generator with Knuth/Numerical Recipes constants;
  all arithmetic masked with `& _MASK64` to emulate 64-bit unsigned wrapping
- `Xorshift64` class: Marsaglia three-shift generator; seed 0 replaced with 1
- `PCG32` class: Permuted Congruential Generator with XSH RR permutation and
  initseq warm-up; rotate-right implemented with `(-rot) & 31` left-rotate trick
- Common API on all three: `next_u32`, `next_u64`, `next_float`, `next_int_in_range`
- Rejection sampling in `next_int_in_range` to eliminate modulo bias
- `_MASK64` and `_MASK32` module-level constants
- 36 tests; 100% statement coverage; ruff clean
- Cross-language reference values verified against Go implementation
