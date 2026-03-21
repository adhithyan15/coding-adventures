# Changelog

All notable changes to the BLAS library (TypeScript) will be documented here.

## [0.1.0] - 2026-03-20

### Added

- Core types: `Vector`, `Matrix`, `StorageOrder`, `Transpose`, `Side` enums
- Conversion utilities: `fromMatrixPkg()`, `toMatrixPkg()` for bridging to existing matrix package
- `BlasBackend` interface with full BLAS Level 1/2/3 operations
- `MlBlasBackend` interface extending BlasBackend with ML operations
- `BackendRegistry` with auto-detection priority ordering
- `createBlas()` and `useBackend()` convenience API
- **CpuBlas** -- pure TypeScript reference implementation (implements MlBlasBackend)
  - Level 1: saxpy, sdot, snrm2, sscal, sasum, isamax, scopy, sswap
  - Level 2: sgemv, sger
  - Level 3: sgemm, ssymm, sgemmBatched
  - ML: relu, gelu, sigmoid, tanh, softmax, layerNorm, batchNorm, conv2d, attention
- **GpuBlasBase** -- abstract base class using template method pattern
- **CudaBlas** -- NVIDIA CUDA backend via CUDARuntime (cudaMalloc/cudaMemcpy/cudaFree)
- **MetalBlas** -- Apple Metal backend via MTLDevice (unified memory, makeBuffer/contents)
- **VulkanBlas** -- Vulkan backend via VkInstance/VkDevice (explicit memory management)
- **OpenClBlas** -- OpenCL backend via CLContext/CLCommandQueue (event-based dependencies)
- **WebGpuBlas** -- WebGPU backend via GPU/GPUDevice (single queue, staging buffers)
- **OpenGlBlas** -- OpenGL backend via GLContext (state machine, SSBOs)
- Auto-registration of all 7 backends in global registry at import time
- Comprehensive test suite (300+ tests) covering:
  - All BLAS Level 1/2/3 operations across all 7 backends
  - Cross-backend equivalence verification
  - ML extension operations (activations, normalization, conv2d, attention)
  - Type validation and error handling
  - Registry and convenience API
