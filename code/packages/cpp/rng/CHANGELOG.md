# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `rng` crate, in namespace
  `ca::rng`: three deterministic PRNGs — `Lcg`, `Xorshift64`, and `Pcg32`.
- Each provides `next_u32`, `next_u64`, `next_float` (double in [0,1)), and
  `next_int_in_range` (via a shared templated, modulo-bias-free helper).
- All arithmetic is 32/64-bit unsigned (no `__int128`); reproduces the crate's
  reference values exactly.
- Tests pinning the reference values and checking determinism, seed divergence,
  the zero-seed remap, and range/float bounds, under GCC and Clang via
  `iso-harness`.
