# Changelog

All notable changes to the `coding_adventures_vendor_api_simulators` gem will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **CUDA Runtime Simulator** (`CUDARuntime`): Implicit stream-based API with `malloc`/`memcpy`/`launch_kernel`/`free` workflow, streams, events, and device management.
- **OpenCL Simulator** (`CLContext`, `CLPlatform`): Portable event-based API with platform discovery, context/queue/buffer/program/kernel management, and event dependency chains.
- **Metal Simulator** (`MTLDevice`): Apple-style API with unified memory buffers, command encoders (compute and blit), libraries/functions, and pipeline state objects.
- **Vulkan Simulator** (`VkInstance`, `VkDevice`): Ultra-explicit API with create-info structs, command pools/buffers, descriptor sets, fences/semaphores, and pipeline barriers.
- **WebGPU Simulator** (`GPU`, `GPUAdapter`, `GPUDevice`): Browser-safe single-queue API with buffer mapping, bind groups, compute pass encoders, and descriptor-based resource creation.
- **OpenGL Compute Simulator** (`GLContext`): Legacy state machine API with integer handles, global state bindings, shader/program management, SSBO support, and sync objects.
- **BaseVendorSimulator**: Shared base class with 4-pass device selection and `_create_and_submit_cb` helper for implicit-execution APIs.
- **Cross-API tests**: Capstone tests verifying all six simulators can dispatch compute work and read/write memory through the same Layer 5 runtime.
- 200+ unit tests across all simulators targeting 95%+ coverage.
