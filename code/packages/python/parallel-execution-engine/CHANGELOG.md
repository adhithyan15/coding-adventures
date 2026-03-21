# Changelog

## [0.1.0] - 2026-03-19

### Added
- `protocols.py`: `ParallelExecutionEngine` protocol, `ExecutionModel` enum, `EngineTrace`, `DivergenceInfo`, `DataflowInfo` dataclasses
- `warp_engine.py`: SIMT warp engine (NVIDIA/ARM Mali style) with divergence stack handling
- `wavefront_engine.py`: SIMD wavefront engine (AMD GCN/RDNA style) with explicit EXEC mask
- `systolic_array.py`: Systolic dataflow engine (Google TPU style) with staggered input timing
- `mac_array_engine.py`: Compiler-scheduled MAC array engine (NPU style) with activation functions
- `subslice_engine.py`: Intel Xe hybrid SIMD engine with multi-threaded EUs and thread arbitration
- Full test suite covering all engines, protocols, and cross-engine verification
