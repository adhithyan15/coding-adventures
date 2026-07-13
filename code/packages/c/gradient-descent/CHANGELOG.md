# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-07-13

### Added

- Pure ISO C17 port of the Rust `gradient-descent` crate: one step of
  stochastic gradient descent, `out[i] = weights[i] - learning_rate *
  gradients[i]`.
- `gd_sgd(weights, gradients, n, learning_rate, out)` — an allocation-free
  update writing into a caller-owned buffer (which may alias `weights` for an
  in-place step); returns `GD_ERR_LENGTH` on an empty vector.
- 16 checks (the crate's core vector and empty-input error, plus zero-gradient,
  larger-step, negative-gradient, and in-place-aliasing cases) run under every
  ISO C compiler via the shared `iso-harness`. Verified clean under ASan + UBSan.
