# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 portable 128-bit integers (`wi_u128` / `wi_i128`) built from two
  `uint64_t` halves — no `__int128`, so identical under GCC, Clang, and MSVC
  (CCPP02 Phase 1, bucket A). The substrate for the campaign's `u128`-using
  crates.
- Arithmetic (add/sub/mul mod 2^128, exact widening `wi_mul_u64`, binary
  long-division `wi_u128_divmod` / signed `wi_i128_divmod` truncating toward
  zero), bitwise + total shifts (logical `shl`/`shr`, arithmetic `sar`), signed
  and unsigned comparison, and decimal/hex formatting.
- 1.8M checks (golden vectors + algebraic property sweeps: `a+b-b == a`,
  `q*d+r == n` with `r < d`, commutativity, shift/mul consistency) under gcc +
  clang via the shared `iso-harness`. Verified under ASan + UBSan and
  cross-checked against native `__int128` over 5M random unsigned+signed
  operations as a local oracle (the committed tests stay pure-ISO).
