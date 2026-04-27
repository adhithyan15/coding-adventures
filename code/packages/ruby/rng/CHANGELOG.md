# Changelog

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-25

### Added

- `CodingAdventures::Rng::LCG` — Linear Congruential Generator (Knuth 1948).
  Full 2^64-period recurrence `state = state × a + c` (mod 2^64) using the
  Knuth/Numerical Recipes constants.  Output: upper 32 bits of state.
- `CodingAdventures::Rng::Xorshift64` — Marsaglia (2003) XOR-shift generator.
  Three XOR-shift operations; period 2^64 − 1; seed 0 replaced with 1.
  Output: lower 32 bits of state.
- `CodingAdventures::Rng::PCG32` — Permuted Congruential Generator (O'Neill 2014).
  LCG recurrence with XSH RR output permutation.  Two-step initseq warm-up
  for high-quality output even from small seeds.
- Shared API on all three generators: `next_u32`, `next_u64`, `next_float`,
  `next_int_in_range(min, max)`.
- Rejection-sampling in `next_int_in_range` to eliminate modulo bias.
- 38 unit tests covering reference values, range bounds, reproducibility,
  composition, and edge cases (seed 0, single-value ranges, negative ranges).
  Line coverage: 100%.
- `simplecov` (>= 0.22) added as a development dependency for coverage reporting.
- `test/test_helper.rb` created with correct SimpleCov/Minitest load order
  (SimpleCov registered first so its `at_exit` fires after tests complete).
