# Changelog

All notable changes to the `vendor-api-simulators` Go package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **BaseVendorSimulator**: Shared foundation with four-pass device selection (vendor+type, vendor-only, type-only, fallback), `CreateAndSubmitCB` helper for immediate command execution, and `DefaultMemType`/`DefaultUsage` convenience functions.
- **CUDA Runtime Simulator** (`cuda.go`): Complete CUDA Runtime API simulation including `Malloc`, `MallocManaged`, `Free`, `Memcpy` (H2D, D2H, D2D, H2H), `Memset`, `LaunchKernel`, streams, events, timing, `DeviceSynchronize`, `DeviceReset`, `SetDevice`/`GetDevice`, `DeviceCount`, and `GetDeviceProperties`.
- **OpenCL Simulator** (`opencl.go`): Platform/device discovery (`GetPlatforms`, `GetDevices`), context and command queue creation, buffer operations (`CreateBuffer`, `EnqueueWriteBuffer`, `EnqueueReadBuffer`, `EnqueueCopyBuffer`, `EnqueueFillBuffer`), program build and kernel creation, `EnqueueNDRangeKernel` (1D/2D/3D), event-based synchronization.
- **Metal Simulator** (`metal.go`): Apple-style API with `MTLDevice`, `MTLCommandQueue`, `MTLCommandBuffer`, `MTLComputeCommandEncoder`, `MTLBlitCommandEncoder`, `MTLBuffer` (write/read/contents), `MTLLibrary`/`MTLFunction`/`MTLComputePipelineState`, `DispatchThreadgroups` and `DispatchThreads`.
- **Vulkan Simulator** (`vulkan_sim.go`): Ultra-explicit Vulkan-style API with `VkInstance`, `VkPhysicalDevice`, `VkDevice`, `VkQueue`, `VkCommandPool`, `VkCommandBuffer`, `VkBuffer`/`VkDeviceMemory`, `VkShaderModule`, `VkPipeline`, `VkDescriptorSetLayout`/`VkPipelineLayout`/`VkDescriptorSet`, `VkFence`/`VkSemaphore`, `VkResult` return codes, and all associated create-info structures.
- **WebGPU Simulator** (`webgpu.go`): Browser-safe API with `GPU`/`GPUAdapter`/`GPUDevice`, single `GPUQueue`, `GPUBuffer` (map/unmap/destroy), `GPUShaderModule`, `GPUComputePipeline`, `GPUBindGroupLayout`/`GPUPipelineLayout`/`GPUBindGroup`, `GPUCommandEncoder`/`GPUComputePassEncoder`, immutable `GPUCommandBuffer`.
- **OpenGL Simulator** (`opengl.go`): Legacy state machine API with `GLContext`, integer handles, `CreateShader`/`CompileShader`, `CreateProgram`/`AttachShader`/`LinkProgram`/`UseProgram`, `GenBuffers`/`BindBuffer`/`BufferData`/`BufferSubData`/`BindBufferBase`, `DispatchCompute`, `MemoryBarrier`, `FenceSync`/`ClientWaitSync`, `GetUniformLocation`/`Uniform1f`/`Uniform1i`.
- **Test suite**: 237 tests across 8 test files (base, CUDA, OpenCL, Metal, Vulkan, WebGPU, OpenGL, cross-API) achieving 87.4% coverage.
- Knuth-style literate programming with extensive inline documentation explaining each GPU programming paradigm.
