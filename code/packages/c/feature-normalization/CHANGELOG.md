# Changelog

All notable changes to the C `feature-normalization` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `feature-normalization` crate.
- `FnStandardScaler` (z-score) and `FnMinMaxScaler` (unit-range) with
  `fn_fit_*` / `fn_transform_*` / `fn_*_free`; matrices passed as flat
  row-major `double` arrays.
- Population standard deviation via a `<math.h>`-free Newton's-method square
  root; zero-spread columns map to `0.0`.
- `FnStatus` status-code API (`FN_OK` / `FN_ERR_EMPTY` / `FN_ERR_WIDTH_MISMATCH`
  / `FN_ERR_NOMEM`) in place of the Rust `Result<_, &'static str>`; allocations
  use `calloc` for the checked width multiply.
- 36 checks over both scalers (fit statistics, transform vectors from the Rust
  tests, constant-column and error cases), run under every available C compiler
  via the shared `iso-harness`.
