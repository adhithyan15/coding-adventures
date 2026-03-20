# Changelog

## 0.1.0 — 2026-03-19

### Added

- **protocols.py**: ComputeUnit protocol, Architecture enum, WorkItem dataclass, WarpState enum, SchedulingPolicy enum, ComputeUnitTrace dataclass, SharedMemory class with bank conflict detection.
- **streaming_multiprocessor.py**: NVIDIA SM simulator with SMConfig, WarpSlot, WarpScheduler (GTO, ROUND_ROBIN, LRR, OLDEST_FIRST, GREEDY policies), ResourceError, thread block decomposition, occupancy calculation, memory stall simulation.
- **amd_compute_unit.py**: AMD CU (GCN/RDNA) simulator with AMDCUConfig, WavefrontSlot, SIMD unit assignment, LDS support, LRR scheduling.
- **matrix_multiply_unit.py**: Google TPU MXU simulator with MXUConfig, systolic array tiling, vector unit activation functions (ReLU, sigmoid, tanh).
- **xe_core.py**: Intel Xe Core simulator with XeCoreConfig, SubsliceEngine integration, SLM support, thread dispatcher.
- **neural_engine_core.py**: Apple ANE Core simulator with ANECoreConfig, MACArrayEngine integration, DMA simulation, activation pipeline.
- **test_protocols.py**: Tests for all shared types, SharedMemory bank conflicts, and ComputeUnitTrace formatting.
- **test_streaming_multiprocessor.py**: Tests for SM dispatch, scheduling policies, occupancy, resource errors, memory stalls.
- **test_amd_compute_unit.py**: Tests for AMD CU dispatch, wavefront scheduling, LDS access, resource errors.
- **test_matrix_multiply_unit.py**: Tests for MXU matmul correctness, activation functions, tiling.
- **test_xe_core.py**: Tests for Xe Core dispatch, SLM access, trace correctness.
- **test_neural_engine_core.py**: Tests for ANE inference, activation functions, matmul correctness.
- **test_cross_architecture.py**: Same matmul across all 5 architectures, architecture identity verification.
