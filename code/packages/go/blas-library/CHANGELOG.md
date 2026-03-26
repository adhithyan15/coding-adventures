# Changelog

All notable changes to the Go BLAS Library package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Core types: `Vector`, `Matrix`, `StorageOrder`, `Transpose`, `Side` enums
- `BlasBackend` interface with Level 1, 2, and 3 BLAS operations
- `MlBlasBackend` interface extending BlasBackend with ML operations (ReLU, GELU, Sigmoid, Tanh, Softmax, LayerNorm, BatchNorm, Conv2d, Attention)
- `BackendRegistry` with factory-based registration, priority-ordered auto-detection, and explicit selection
- `GlobalRegistry` singleton and `CreateBlas()` convenience function
- Seven backend implementations:
  - `CpuBlas` -- pure Go reference implementation (both BlasBackend and MlBlasBackend)
  - `CudaBlas` -- NVIDIA CUDA backend wrapping CUDARuntime
  - `MetalBlas` -- Apple Metal backend wrapping MTLDevice
  - `VulkanBlas` -- Vulkan backend using explicit memory management
  - `OpenClBlas` -- OpenCL backend wrapping CLContext
  - `WebGpuBlas` -- WebGPU backend wrapping GPUDevice
  - `OpenGlBlas` -- OpenGL backend wrapping GLContext
- `gpuBase` shared GPU template using the Template Method pattern
- Automatic backend registration via `init()` in the backends package
- Comprehensive test suite (300+ tests) covering all operations across all backends
