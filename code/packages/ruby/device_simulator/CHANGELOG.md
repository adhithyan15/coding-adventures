# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- `SimpleGlobalMemory` -- sparse VRAM/HBM simulator with coalescing, partitioning, host transfers, and unified memory support.
- `GPUWorkDistributor` -- distributes thread blocks to compute units with round-robin, fill-first, and least-loaded policies.
- `TPUSequencer` -- orchestrates Scalar/MXU/Vector three-stage pipeline with tiling.
- `ANEScheduleReplayer` -- replays compiler-generated execution schedules for Apple Neural Engine.
- `NvidiaGPU` -- full NVIDIA GPU simulator with SMs, L2 cache, HBM, and GigaThread Engine.
- `AmdGPU` -- AMD GPU simulator with CUs grouped into Shader Engines, Infinity Cache, and Command Processor.
- `GoogleTPU` -- Google TPU simulator with MXU, sequencer pipeline, and HBM.
- `IntelGPU` -- Intel GPU simulator with Xe-Cores grouped into Xe-Slices, L2 cache, and Command Streamer.
- `AppleANE` -- Apple Neural Engine simulator with unified memory (zero-copy), SRAM, DMA, and compiler-driven scheduling.
- `KernelDescriptor` -- unified descriptor for GPU-style (program + grid/block) and dataflow-style (operation + matrices) workloads.
- `DeviceConfig` and vendor-specific configs (`AmdGPUConfig`, `IntelGPUConfig`, `TPUConfig`, `ANEConfig`).
- `DeviceTrace` -- cycle-by-cycle device-wide trace with formatting.
- `DeviceStats` -- aggregate performance statistics.
- `GlobalMemoryStats` -- memory access pattern tracking with coalescing efficiency.
- `MemoryTransaction` -- coalesced memory transaction representation.
- Comprehensive test suite: global memory, work distributors, all five device types, and cross-device compatibility tests.
