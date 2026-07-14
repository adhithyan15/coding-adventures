# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 from-scratch elementary functions (CCPP02 Phase 1, bucket A) —
  a libm-free replacement for the roots/exp/log/pow/hyperbolic functions the
  campaign's math ports need. Computed from only + - * /, comparisons, and
  IEEE-754 bit tricks (memcpy); identical under GCC, Clang, and MSVC.
- sqrt/cbrt/hypot; exp/expm1/log/log2/log10/log_base; pow (exact integer-exponent
  path); sinh/cosh/tanh; plus classification and exact fabs/copysign/floor/ceil/
  trunc/round/fmod/ldexp/frexp. Companion to the trig crate.
- Solid double precision (~1 ULP), reached by two-part ln2 argument reduction +
  short Taylor/atanh series + Newton steps with exact power-of-two reconstruction.
- 1.84M checks (golden constants + oracle-free algebraic identity sweeps) under
  gcc + clang via the shared iso-harness. Verified clean under ASan + UBSan;
  accuracy cross-checked against the platform libm over tens of millions of
  random inputs locally (the committed tests stay pure-ISO — this lane forbids
  libm).
