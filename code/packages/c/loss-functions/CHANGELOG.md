# Changelog

All notable changes to the C `loss-functions` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `loss-functions` crate.
- Four losses plus their gradients: `loss_mse`, `loss_mae`, `loss_bce`,
  `loss_cce` (scalar, via `*out`) and `loss_*_derivative` (per-element, written
  into a caller-provided array), plus the `LOSS_EPSILON` clamp constant.
- libm-free natural log (range reduction to `[1, 2)` + an atanh series) for
  cross-entropy; predictions clamped to `[1e-7, 1 - 1e-7]` before the log.
- `LossStatus` status-code API (`LOSS_OK` / `LOSS_ERR_LENGTH`) in place of the
  Rust `Result<_, &'static str>`; each array carries its own length so an
  unequal-length or empty call is rejected, matching Rust.
- 33 checks against the Rust crate's reference values (1e-6 tolerance), run
  under every available C compiler via the shared `iso-harness`.
