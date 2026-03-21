# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial release of the compute unit package (Layer 7).
- **Protocols**: Architecture symbols, WarpState symbols, SchedulingPolicy symbols, WorkItem, ComputeUnitTrace, SharedMemory with bank conflict detection, ResourceError.
- **StreamingMultiprocessor** (NVIDIA SM): 4 warp schedulers, GTO/round-robin/LRR/oldest-first scheduling, occupancy calculation, register file partitioning, shared memory management.
- **AMDComputeUnit** (AMD CU): 4 SIMD units, wavefront scheduling with LRR, LDS (Local Data Share), scalar unit tracking.
- **MatrixMultiplyUnit** (Google TPU MXU): Systolic array-based matmul, tiling support, vector unit with activation functions (ReLU, sigmoid, tanh).
- **XeCore** (Intel Xe Core): EU + thread hierarchy via SubsliceEngine, SLM, thread dispatcher.
- **NeuralEngineCore** (Apple ANE): MAC array-based inference, DMA simulation, activation pipeline.
- Full test suite covering all five compute units with >80% coverage.
