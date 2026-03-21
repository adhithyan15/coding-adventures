# Changelog

## 0.1.0 (2026-03-19)

### Added

- Initial TypeScript implementation ported from Python device-simulator package
- **NvidiaGPU**: NVIDIA GPU device with GigaThread Engine, SMs, L2 cache, HBM
- **AmdGPU**: AMD GPU device with Shader Engines, Infinity Cache, Command Processor
- **GoogleTPU**: Google TPU device with Scalar/Vector/MXU pipeline, HBM
- **IntelGPU**: Intel GPU device with Xe-Slices, L2 cache, Command Streamer
- **AppleANE**: Apple Neural Engine with unified memory (zero-copy), DMA, schedule replay
- **SimpleGlobalMemory**: Global memory simulator with coalescing, partitioning, allocation
- **GPUWorkDistributor**: Block distributor with round-robin, fill-first, and least-loaded policies
- **TPUSequencer**: Three-stage pipeline (Scalar -> MXU -> Vector) for tile operations
- **ANEScheduleReplayer**: Compiler-generated schedule replay for ANE workloads
- **KernelDescriptor**: Unified kernel descriptor for GPU programs and dataflow operations
- **DeviceTrace/DeviceStats**: Cycle-by-cycle tracing and aggregate statistics
- Full test suite with >80% coverage across all device types
- Cross-device tests verifying uniform interface behavior
