# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `LCG` struct: Linear Congruential Generator with Knuth/Numerical Recipes constants
- `Xorshift64` struct: Marsaglia three-shift generator; seed 0 replaced with 1
- `PCG32` struct: Permuted Congruential Generator with XSH RR permutation and initseq warm-up
- Common API on all three: `NextU32`, `NextU64`, `NextFloat`, `NextIntInRange`
- Rejection sampling in `NextIntInRange` to eliminate modulo bias
- 22 tests; 100% statement coverage
