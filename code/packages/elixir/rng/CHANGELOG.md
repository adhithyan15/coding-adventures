# Changelog

All notable changes to this package will be documented in this file.

The format follows [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-04-25

### Added

- `CodingAdventures.Rng.LCG` — Linear Congruential Generator (Knuth 1948).
  Full 2^64-period recurrence `state = state × a + c` (mod 2^64) using the
  Knuth/Numerical Recipes constants.  Output: upper 32 bits of state.
  Struct-based; all functions return `{value, new_struct}` tuples.
- `CodingAdventures.Rng.Xorshift64` — Marsaglia (2003) XOR-shift generator.
  Three XOR-shift operations; period 2^64 − 1; seed 0 replaced with 1.
  Output: lower 32 bits of state.
- `CodingAdventures.Rng.PCG32` — Permuted Congruential Generator (O'Neill 2014).
  LCG recurrence with XSH RR output permutation.  Two-step initseq warm-up
  for high-quality output from any seed.
- Shared constants (`@multiplier`, `@increment`, `@mask64`, `@mask32`,
  `@float_div`) injected via `CodingAdventures.Rng.__using__/1` macro.
- Shared API on all three generators: `new/1`, `next_u32/1`, `next_u64/1`,
  `next_float/1`, `next_int_in_range/3`.
- Rejection-sampling in `next_int_in_range` using Elixir's `rem/2` to ensure
  correct positive modulo behaviour with negative range sizes.
- 38 ExUnit tests covering reference values, range bounds, reproducibility,
  composition, and edge cases (seed 0, single-value ranges, negative ranges).
  Coverage: 93.88% total (92.86% LCG, 93.75% Xorshift64, 94.74% PCG32,
  100% top-level module).
- `import Bitwise` used in all generator modules (not deprecated `use Bitwise`).
