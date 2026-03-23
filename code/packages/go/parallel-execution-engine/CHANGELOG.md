# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- **protocols.go**: ExecutionModel enum (SIMT, SIMD, Systolic, ScheduledMAC, VLIW),
  DivergenceInfo, DataflowInfo, EngineTrace with Format(), and
  ParallelExecutionEngine interface.
- **warp_engine.go**: SIMT warp execution engine (NVIDIA CUDA / ARM Mali style)
  with per-thread GPUCore instances, divergence stack, reconvergence detection,
  and configurable warp width.
- **wavefront_engine.go**: SIMD wavefront execution engine (AMD GCN/RDNA style)
  with VectorRegisterFile, ScalarRegisterFile, explicit EXEC mask, and
  one-PC-for-all-lanes semantics.
- **systolic_array.go**: Systolic dataflow execution engine (Google TPU style)
  with NxN PE grid, weight preloading, staggered input feeding, and
  RunMatmul convenience method.
- **mac_array_engine.go**: Compiler-scheduled MAC array engine (Apple ANE /
  Qualcomm Hexagon style) with input/weight/output buffers, 6-stage pipeline
  (LOAD_INPUT, LOAD_WEIGHTS, MAC, REDUCE, ACTIVATE, STORE_OUTPUT), and
  hardware activation functions (ReLU, Sigmoid, Tanh).
- **subslice_engine.go**: Intel Xe hybrid SIMD engine with multiple EUs,
  per-EU thread arbitration (round-robin), and SIMD8 lane execution.
- Full test suite with 95%+ coverage across all engines.
- Cross-engine interface compliance tests.
- Integration "programs" tests for realistic workloads.

### Architecture

- All 5 engines implement `ParallelExecutionEngine` interface for uniform driving.
- Ported faithfully from the Python reference implementation.
- Uses Go idioms: interfaces, exported PascalCase types, error returns, functional options.
