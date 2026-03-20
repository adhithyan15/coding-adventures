# Changelog

All notable changes to the `device-simulator` crate will be documented in this file.

## [0.1.0] - 2026-03-19

### Added

- **protocols.rs**: Shared types for all device simulators -- `MemoryTransaction`, `KernelDescriptor`, `DeviceConfig`, `DeviceTrace`, `DeviceStats`, and the `AcceleratorDevice` trait.
- **global_memory.rs**: Sparse global memory (VRAM/HBM) simulator with allocation, read/write, host transfers (with unified memory support), coalescing, partition conflict detection, and statistics tracking.
- **work_distributor.rs**: Three work distribution strategies:
  - `GPUWorkDistributor` -- round-robin, fill-first, and least-loaded policies for NVIDIA/AMD/Intel GPUs.
  - `TPUSequencer` -- Scalar/MXU/Vector three-stage pipeline for Google TPU.
  - `ANEScheduleReplayer` -- compiler-generated schedule replay for Apple ANE.
- **nvidia_gpu.rs**: NVIDIA GPU device simulator with GigaThread Engine, multiple SMs, and HBM memory.
- **amd_gpu.rs**: AMD GPU device simulator with Shader Engine grouping and Command Processor.
- **google_tpu.rs**: Google TPU device simulator with systolic array pipeline.
- **intel_gpu.rs**: Intel GPU device simulator with Xe-Slice grouping and Command Streamer.
- **apple_ane.rs**: Apple Neural Engine device simulator with unified memory (zero-copy transfers) and DMA-based scheduling.
- Comprehensive integration tests covering all 5 device types, global memory operations, work distributors, cross-device comparisons, memory+compute integration, and partition conflict detection.

### Notes

- Ported from the Python `device-simulator` package.
- Uses trait objects (`Box<dyn ComputeUnit>`) for polymorphic compute unit handling.
- Global memory uses `HashMap<u64, u8>` for sparse representation -- a 16 GB address space doesn't require 16 GB of simulator RAM.
