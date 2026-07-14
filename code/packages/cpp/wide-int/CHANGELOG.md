# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Header-only ISO C++17 portable 128-bit integers (`ca::wide_int::u128` /
  `i128`) built from two `std::uint64_t` halves — no `__int128`, so identical
  under GCC, Clang, and MSVC (CCPP02 Phase 1, bucket A).
- Full operator set (`+ - * / % & | ^ ~ << >>`, comparisons), `divmod`, the
  exact widening `u128::mul_u64`, arithmetic `i128::operator>>`, and
  `to_string`/`to_hex`. Core ops are `constexpr` (compile-time capable).
- 1.4M checks (golden vectors + property sweeps + `static_assert` constexpr
  checks) under g++ + clang++ via the shared `iso-harness`. Verified under ASan +
  UBSan; algorithm cross-checked against native `__int128` via the C sibling.
