# Changelog

## 0.1.0 — 2026-04-08

Initial release.

### Added
- `fnv1a_32`: FNV-1a 32-bit hash with module-level `FNV32_OFFSET_BASIS` and `FNV32_PRIME` constants
- `fnv1a_64`: FNV-1a 64-bit hash with module-level `FNV64_OFFSET_BASIS` and `FNV64_PRIME` constants
- `djb2`: Dan Bernstein's shift-and-add hash (64-bit output, no truncation)
- `polynomial_rolling`: Polynomial rolling hash over configurable base and Mersenne prime modulus
- `murmur3_32`: MurmurHash3 32-bit with seed parameter, implementing full 4-byte block loop, tail handler, and fmix32 finalizer
- `avalanche_score`: Measures fraction of output bits changed per input bit flip
- `distribution_test`: Chi-squared uniformity test across buckets
- All functions accept `bytes | str` (str UTF-8 encoded automatically)
- Literate comments explaining the bit-level mathematics of each algorithm
- 95%+ test coverage with known-answer vectors from official test suites
