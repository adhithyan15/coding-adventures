# Changelog

All notable changes to the `coding_adventures_blas_library` gem will be documented in this file.

## [0.1.0] - 2026-03-20

### Added
- Initial release of the Ruby BLAS library.
- Seven interchangeable backends: CPU, CUDA, Metal, OpenCL, Vulkan, WebGPU, OpenGL.
- Core data types: `Vector`, `Matrix`, `StorageOrder`, `Transpose`, `Side`.
- BLAS Level 1 operations: `saxpy`, `sdot`, `snrm2`, `sscal`, `sasum`, `isamax`, `scopy`, `sswap`.
- BLAS Level 2 operations: `sgemv`, `sger`.
- BLAS Level 3 operations: `sgemm`, `ssymm`, `sgemm_batched`.
- ML extensions: `relu`, `gelu`, `sigmoid`, `tanh_activation`, `softmax`, `layer_norm`, `batch_norm`, `conv2d`, `attention`.
- `BackendRegistry` for backend discovery and selection with configurable priority.
- `create_blas` convenience function for easy backend creation.
- `use_backend` block helper for temporary backend switching.
- `GpuBlasBase` template class implementing the GPU memory pipeline pattern.
- Comprehensive test suite with 300+ tests and 95%+ coverage.
- Cross-backend consistency tests verifying all backends produce identical results.
