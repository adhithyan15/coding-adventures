# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `gf256` crate: Galois Field GF(2^8) arithmetic.
- Module-level operations on the default Reed-Solomon field (0x11D) via
  lazily-built log/antilog tables: `gf256_add`, `gf256_subtract`,
  `gf256_multiply`, `gf256_divide`, `gf256_power`, `gf256_inverse`.
- `gf256_field` — a field parameterised by any primitive polynomial (e.g. AES's
  0x11B) using table-free Russian-peasant multiplication, with matching
  add/subtract/multiply/divide/power/inverse.
- Division by zero and inverse of zero return 0 (the crate panics).
- Tests pinning known values in the Reed-Solomon and AES fields plus algebraic
  round-trips (`a·inverse(a)=1`, `divide(multiply(a,b),b)=a`) under GCC and Clang
  via `iso-harness`.
