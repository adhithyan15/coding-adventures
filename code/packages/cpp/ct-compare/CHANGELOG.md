# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `ct-compare` crate, in namespace
  `ca::ct_compare`: constant-time byte comparison with no data-dependent
  branches.
- API: `ct_eq` (equal length AND bytes; length public), `ct_eq_fixed<N>` (over a
  `std::array`), `ct_select_bytes` (branchless select, throws
  `std::invalid_argument` on length mismatch), `ct_eq_u64` (constant-time 64-bit
  equality).
- Rust's `core::hint::black_box` optimiser barrier is realised as a read through
  a `volatile` object — the pure-ISO way to stop the compiler folding the loop
  back into an early exit.
- Tests covering equality, first/last-byte differences, empty inputs, the
  length-mismatch throw, and the full single-bit-flip sweep (every bit at every
  byte position, and every bit of a u64) under GCC and Clang via `iso-harness`.
