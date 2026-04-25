# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `LCG` struct — Linear Congruential Generator (Knuth 1948).
  State: `UInt64`.  Output: upper 32 bits (`UInt32`).
  Constants: multiplier = 6364136223846793005, increment = 1442695040888963407.
- `Xorshift64` struct — Marsaglia (2003) XOR-shift generator.
  Three shifts (13, 7, 17).  Seed 0 replaced with 1.  Output: lower 32 bits.
- `PCG32` struct — Permuted Congruential Generator (O'Neill 2014).
  XSH RR output permutation using `&>>` / `&<<` overflow operators.
  InitSeq warm-up matches reference C library.
- All three types expose: `init(seed:)`, `nextU32()`, `nextU64()`,
  `nextFloat()`, `nextIntInRange(min:max:)`.
- Methods are `mutating` (value-type semantics): copying a generator gives
  an independent reproducible stream.
- `nextIntInRange` uses rejection sampling to eliminate modulo bias.
- 29 XCTest tests covering: known reference values (seed=1 vs Go reference),
  reproducibility, seed-0 edge cases, `nextU64` hi/lo composition, unit-interval
  bounds for `nextFloat`, range bounds and coverage for `nextIntInRange`,
  float-mean statistical sanity, cross-generator independence, struct
  value-type copy semantics.
