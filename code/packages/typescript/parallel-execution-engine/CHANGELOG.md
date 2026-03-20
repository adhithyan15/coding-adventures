# Changelog

## 0.1.0 (2026-03-19)

### Added
- Initial TypeScript implementation of parallel execution engine (Layer 8)
- `WarpEngine` -- SIMT execution (NVIDIA CUDA / ARM Mali style)
  - Per-thread registers, active masks, divergence stack
  - Support for branch divergence and reconvergence
- `WavefrontEngine` -- SIMD execution (AMD GCN/RDNA style)
  - Vector register file (VGPRs), scalar register file (SGPRs)
  - Explicit EXEC mask control
- `SystolicArray` -- Dataflow execution (Google TPU style)
  - NxN PE grid with multiply-accumulate units
  - Weight preloading, staggered input feeding
  - Matrix multiplication convenience method
- `MACArrayEngine` -- Compiler-scheduled MAC execution (NPU style)
  - Static schedule-driven execution
  - Activation functions: ReLU, sigmoid, tanh
  - Input/weight/output buffers
- `SubsliceEngine` -- Intel Xe hybrid SIMD execution
  - Multiple EUs with hardware thread arbitration
  - SIMD8 lane processing per thread
- Unified `EngineTrace` interface across all engines
- `ExecutionModel` enum (SIMT, SIMD, SYSTOLIC, SCHEDULED_MAC, VLIW)
- `DivergenceInfo` and `DataflowInfo` trace extensions
- Cross-engine tests verifying numerical equivalence
