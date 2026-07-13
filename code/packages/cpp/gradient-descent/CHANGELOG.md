# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C++17, header-only port of the Rust `gradient-descent` crate in
  namespace `ca::gradient_descent`: one step of stochastic gradient descent,
  `weights[i] - learning_rate * gradients[i]`.
- `sgd(weights, gradients, learning_rate)` returning the updated weight vector;
  throws `GradientDescentError` (a `std::invalid_argument`) when the vectors
  differ in length or are empty, mirroring the Rust crate's `Result` error.
- 11 checks (the crate's core vector and error cases, plus zero-gradient,
  larger-step, and negative-gradient cases) run under every ISO C++ compiler via
  the shared `iso-harness`. Verified clean under ASan + UBSan.
