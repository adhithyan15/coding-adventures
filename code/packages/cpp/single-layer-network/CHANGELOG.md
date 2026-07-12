# Changelog

All notable changes to the C++ `single-layer-network` package are documented
here.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.0.0/).

## [0.1.0] - 2026-07-12

### Added

- Initial header-only pure-ISO C++17 port of the Rust `single-layer-network`
  crate (namespace `ca::single_layer_network`).
- A single dense layer over `std::vector<std::vector<double>>` matrices:
  `predict_with_parameters`, `train_one_epoch_with_matrices`, the
  `SingleLayerNetwork` class (`predict` / `fit`), and `fit_single_layer_network`.
- Full-batch mean-squared-error gradient descent; Linear and Sigmoid
  activations, the sigmoid computed in the numerically stable direction from a
  libm-free `e^x` (with overflow/underflow/NaN guards).
- Shape errors throw `std::invalid_argument` (empty / ragged / mismatched) in
  place of the Rust `Result<_, String>`.
- 27 checks including the Rust crate's exact-gradient vector and loss-decay
  training run, plus sigmoid saturation and shape errors, run under every
  available C++ compiler via the shared `iso-harness`.
