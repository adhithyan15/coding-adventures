# Changelog — cuda-compute

## 0.1.2 — 2026-05-13

### Added

- `unsafe impl {Send, Sync} for CudaLib {}` and `for NvrtcLib {}`.
  Both wrap a `dynlib::DynLib` (a `*mut c_void` library handle from
  `dlopen` / `LoadLibrary`) plus function pointers; the dynamic
  linker explicitly supports concurrent reads, and function
  pointers are already `Send + Sync` by default.
- `unsafe impl Sync for CudaModuleInner {}` (Send was already
  there).  Lets `Arc<CudaModuleInner>` be `Send`, which in turn
  makes `CudaModule` and `CudaFunction` `Send`.
- `unsafe impl Sync for CudaFunction {}` (Send was already there).

### Why

`matrix-cuda` Phase 5 lifts `Kernels` (which holds a `CudaModule`
and a `HashMap<&str, CudaFunction>`) into its executor's
`Mutex<State>`.  For that to compile, every type inside `State`
must be `Send`, which means `Arc<...>` of these types must be
`Send + Sync`, which means the inner type must be both.  The Sync
adds in this release are the missing pieces.

Concurrent driver calls against the same module / function from
multiple threads remain undefined; callers serialise via
`Mutex<State>`.  Sync here means "transferring shared refs is
safe," not "concurrent driver calls are safe."

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
