# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Pure ISO C17 port of the Rust `polynomial` crate: coefficient-array polynomial
  arithmetic over doubles (little-endian coefficients).
- API: `poly_normalize`, `poly_degree`, `poly_add`, `poly_subtract`,
  `poly_multiply`, `poly_divmod` / `poly_divide` / `poly_modulo`, `poly_evaluate`
  (Horner), `poly_gcd` (Euclidean).
- No libm: absolute value is done manually and the zero threshold uses
  `DBL_EPSILON`. Caller-provided output buffers; only `poly_gcd` allocates
  (overflow-guarded) scratch.
- Tests with integer coefficients (exact results) covering the crate's
  identities — arithmetic, long-division reconstruction, evaluation, and GCD —
  under GCC and Clang via `iso-harness`.
