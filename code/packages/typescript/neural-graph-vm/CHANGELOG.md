# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- Added scalar `constant` node compilation through `LOAD_CONST`.
- Added `runNeuralBytecodeForwardWithTrace` for instruction-level VM traces.
- Added explicit `WebGpuMatrixBackend.destroy()` device cleanup for bounded
  browser probes.

### Changed (transparent via dep)

- **MX08 Phase 3 (verification)**: on Node, the `matrix` package's
  `CpuMatrixBackend` now delegates to the Rust `matrix-cpu` executor
  via `@coding-adventures/matrix-rust-napi` (MX08 Phase 2, PR #3571).
  `neural-graph-vm`'s `TypeScriptMatrixBackend` reference adapter picks
  up the speedup transparently — **no source change in this package**.
  Verified: all 16 tests pass after the MX08 Phase 2 refactor.  Browser
  builds keep the pure-TS implementation per the new package.json
  `exports` conditional.
- Added XOR bytecode coverage through the neural-network helper graph.
- Added NN01 matrix plan lowering and a swappable `MatrixBackend` interface.
- Added `TypeScriptMatrixBackend` as the reference CPU adapter for the existing
  `matrix` package.

## [0.1.0] - 2026-04-29

### Added

- Added a reference Neural Graph VM package.
- Added a compiler that lowers `@coding-adventures/neural-network` models to
  NN00 forward bytecode.
- Added a scalar bytecode interpreter for reference execution and smoke tests.
