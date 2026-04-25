# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `LCG` class: Linear Congruential Generator with Knuth/Numerical Recipes constants;
  all state arithmetic uses `BigInt` with `& MASK64` for 64-bit wrapping
- `Xorshift64` class: Marsaglia three-shift generator; seed `0n` replaced with `1n`
- `PCG32` class: Permuted Congruential Generator with XSH RR permutation and
  initseq warm-up; rotate-right computed with `BigInt` bit operations
- Common API on all three: `nextU32` (returns `number`), `nextU64` (returns `bigint`),
  `nextFloat` (returns `number`), `nextIntInRange` (returns `number`)
- Rejection sampling in `nextIntInRange` to eliminate modulo bias
- `MASK64` and `MASK32` BigInt constants
- 31 tests; all passing
- Cross-language reference values verified against Go implementation
