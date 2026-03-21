# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `protocols` module: `ParallelExecutionEngine` trait, `ExecutionModel` enum,
  `EngineTrace`, `DivergenceInfo`, and `DataflowInfo` types.
- `warp_engine` module: SIMT parallel execution (NVIDIA CUDA / ARM Mali style)
  with hardware divergence stack support.
- `wavefront_engine` module: SIMD parallel execution (AMD GCN/RDNA style) with
  explicit EXEC mask, vector register file, and scalar register file.
- `systolic_array` module: Dataflow execution (Google TPU style) with NxN PE
  grid and `run_matmul()` for complete matrix multiplication.
- `mac_array_engine` module: Compiler-scheduled MAC array execution (Apple ANE
  style) with ReLU, sigmoid, and tanh activation functions.
- `subslice_engine` module: Intel Xe hybrid SIMD execution with multiple EUs,
  per-EU hardware thread arbitration (round-robin), and SIMD8 lanes.
- Comprehensive unit tests in each module.
- Integration tests covering cross-engine comparison, per-thread data, EXEC
  mask masking, matrix multiplication, scheduled dot products, and trait
  compliance for all five engines.
