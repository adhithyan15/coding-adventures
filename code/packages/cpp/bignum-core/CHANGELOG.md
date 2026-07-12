# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-11

### Added

- Initial pure-ISO C++17 header-only port of the `BigInteger` core of the Rust
  `bignum-core` crate, in namespace `ca`: sign-magnitude arbitrary-precision
  integers over `std::vector<std::uint32_t>` limbs (64-bit accumulator, no
  128-bit integers).
- Value type with operator overloads (`+ - * / % -` and the six comparisons);
  `abs` / `neg` / `pow` / `try_pow` / `gcd`.
- Arithmetic: column add/sub, schoolbook multiply, Knuth Algorithm D `div_rem`
  (truncating toward zero); `to_str_radix` / `to_string` and `parse_radix`
  (radix 2–36).
- Errors as exceptions: `std::domain_error` (divide by zero),
  `ca::ParseBigIntError` (parse), `ca::PowTooLargeError` (try_pow guard).
- Tests via the shared `iso-harness` (GCC, Clang, MSVC) cross-checked against
  Python's arbitrary-precision integers, matching the Rust crate's oracle tests.
