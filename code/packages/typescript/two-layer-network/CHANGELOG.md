# Changelog

## Unreleased

- **MX08 Phase 3 (verification)**: on Node, the `matrix` package's
  `CpuMatrixBackend` now delegates to the Rust `matrix-cpu` executor
  via `@coding-adventures/matrix-rust-napi` (MX08 Phase 2, PR #3571).
  This package's two-layer training loop and XOR demo pick up the
  speedup transparently — **no source change in this package**.  Test
  re-run on this branch hits a pre-existing vitest/vite ESM/CJS
  version-conflict that also occurs on `main` (unrelated to MX08);
  MX08 Phase 2's parity tests prove numerical equivalence within f32
  tolerance for every op this package uses.  Browser builds keep the
  pure-TS implementation.

## 0.1.0

- Added a two-layer fully connected network with backpropagation.
- Added sigmoid, tanh, ReLU, and linear activations.
- Added deterministic initialization helpers and XOR-oriented tests.
