# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Header-only ISO C++17 from-scratch elementary functions in namespace
  ca::float_math (CCPP02 Phase 1, bucket A) — a libm-free replacement for the
  roots/exp/log/pow/hyperbolic functions the campaign's math ports need.
- sqrt/cbrt/hypot; exp/expm1/log/log2/log10/log_base; pow; sinh/cosh/tanh; plus
  classification, exact rounding/remainder helpers, and constexpr constants.
  Computed from only + - * /, comparisons, and IEEE-754 bit tricks (std::memcpy);
  identical under GCC, Clang, and MSVC.
- Solid double precision (~1 ULP). 1.64M checks (golden constants + oracle-free
  identity sweeps) under g++ + clang++ via the shared iso-harness. Verified clean
  under ASan + UBSan; algorithm cross-checked against libm via the C sibling.
