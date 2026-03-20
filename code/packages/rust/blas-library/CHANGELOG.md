# Changelog

All notable changes to the blas-library crate will be documented in this file.

## [0.1.0] - 2026-03-20

### Added

- **Core types**: `Vector`, `Matrix`, `StorageOrder`, `Transpose`, `Side` enumerations
- **BlasBackend trait**: contract for all backends covering L1/L2/L3 operations
  - Level 1: SAXPY, SDOT, SNRM2, SSCAL, SASUM, ISAMAX, SCOPY, SSWAP
  - Level 2: SGEMV, SGER
  - Level 3: SGEMM, SSYMM, SGEMM_BATCHED
- **MlBlasBackend trait**: optional ML extensions
  - Activations: ReLU, GELU, Sigmoid, Tanh
  - Normalization: Softmax, LayerNorm, BatchNorm
  - Operations: Conv2D, Attention
- **CPU backend** (`CpuBlas`): pure Rust reference implementation of both traits
- **GPU base template** (`GpuBlasBackend` trait + `GpuBlasWrapper`): Template Method pattern for GPU backends
- **6 GPU backends**: CUDA, Metal, OpenCL, Vulkan, WebGPU, OpenGL
  - Each exercises the full vendor API memory pipeline (allocate, upload, download, free)
  - Arithmetic delegated to CPU reference for correctness
- **BackendRegistry**: factory-based backend discovery and selection
  - Explicit: `registry.get("cuda")`
  - Auto-detect: `registry.get_best()` with configurable priority
  - Custom: `registry.register("name", factory_fn)`
- **301 tests** across 8 test files covering all operations, backends, and edge cases

### Fixed

- Vulkan `download()` now properly unmaps memory before returning (prevents "still mapped" error on free)
- WebGPU backend uses memory manager directly instead of constructing `GpuBuffer` with private fields
- Doc comments inside function bodies converted from `///` to `//` to avoid rustdoc warnings
- Pseudo-code in `sgemm_batched` doc wrapped in `text` fence to prevent doctest compilation
