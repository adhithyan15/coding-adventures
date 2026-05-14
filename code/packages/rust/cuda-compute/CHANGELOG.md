# Changelog — cuda-compute

## 0.1.1 — 2026-05-13

### Added

- `unsafe impl Send for CudaBuffer {}` — mirrors the existing
  `Send` impls on `CudaDevice`, `CudaModuleInner`, and
  `CudaFunction`.  Lets callers move buffers into a `Mutex<...>`,
  which is required for `matrix-cuda`'s `Mutex<State>` to compile
  once `BufferStore` (which holds `CudaBuffer`s) moves inside the
  executor state.
- `Sync` is intentionally NOT impl'd: concurrent calls against the
  same buffer from multiple threads have undefined ordering in the
  CUDA driver API.  Callers serialise via their own `Mutex`.

Justification: `CUdeviceptr` is a `u64`-typed opaque driver handle;
NVIDIA's docs say it's safe to transfer between threads.  The
thread-bound entity is the CUDA *context*, not individual
allocations.

## 0.1.0 — 2026-04-23

Initial release.

### Added

- Zero link-time NVIDIA dependency: CUDA Driver API (`libcuda.so.1`) and
  NVRTC (`libnvrtc.so`) are loaded at runtime via `dlopen`/`LoadLibrary`.
  If CUDA is absent, `CudaDevice::new()` returns `Err(NotAvailable)`.
- `CudaDevice` — wraps a CUDA context; provides `alloc`, `alloc_with_bytes`,
  `compile`, `launch`, `synchronize`, `download`.
- `CudaBuffer` — device memory allocation with length tracking.
- `CudaModule` — NVRTC-compiled PTX module loaded into the driver.
- `CudaFunction` — handle to a `__global__` kernel function.
- `CudaError` — typed error enum: `NotAvailable`, `DriverError`,
  `CompileFailed`, `FunctionNotFound`, `MemError`, `LaunchError`.
- Platform support: Unix (`dlopen`/`dlsym`), Windows (`LoadLibraryA`/
  `GetProcAddress`), other (always returns `NotAvailable`).
- Thread safety: `CudaDevice` is `Send` (moveable between threads) but not
  `Sync` (CUDA contexts are single-threaded).
- Unit tests: device probe returns `NotAvailable` gracefully on non-CUDA
  machines; round-trip buffer alloc/download; NVRTC compilation and kernel
  launch on CUDA machines.
