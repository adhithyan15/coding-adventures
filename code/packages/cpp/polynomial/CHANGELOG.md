# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C++17, header-only port of the Rust `polynomial` crate, in namespace
  `ca::polynomial`: coefficient-array polynomial arithmetic over doubles
  (`poly` = `std::vector<double>`, little-endian).
- `normalize`, `degree`, `zero`, `one`, `add`, `subtract`, `multiply`, `divmod`
  (→ `std::pair<poly, poly>`), `divide`, `modulo`, `evaluate` (Horner), `gcd`.
- Division by the zero polynomial throws `std::invalid_argument`. No libm
  (manual absolute value, `DBL_EPSILON` threshold).
- Tests with integer coefficients covering the crate's identities under GCC and
  Clang via `iso-harness`.
