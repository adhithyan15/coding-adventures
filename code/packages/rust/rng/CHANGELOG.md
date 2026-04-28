# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `Lcg` struct: Linear Congruential Generator with Knuth/Numerical Recipes constants;
  all arithmetic uses `wrapping_mul`/`wrapping_add` for correct 64-bit overflow
- `Xorshift64` struct: Marsaglia three-shift generator; seed 0 replaced with 1
- `Pcg32` struct: Permuted Congruential Generator with XSH RR permutation,
  initseq warm-up, and `u32::rotate_right` for the output permutation
- Common API on all three: `new`, `next_u32`, `next_u64`, `next_float`, `next_int_in_range`
- Rejection sampling in `next_int_in_range` to eliminate modulo bias
- 27 unit tests + 1 doc test; 100% statement coverage
- Cross-language reference values verified against Go implementation
