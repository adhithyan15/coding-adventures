# Changelog

All notable changes to this package will be documented in this file.

## [0.1.0] - 2026-03-19

### Added
- Initial Go implementation of the device-simulator package (Layer 6).
- Ported from the Python implementation.

#### Core Types (`protocols.go`)
- `MemoryTransaction` — coalesced memory access record with thread mask.
- `GlobalMemoryStats` — tracks coalescing efficiency, partition conflicts, and host transfer metrics.
- `KernelDescriptor` — unified descriptor for GPU kernels and dataflow operations.
- `DeviceConfig` — full device specification with memory hierarchy parameters.
- Vendor-specific configs: `AmdGPUConfig`, `IntelGPUConfig`, `TPUConfig`, `ANEConfig`.
- Default configs for all five architectures modeling real hardware.
- `DeviceTrace` — cycle-by-cycle device-wide trace with formatting.
- `DeviceStats` — aggregate statistics across all compute units and memory.
- `AcceleratorDevice` interface — unified interface for all device types.

#### Global Memory (`global_memory.go`)
- `SimpleGlobalMemory` — sparse VRAM/HBM simulator with bump allocator.
- Memory coalescing algorithm that merges per-thread addresses into transactions.
- Partition conflict detection across memory channels.
- Host transfer simulation with unified memory (zero-copy) support.

#### Work Distributors (`work_distributor.go`)
- `GPUWorkDistributor` — round-robin, fill-first, and least-loaded policies.
- `TPUSequencer` — three-stage Scalar→MXU→Vector pipeline with tile decomposition.
- `ANEScheduleReplayer` — compiler-generated schedule replay with DMA transfers.

#### Device Simulators
- `NvidiaGPU` — SMs + GigaThread Engine + L2 Cache + HBM.
- `AmdGPU` — CUs in Shader Engines + Infinity Cache + GDDR6.
- `GoogleTPU` — MXU + Scalar/Vector pipeline + HBM.
- `IntelGPU` — Xe-Cores in Xe-Slices + L2 Cache + GDDR6.
- `AppleANE` — NE Cores + unified memory (zero-copy) + DMA schedule replay.

#### Testing
- 97.8% test coverage across all files.
- Cross-architecture tests verifying interface compliance and shared behavior.
- Per-device tests for creation, memory operations, kernel execution, stats, and reset.
