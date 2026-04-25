# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `Lcg` class: Linear Congruential Generator (Knuth 1948) with 64-bit state,
  upper-32-bit output, and the Knuth multiplier/increment constants.
- `Xorshift64` class: Marsaglia (2003) three-shift generator with seed-0 fixup.
- `Pcg32` class: O'Neill (2014) Permuted Congruential Generator with XSH RR
  output permutation and two-step initseq warm-up.
- All three classes expose: `NextU32()`, `NextU64()`, `NextFloat()`,
  `NextIntInRange(long, long)`.
- `NextIntInRange` uses rejection sampling to eliminate modulo bias.
- 29 xUnit `[Fact]` tests covering known reference vectors, determinism,
  float bounds, range bounds, full coverage of small ranges, and
  cross-generator independence.
- Targets .NET 9; uses `uint.RotateRight` (available since .NET 6) for PCG32.
