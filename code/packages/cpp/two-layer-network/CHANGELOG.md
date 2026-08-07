# Changelog

All notable changes to the C++ `two-layer-network` package are documented here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `two-layer-network` crate
  (namespace `ca::two_layer_network`).
- A one-hidden-layer network over `std::vector<std::vector<double>>` matrices:
  `Parameters` / `ForwardPass` / `TrainingStep` structs, `xor_warm_start_parameters`,
  `forward`, and `train_one_epoch` (both layers' gradients + next parameters).
- Full-batch mean-squared-error backpropagation through both layers (output
  deltas, then hidden deltas via the transposed output weights); Linear and
  Sigmoid activations, the sigmoid from a libm-free `e^x`.
- Shape errors throw `std::invalid_argument` (empty / ragged / misaligned) in
  place of the Rust `Result<_, String>`.
- 22 checks including the Rust crate's XOR forward-pass expectations and
  two-layer gradient-shape checks, run under every available C++ compiler via
  the shared `iso-harness`.
