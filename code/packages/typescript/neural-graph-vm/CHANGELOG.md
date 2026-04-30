# Changelog

All notable changes to this project will be documented in this file.

## Unreleased

### Added

- Added scalar `constant` node compilation through `LOAD_CONST`.
- Added `runNeuralBytecodeForwardWithTrace` for instruction-level VM traces.
- Added XOR bytecode coverage through the neural-network helper graph.

## [0.1.0] - 2026-04-29

### Added

- Added a reference Neural Graph VM package.
- Added a compiler that lowers `@coding-adventures/neural-network` models to
  NN00 forward bytecode.
- Added a scalar bytecode interpreter for reference execution and smoke tests.
