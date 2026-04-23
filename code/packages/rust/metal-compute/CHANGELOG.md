# Changelog — metal-compute

## 0.1.0 — 2026-04-23

Initial release.

### Added

- `MetalDevice` — wraps `id<MTLDevice>`; creates buffers, compiles MSL
  libraries, creates compute pipelines, and returns a command queue.
- `MetalBuffer` — GPU/CPU shared memory buffer (`MTLResourceStorageModeShared`);
  `to_vec()` reads back bytes on the CPU side.
- `MetalLibrary` — compiled MSL library (`id<MTLLibrary>`).
- `MetalFunction` — reference to a named compute kernel.
- `MetalComputePipeline` — compiled pipeline state object (PSO);
  exposes `preferred_threads_1d()` and `preferred_threads_2d()` for
  optimal threadgroup size selection.
- `MetalCommandQueue` — serialised command queue; `dispatch(|enc| { … })`
  creates a command buffer, encodes compute work, commits, and waits.
- `MetalComputeEncoder` — builder inside `dispatch` closure; `set_pipeline`,
  `set_buffer`, `set_bytes`, `dispatch_threads_1d`, `dispatch_threads`.
- `MetalError` — typed error enum: `NotSupported`, `NoDevice`,
  `CompileFailed`, `FunctionNotFound`, `PipelineFailed`.
- Non-Apple stub: on non-Apple targets, `MetalDevice::new()` returns
  `Err(MetalError::NotSupported)`.
- Thread-safety: `MetalDevice`, `MetalLibrary`, `MetalComputePipeline`,
  `MetalCommandQueue` are `Send + Sync`; `MetalBuffer` is `Send` only.
- Ten unit tests covering device creation, MSL compilation, pipeline setup,
  buffer allocation, round-trip dispatch, and preferred threadgroup sizes.
