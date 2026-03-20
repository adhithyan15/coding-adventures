# Changelog

All notable changes to the BLAS Library package will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **Core types**: Matrix (flat storage), Vector, StorageOrder, Transpose, Side enums
- **BlasBackend protocol**: Level 1 (SAXPY, DOT, NRM2, SCAL, ASUM, IAMAX, COPY, SWAP), Level 2 (GEMV, GER), Level 3 (GEMM, SYMM, batched GEMM)
- **MlBlasBackend protocol**: ReLU, GELU, sigmoid, tanh, softmax, layer_norm, batch_norm, conv2d, attention
- **CpuBlas**: Pure Python reference implementation of all BLAS and ML operations
- **CudaBlas**: NVIDIA CUDA backend wrapping CUDARuntime
- **OpenClBlas**: Portable OpenCL backend wrapping CLContext
- **MetalBlas**: Apple Metal backend wrapping MTLDevice (unified memory)
- **VulkanBlas**: Explicit Vulkan backend wrapping VkDevice
- **WebGpuBlas**: Browser-first WebGPU backend wrapping GPUDevice
- **OpenGlBlas**: Legacy OpenGL compute backend wrapping GLContext
- **BackendRegistry**: Register, discover, and auto-select backends by priority
- **Convenience API**: `create_blas()` and `use_backend()` context manager
- **Matrix converter utilities**: `from_matrix_pkg()` / `to_matrix_pkg()` for existing Matrix class
- **Cross-backend equivalence tests**: All 7 backends produce identical results
- **Comprehensive test suite**: 200+ tests covering all operations, edge cases, and error handling
