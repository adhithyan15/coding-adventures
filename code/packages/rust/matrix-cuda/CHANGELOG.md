# Changelog — matrix-cuda

## 0.3.0 — 2026-05-13

### Added — MX06 Phase 3 (generic NVRTC kernels + buffer-op wiring)

This phase lands two things in one PR:

1. The handwritten CUDA C source for the V1 op set, compiled
   through NVRTC at executor startup via `cuda-compute`'s
   `CudaDevice::compile`.
2. The executor's `handle()` dispatch surface for `AllocBuffer` /
   `UploadBuffer` / `DownloadBuffer` / `FreeBuffer` is now wired
   through the per-`State` `BufferStore` (Phase 2 shipped the
   module standalone; this PR moves it into `Mutex<State>` and
   routes the four request variants).

#### `src/kernels.rs`

- `pub const KERNELS_CUDA_C: &str` — single source string with
  every V1 kernel: 7 unary (`neg_f32`, `abs_f32`, `sqrt_f32`,
  `exp_f32`, `log_f32`, `tanh_f32`, `recip_f32`), 7 binary
  (`add_f32`, `sub_f32`, `mul_f32`, `div_f32`, `max_f32`,
  `min_f32`, `pow_f32`), plus `matmul_f32` (rank-2 row-major).
  Mirrors `matrix-metal::KERNELS_MSL` in shape.
- `pub const KERNEL_ENTRY_POINTS: &[&str]` — every entry-point
  name; `Kernels::new` errors if any name fails to resolve.
- `pub struct Kernels { module, fns: HashMap<&'static str, CudaFunction> }`
  with:
  - `new(device) -> Result<Self, String>` — compiles once, caches
    every function.
  - `get(name)` — function lookup.
  - `launch_unary` / `launch_binary` / `launch_matmul` — launch
    helpers with synchronise.  Used by tests and by Phase 5
    dispatch.
- `Const` (op tag 0x1B) is intentionally not a kernel — it's
  handled by buffer upload (`BufferStore::write`).

#### Executor wiring

- `State` gains a `buffers: BufferStore` field and a
  `next_buffer: u64` counter.
- `handle()` now routes `AllocBuffer` / `UploadBuffer` /
  `DownloadBuffer` / `FreeBuffer` through `BufferStore` (using
  `split-borrow` of the Mutex guard to satisfy the borrow checker).
- `Dispatch` / `DispatchSpecialised` / `PrepareKernel` /
  `CancelJob` still return `NOT_IMPLEMENTED` — those land in
  Phase 5.

#### `cuda-compute` 0.1.1 (companion change)

- `unsafe impl Send for CudaBuffer {}` — mirrors the existing
  `Send` impls on sibling types.  Lets `BufferStore` live behind
  `Mutex<State>`.  Sync is intentionally not impl'd; callers
  serialise via their own `Mutex`.

### Deferred to Phase 5

- Lifting `Kernels` into `State` requires `cuda-compute`'s
  `CudaLib` / `NvrtcLib` / `CudaModuleInner` to be `Send + Sync`.
  That wider audit belongs in the Phase 5 PR that also wires the
  `Dispatch` request.  Until then `Kernels` is callable directly.
- `supported_ops_bitset()` stays at `0`; Phase 5 flips it on at
  the same time `Dispatch` becomes real, so the planner never
  routes an op to us that we can't execute.

### Tests

18 new unit tests in `kernels::tests`:

- `kernels_new_compiles_all_entry_points` — every name in
  `KERNEL_ENTRY_POINTS` resolves after compilation.
- `unknown_kernel_name_errors` — graceful error path.
- One test per unary kernel (`neg`, `abs`, `sqrt`, `exp`, `log`,
  `tanh`, `recip`) comparing GPU output to a CPU oracle.
- One test per binary kernel (`add`, `sub`, `mul`, `div`, `max`,
  `min`, `pow`).
- `matmul_f32_2x2_matches_cpu` — small handwritten case.
- `matmul_f32_3x4_4x2_matches_cpu_oracle` — non-square,
  computed CPU oracle.

