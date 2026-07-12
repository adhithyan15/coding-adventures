# Changelog

All notable changes to the C `single-layer-network` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `single-layer-network` crate.
- A single dense layer over flat row-major `double` matrices: `sln_predict`,
  `sln_train_one_epoch` (full `SlnTrainingStep` record), and an `SlnNetwork`
  with `sln_network_init` / `sln_network_predict` / `sln_network_fit` /
  `sln_network_free` plus `sln_history_free`.
- Full-batch mean-squared-error gradient descent; Linear and Sigmoid
  activations, the sigmoid computed in the numerically stable direction from a
  libm-free `e^x` (with overflow/underflow/NaN guards).
- `SlnStatus` status-code API (`SLN_OK` / `SLN_ERR_SHAPE` / `SLN_ERR_NOMEM`) in
  place of the Rust `Result<_, String>`; every matrix allocation guards its
  `rows*cols` multiply against `size_t` overflow.
- 31 checks including the Rust crate's exact-gradient vector and loss-decay
  training run, plus sigmoid saturation and shape errors, run under every
  available C compiler via the shared `iso-harness`.
