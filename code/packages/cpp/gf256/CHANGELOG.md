# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `gf256` crate, in namespace
  `ca::gf256`: Galois Field GF(2^8) arithmetic.
- Free functions on the default Reed-Solomon field (0x11D) via log/antilog tables
  built once through a thread-safe function-local static: `add`, `subtract`,
  `multiply`, `divide`, `power`, `inverse`.
- `ca::gf256::Field` — parameterised by any primitive polynomial (e.g. AES's
  0x11B) using table-free Russian-peasant multiplication.
- Division by zero and inverse of zero return 0 (the crate panics).
- Tests pinning known values in the Reed-Solomon and AES fields plus algebraic
  round-trips, under GCC and Clang via `iso-harness`.
