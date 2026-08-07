# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the pure core of the Rust `wall-clock` crate: an
  injectable clock abstraction.
- `WcInstant` (f64 seconds since the epoch) with `from_secs` / `epoch` /
  `add_secs` / `duration_since` and the f64 comparison predicates.
- `WcClock` (a `now` function + `void *` context — the C analog of a `dyn Clock`
  trait object) with `wc_clock_now`.
- `WcFixedClock` (`new` / `epoch` / `now` / `as_clock`) and `WcAdvancingClock`
  (`new` / `now` advancing its state / `as_clock`). All plain value types — no
  allocation.
- The Rust `SystemClock` (host-clock-reading, behind a feature flag) is omitted
  from this pure port, matching the WASM/no-`std::time` build.
- Tests via the shared `iso-harness` (GCC, Clang, MSVC): Instant arithmetic and
  ordering, the fixed and advancing clocks, and polymorphic injection through the
  WcClock trait object — mirroring the Rust crate's tests.