All device-gated: on hosts without an NVIDIA driver
(`CudaDevice::new(0)` fails) the tests silently pass.  On a
real CUDA box they exercise the full compile → launch →
download → compare loop.

Total crate test count: 44 (26 from Phases 1+2 + 18 new).

### Why this is its own PR

Phase 3 ships **two coherent pieces**: the kernels themselves (so
later phases have a known source surface to dispatch against) and
the buffer-op wiring (so the executor's protocol surface stops
returning `NOT_IMPLEMENTED` for buffer requests).

Splitting kernel + buffer wiring across two PRs would have been
artificial — both depend on `CudaBuffer: Send` (which cuda-compute
0.1.1 ships in the same PR) and the kernel tests need allocated
buffers anyway.

## 0.2.0 — 2026-05-13

### Added — MX06 Phase 2 (`BufferStore` over `cuMemAlloc` / `cuMemcpy*`)

Lands the device-memory module, the per-executor `HashMap<BufferId,
CudaBuffer>` that Phase 3+ dispatch will use to alloc / upload /
download / free GPU buffers.

- New module `src/buffers.rs` with `pub struct BufferStore`:
  - `new()` / `default()` — empty store.
  - `alloc(device, id, bytes) -> Result<(), String>` —
    `cuMemAlloc` via `cuda-compute`, replaces any prior buffer at
    `id`, propagates `cuda-compute`'s typed error on zero-length
    allocations.
  - `write(device, id, offset, data)` — `cuMemcpyHtoD` via
    `cuda-compute::CudaDevice::upload`.  Phase 2 supports
    `offset = 0` only (every caller in `matrix-metal::dispatch`
    uses 0 today; partial writes land in a later phase).
  - `read(device, id, offset, len) -> Result<Vec<u8>, _>` —
    `cuMemcpyDtoH` via `cuda-compute::CudaDevice::download`, then
    slice on the host.  Supports arbitrary offset / len.
  - `free(id)` — idempotent.
  - `get(id) -> Result<&CudaBuffer, _>` — used by Phase 3 dispatch
    to look up `CUdeviceptr`s for `cuLaunchKernel`.
  - `contains(id)`, `len()`, `is_empty()`.
- Re-exported from `lib.rs` as `matrix_cuda::BufferStore`.

### Why this is its own PR

Phase 2 ships the module **alone**, without wiring it into the
executor's `Mutex<State>` and `handle()`.  That deferral exists
because `cuda-compute::CudaBuffer` doesn't yet implement `Send` —
adding it (and wiring the alloc/upload/download/free request paths
through the executor mutex) is a coherent unit of work that
belongs in Phase 3 alongside generic NVRTC kernel compilation.

The module is usable directly today (see the unit tests for
example call patterns); Phase 3 will simply replace the existing
`NOT_IMPLEMENTED` branches for `AllocBuffer` / `UploadBuffer` /
`DownloadBuffer` / `FreeBuffer` with delegation to a
`Mutex<BufferStore>` field on the executor.

### Tests

14 new unit tests in `buffers::tests`, all passing:

- `new_store_is_empty`, `default_matches_new`,
  `free_unknown_id_is_idempotent_noop`, `get_unknown_id_errors`
  run on every platform.
- `alloc_then_free_round_trips`, `alloc_replaces_existing_buffer_at_same_id`,
  `round_trip_write_then_read_matches_input`,
  `read_with_offset_returns_correct_slice`,
  `write_unknown_id_errors`, `write_nonzero_offset_errors_in_phase_2`,
  `read_unknown_id_errors`, `read_past_end_errors`,
  `read_offset_plus_len_overflow_errors`, `alloc_zero_bytes_errors`
  are device-gated: on hosts without an NVIDIA driver
  (`CudaDevice::new(0)` fails) they silently pass.  On the Linux
  CI runner they currently no-op for the same reason; Phase 5 will
  add a `MATRIX_CUDA_TESTS=1` env-gated suite for real GPU
  coverage.

