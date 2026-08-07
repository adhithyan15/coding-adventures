# Changelog

All notable changes to the C++ `feature-normalization` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `feature-normalization`
  crate (namespace `ca::feature_normalization`).
- `StandardScaler` (z-score) and `MinMaxScaler` (unit-range) structs with
  `fit_standard_scaler` / `transform_standard` / `fit_min_max_scaler` /
  `transform_min_max`, operating on `std::vector<std::vector<double>>` matrices.
- Population standard deviation via a `<cmath>`-free Newton's-method square
  root; zero-spread columns map to `0.0`.
- Validation throws `std::invalid_argument` (empty matrix, ragged rows, width
  mismatch) in place of the Rust `Result<_, &'static str>`.
- 26 checks over both scalers (fit statistics, transform vectors from the Rust
  tests, constant-column and error cases), run under every available C++
  compiler via the shared `iso-harness`.
