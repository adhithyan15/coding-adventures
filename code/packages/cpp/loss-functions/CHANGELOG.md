# Changelog

All notable changes to the C++ `loss-functions` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `loss-functions` crate
  (namespace `ca::loss_functions`).
- Four losses plus their gradients: `mse`, `mae`, `bce`, `cce` (returning
  `double`) and `*_derivative` (returning `std::vector<double>`), plus the
  `EPSILON` clamp constant.
- libm-free natural log (range reduction to `[1, 2)` + an atanh series) for
  cross-entropy; predictions clamped to `[1e-7, 1 - 1e-7]` before the log.
- Mismatched-length or empty inputs throw `std::invalid_argument` in place of
  the Rust `Result<_, &'static str>`.
- 20 checks against the Rust crate's reference values (1e-6 tolerance), run
  under every available C++ compiler via the shared `iso-harness`.
