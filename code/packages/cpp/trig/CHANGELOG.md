# Changelog

All notable changes to the C++ `trig` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `trig` crate (namespace
  `ca::trig`) — trigonometric functions from first principles, no `<cmath>`.
- `sin` / `cos` (20-term Maclaurin series with range reduction into `[-pi, pi]`),
  `tan` (sin/cos with pole saturation), `atan` / `atan2` (Taylor series with
  two-layer range reduction), `radians` / `degrees`, and `PI` / `TWO_PI` /
  `HALF_PI` constants.
- `sqrt` via Newton's method, throwing `std::domain_error` on negative input in
  place of the Rust panic.
- libm-free `detail` helpers for absolute value, truncation, and floating-point
  remainder so the header has zero external dependencies.
- 45 checks over sin/cos/tan/sqrt/atan/atan2/conversions (including non-finite
  / NaN inputs), run under every available C++ compiler via the shared
  `iso-harness`.
- Range-reduction truncation guards NaN out of the `double`->`long long` cast
  (converting NaN to an integer is undefined behavior).
