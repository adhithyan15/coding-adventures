# Changelog

All notable changes to the C `two-layer-network` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial pure-ISO C17 port of the Rust `two-layer-network` crate.
- A one-hidden-layer network over flat row-major `double` matrices:
  `TlnParameters` (`tln_parameters_init` / `tln_xor_warm_start_parameters` /
  `tln_parameters_free`), `tln_forward` (`TlnForwardPass`), and
  `tln_train_one_epoch` (`TlnTrainingStep` with both layers' gradients and the
  next parameters).
- Full-batch mean-squared-error backpropagation through both layers (output
  deltas, then hidden deltas via the transposed output weights); Linear and
  Sigmoid activations, the sigmoid from a libm-free `e^x`.
- `TlnStatus` status-code API (`TLN_OK` / `TLN_ERR_SHAPE` / `TLN_ERR_NOMEM`) in
  place of the Rust `Result<_, String>`; every matrix allocation guards its
  `rows*cols` multiply against `size_t` overflow, with a single cleanup path
  freeing all intermediates on any failure.
- 36 checks including the Rust crate's XOR forward-pass expectations and
  two-layer gradient-shape checks, run under every available C compiler via the
  shared `iso-harness`.
