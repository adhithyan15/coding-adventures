# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `Lcg` class: Linear Congruential Generator (Knuth 1948) with 64-bit state,
  upper-32-bit output, and the Knuth multiplier/increment constants.
  All state arithmetic masked to 64 bits because Dart integers are
  arbitrary-precision.
- `Xorshift64` class: Marsaglia (2003) three-shift generator. Uses `>>>` for
  unsigned right shift (Dart 2.14+). Seed 0 replaced with 1 to avoid the
  zero fixed point.
- `Pcg32` class: O'Neill (2014) Permuted Congruential Generator with XSH RR
  output permutation. Rotation implemented manually:
  `((xorshifted >>> rot) | (xorshifted << (32 - rot))) & 0xFFFFFFFF`.
  Two-step initseq warm-up applied in the constructor.
- All three classes expose: `nextU32()`, `nextU64()`, `nextFloat()`,
  `nextIntInRange(int, int)`.
- `nextIntInRange` uses rejection sampling to eliminate modulo bias.
- Barrel export via `lib/coding_adventures_rng.dart`.
- 32 `package:test` tests covering known reference vectors, seed-0 fixup,
  determinism, float bounds, range bounds, full coverage of small ranges, and
  cross-generator independence.
