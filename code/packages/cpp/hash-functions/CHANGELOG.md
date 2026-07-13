# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `hash-functions` crate: the
  non-cryptographic "DT17" hash family in namespace `ca::hash_functions`.
- Free functions: `fnv1a_32`, `fnv1a_64`, `djb2`, `polynomial_rolling` /
  `_with_params`, `murmur3_32` / `_with_seed`, `siphash_2_4`, and
  `string_view` helpers `hash_str_fnv1a_32` / `hash_str_siphash`. Named
  `constexpr` constants.
- `HashFunction` abstract base with concrete `final` structs (`Fnv1a32`,
  `Fnv1a64`, `Djb2`, `PolynomialRolling`, `Murmur3_32`, `SipHash24`), each with
  `hash()` / `output_bits()` — the Rust trait, usable polymorphically.
- Analysis helpers as function templates generic over the hash callable:
  `avalanche_score` (caller-supplied fill callable; Rust's `getrandom` OS
  entropy has no pure-ISO equivalent) and `distribution_test` (chi-square).
- Rust's `u128` intermediate in polynomial rolling is replaced by an exact,
  overflow-safe `mulmod`/`addmod` (no 128-bit type).
- 48 checks mirroring the crate's known-answer vectors, run under every ISO C++
  compiler via the shared `iso-harness`; also clean under ASan + UBSan.
