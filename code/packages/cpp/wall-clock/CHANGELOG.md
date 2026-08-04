# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C++17 header-only port of the pure core of the Rust
  `wall-clock` crate, in namespace `ca::wallclock`.
- `Instant` (f64 seconds since the epoch) with `constexpr` `from_secs` /
  `add_secs` / `duration_since` and the six comparison operators; an inline
  `constexpr EPOCH`.
- An abstract `Clock` base (virtual `now()`) — the analog of Rust's `dyn Clock`
  — with `FixedClock` and `AdvancingClock` (the latter's `now()` is `const` with
  a `mutable` state, matching Rust's `Cell<f64>`).
- The Rust `SystemClock` (behind a `std::time` feature flag) is omitted from this
  pure port.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): Instant arithmetic /
  ordering / constexpr use, the fixed and advancing clocks, and polymorphic
  injection through the Clock base — mirroring the Rust crate's tests.
