# Changelog

All notable changes to the `compute-unit` Go package will be documented in this file.

## [0.1.0] - 2026-04-02

### Added
- `protocols.go`: Core interfaces and types — `ComputeUnit` interface, `ComputeUnitTrace`, `WorkItem`, `WorkGroup`, `Wavefront`, `SIMDLane`, `SharedMemory`, `RegisterBank`, `SchedulingPolicy`, `ActivationFn`.
- `amd_compute_unit.go`: `AMDComputeUnit` — AMD GCN/RDNA-style compute unit with wavefront-based SIMD execution. Public API: `DefaultAMDCUConfig()`, `NewAMDComputeUnit()`, `Name()`, `Arch()`, `Idle()`, `Occupancy()`, `Config()`, `LDS()`, `WavefrontSlots()`, `Dispatch()`, `Step()`, `Run()`, `Reset()`, `String()`.
- `streaming_multiprocessor.go`: `StreamingMultiprocessor` — NVIDIA-style SM with warp scheduling. Includes `WarpScheduler` with round-robin, GTO, and oldest-first policies. Public API: `DefaultSMConfig()`, `NewWarpScheduler()`, `AddWarp()`, `TickStalls()`, `PickWarp()`, `MarkIssued()`, `ResetScheduler()`, `NewStreamingMultiprocessor()`, and all SM public methods.
- `matrix_multiply_unit.go`: `MatrixMultiplyUnit` — TPU/Apple ANE-style systolic array MXU. Public API: `DefaultMXUConfig()`, `NewMatrixMultiplyUnit()`, `Name()`, `Arch()`, `Idle()`, `Config()`, `Result()`, `SystolicArray()`, `Dispatch()`, `Step()`, `Run()`, `RunMatmul()`, `Reset()`, `String()`.
- `neural_engine_core.go`: `NeuralEngineCore` — Apple Neural Engine-style inference core with MAC engine. Public API: `DefaultANECoreConfig()`, `NewNeuralEngineCore()`, `Name()`, `Arch()`, `Idle()`, `Config()`, `ResultMatrix()`, `MACEngine()`, `Dispatch()`, `Step()`, `Run()`, `RunInference()`, `Reset()`, `String()`.
- `xe_core.go`: `XeCore` — Intel Xe GPU compute core with vector and matrix engines. Public API: `DefaultXeCoreConfig()`, `NewXeCore()`, `Name()`, `Arch()`, `Idle()`, `Config()`, `SLM()`, `Engine()`, `Dispatch()`, `Step()`, `Run()`, `Reset()`, `String()`.
- All public functions and methods wrapped with the Operations system (`StartNew`) for unified observability, capability enforcement, and telemetry tracing.