Total crate test count: 26 (12 from Phase 1 + 14 new).

### No public surface broken

Phase 1's API is unchanged.  `BufferStore` is a strictly additive
addition.  `local_transport()`, `register()`, `CudaExecutor::new()`
all behave exactly as before.

### No `unsafe` introduced

`buffers.rs` is `unsafe`-free; all FFI safety lives in
`cuda-compute`.

## 0.1.0 — 2026-05-13

### Added — MX06 Phase 1 (crate skeleton + stub)

Initial release.  Lands the crate placeholder so Phases 2–7 of MX06
each ship as small, isolated PRs against a stable surface.

- `CudaExecutor` struct with `new()` constructor that probes for
  CUDA via `cuda-compute`'s `CudaDevice::new(0)`.  On hosts without
  an NVIDIA driver / device, returns `Err(...)` and upstream
  registration silently skips the executor.
- `profile()` returns a `BackendProfile` with `kind: "cuda"`,
  `supported_ops: 0` (Phase 1 is dormant — Phase 5 flips the V1 op
  bits on), and placeholder cost-model coefficients representative
  of a mid-range Ampere card over PCIe gen 4.  Real calibration
  lands in Phase 7.
- `handle(req)` implements the full `ExecutorRequest` surface:
  - `Register` echoes our currently-stored `ExecutorId`.
  - `Heartbeat` replies `Alive { profile }`.
  - `Shutdown` is a graceful no-op.
  - Every other variant returns
    `ErrorCode::NOT_IMPLEMENTED` with a pointer to the spec
    (`code/specs/MX06-cuda-executor.md`) so future readers know
    which phase fills it in.
- `set_our_id(id)` so `matrix-runtime::register` can hand back the
  assigned `ExecutorId`.
- **MX05 specialisation surface** (`install_specialised`,
  `install_specialised_from_emitted`, `specialised_count`,
  `evict_specialised`) as contract-preserving no-ops.  Lets MX05's
  auto-installer hook into us in MX06 Phase 6 without changing call
  sites.
- `EmittedKernelPlaceholder` — the surface
  `install_specialised_from_emitted` accepts.  Replaced in Phase 4
  by the real type from `cuda_emitter`.
- Free helpers: `local_transport()` (wraps the executor in a
  `LocalTransport`) and `register(runtime)` (registers under the
  name `"cuda"`).

### Tests

12 unit tests — they assert:

- `profile()` advertises `kind = "cuda"` and the documented
  placeholder coefficients.
- `supported_ops_bitset()` returns `0` (sentinel that Phase 5 has
  not been accidentally merged in).
- `new()` returns either `Ok` (CUDA-bearing developer box) or `Err`
  with a message tagged with the crate name — no panics, no hangs.
- The MX05 surface (install / count / evict) all return the
  documented placeholder values.
- `handle(req)` routes every variant to the correct stub branch
  (Register → Registered, Heartbeat → Alive, Dispatch* / CancelJob
  → Error with NOT_IMPLEMENTED, buffer ops → Error with
  NOT_IMPLEMENTED).

### Dependencies

- `matrix-ir`, `compute-ir`, `matrix-runtime`, `matrix-profile`,
  `executor-protocol` — the standard executor stack.
- `cuda-compute` — runtime-loaded libcuda wrapper (zero link-time
  NVIDIA dependency).

No `unsafe` blocks added in this phase.

### Why this is its own PR

Phase 1 is the placeholder.  Splitting it from Phases 2–7 means each
later PR is a focused, isolated change against a known surface, and
the planner / above-layer code can start referencing
`matrix_cuda::profile()` immediately without waiting for real
dispatch to land.

### Migration

No behaviour change for existing users — `matrix-cuda` is not yet
registered by `image-gpu-core` (that wiring is Phase 6) and the
planner does not see it.

Adding `matrix-cuda` as a workspace member exposes one new package
in `cargo build --workspace` output; nothing else changes.
