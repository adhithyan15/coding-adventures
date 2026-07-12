# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C17 port of the `BigInteger` core of the Rust `bignum-core`
  crate: sign-magnitude arbitrary-precision integers, little-endian base-2^32
  limbs, 32-bit limbs with a 64-bit accumulator (no 128-bit integers).
- Construction (`bigint_zero` / `one` / `from_i64` / `from_u64` / `clone`),
  queries (`is_zero` / `signum` / `num_limbs` / `bit_len` / `cmp`), and sign
  transforms (`abs` / `neg`).
- Arithmetic: `add` / `sub` (column methods), `mul` (schoolbook), `div_rem` /
  `div` / `rem` (Knuth Algorithm D, truncating toward zero), `pow`
  (exponentiation by squaring), `try_pow` (O(1) size guard), `gcd` (Euclid).
- Radix 2–36 `parse_radix` (typed `BigIntStatus` errors, never crashes) and
  `to_str_radix` / `to_string`.
- malloc-owned handles freed with `bigint_free`; overflow-guarded via checked
  `calloc` in multiply and a `BigIntStatus` throughout.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) cross-checked against
  Python's arbitrary-precision integers (factorials, 2^128, 7^99, div_rem, gcd,
  base-16/36), matching the Rust crate's oracle tests.
