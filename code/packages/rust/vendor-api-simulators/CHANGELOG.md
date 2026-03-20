# Changelog

All notable changes to the `vendor-api-simulators` package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **BaseSimulator**: Shared foundation with 4-pass device selection strategy, immediate execution helper (`create_and_submit_cb`), and buffer allocation convenience method.
- **CUDA simulator** (`CudaRuntime`): Full CUDA-style API including `malloc`/`free`/`memcpy`/`memset`, kernel launch with grid/block dimensions (`Dim3`), streams, events with elapsed time, and device management.
- **OpenCL simulator** (`ClContext`): Platform discovery, device enumeration, buffer management with `ClMemFlags`, program compilation with kernel registration, command queue with event-based dependencies, and `enqueue_nd_range_kernel`.
- **Metal simulator** (`MtlDevice`): Apple-style encoder model with `MtlComputeCommandEncoder` and `MtlBlitCommandEncoder`, unified memory buffers, library/function/PSO creation, `dispatch_threadgroups`, and blit operations (copy/fill).
- **Vulkan simulator** (`VkInstance`/`VkDevice`): Ultra-explicit API with create-info structs (`VkBufferCreateInfo`, `VkMemoryAllocateInfo`, etc.), full command buffer lifecycle, descriptor set management, pipeline creation, fence synchronization, and pipeline barriers.
- **WebGPU simulator** (`Gpu`/`GpuAdapter`/`GpuDevice`): Browser-first API with single queue, `GpuBuffer` with map/unmap, bind groups, compute pass encoder, command encoder with deferred execution, and `queue_write_buffer` convenience.
- **OpenGL simulator** (`GlContext`): State machine model with integer handles, `glBindBufferBase`/`glDispatchCompute`, shader compilation and program linking, buffer data operations, sync objects with `glClientWaitSync`, and uniform management.
- **Integration tests**: 186 tests across all six simulators plus cross-API interop tests.
- **Literate programming**: Extensive doc comments explaining each API's design philosophy, how it maps to the compute runtime, and analogies for newcomers.
