# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- Initial TypeScript implementation of compute-unit (Layer 7).
- `protocols.ts`: Architecture enum, WarpState enum, SchedulingPolicy enum, WorkItem, ComputeUnitTrace, SharedMemory with bank conflict detection, ComputeUnit interface.
- `streaming-multiprocessor.ts`: NVIDIA SM simulator with configurable warp schedulers (GTO, round-robin, LRR, oldest-first, greedy), register file tracking, shared memory, occupancy calculation, and resource error handling.
- `amd-compute-unit.ts`: AMD CU (GCN/RDNA) simulator with SIMD units, wavefront scheduling, LDS (Local Data Share), and VGPR tracking.
- `matrix-multiply-unit.ts`: Google TPU MXU simulator with systolic array integration, tiling, and vector unit activation functions (relu, sigmoid, tanh).
- `xe-core.ts`: Intel Xe Core simulator with SubsliceEngine integration, SLM (Shared Local Memory), and EU thread dispatching.
- `neural-engine-core.ts`: Apple ANE Core simulator with MAC array integration, DMA simulation, and activation pipeline.
- Full test suite with per-architecture tests and cross-architecture validation.
