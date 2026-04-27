# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-04-25

### Added

- `RandomGen` typeclass with five methods: `nextU32`, `nextU64`, `nextFloat`,
  `nextIntInRange` (default implementations for all but `nextU32`).
- `LCG` newtype — Linear Congruential Generator (Knuth 1948).
  State: `Word64`.  Output: upper 32 bits (`Word32`).
  Constants: multiplier = 6364136223846793005, increment = 1442695040888963407.
- `Xorshift64` newtype — Marsaglia (2003) XOR-shift generator.
  Three shifts (13, 7, 17).  Seed 0 replaced with 1.  Output: lower 32 bits.
- `PCG32` data type — Permuted Congruential Generator (O'Neill 2014).
  XSH RR output permutation.  InitSeq warm-up matches reference C library.
- 29 Hspec tests covering: known reference values (seed=1 vs Go reference),
  reproducibility, seed-0 edge cases, `nextU64` composition, unit-interval
  bounds for `nextFloat`, range bounds and coverage for `nextIntInRange`,
  float-mean statistical sanity, cross-generator independence.
- Corrected `.cabal` file: fixed corrupted `hs-source-dirs` line, added
  `hspec == 2.*` test dependency, added `other-modules: RngSpec`.
