# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `rng` crate: three deterministic PRNGs — a
  64-bit LCG (`rng_lcg`), Marsaglia's Xorshift64 (`rng_xorshift64`), and
  O'Neill's PCG32 (`rng_pcg32`).
- Each exposes `init`, `next_u32`, `next_u64`, `next_float` (double in [0,1)), and
  `next_int_in_range` (rejection-sampled, modulo-bias-free).
- All arithmetic is 32/64-bit unsigned — no `__int128` extension needed;
  reproduces the crate's reference values exactly. PCG rotate-right guarded
  against undefined shift-by-32.
- Tests pinning the reference values and checking determinism, seed divergence,
  the Xorshift64 zero-seed remap, and range/float bounds, under GCC and Clang via
  `iso-harness`.
