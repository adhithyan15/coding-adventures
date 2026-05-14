# matrix-cuda

**MX06 Phase 1 — CUDA executor skeleton for the matrix execution layer.**

Third executor in the stack, behind [`matrix-cpu`](../matrix-cpu) and
[`matrix-metal`](../matrix-metal).  This package puts the matrix
execution layer on NVIDIA hardware (Linux / Windows / WSL2).  Phase 1
is intentionally a stub: the crate exists, the planner knows the
executor's name, and every other phase has a stable surface to grow
into.

## Where this fits

```
┌─────────────────────────────────────────────────────────┐
│  matrix-runtime planner  (MX04)                          │
│   ├──→ matrix-cpu              (always)                  │
│   ├──→ matrix-metal            (Apple targets)           │
│   └──→ matrix-cuda             (Linux / Windows, this)   │
└─────────────────────────────────────────────────────────┘
              │
              ▼
┌─────────────────────────────────────────────────────────┐
│  cuda-compute  (G09 Layer 4B — runtime-loaded libcuda)   │
└─────────────────────────────────────────────────────────┘
```

`matrix-cuda` is to `cuda-compute` as `matrix-metal` is to
`metal-compute`: a higher-level wrapper that consumes
`executor-protocol` requests, lowers `matrix-ir::Op` to GPU kernels,
and slots into the runtime's planner / cost model.

## Roadmap

See [`code/specs/MX06-cuda-executor.md`](../../../specs/MX06-cuda-executor.md)
for the full design.  Phased rollout, one PR per phase:

| Phase | Lands                                                  | Status |
| ----- | ------------------------------------------------------ | ------ |
| 1     | crate skeleton, `BackendProfile`, stubbed dispatch     | landed (0.1.0) |
| 2     | `BufferStore` over `cuMemAlloc` / `cuMemcpy*`           | landed (0.2.0) |
| 3     | `kernels.rs` — generic NVRTC-compiled kernels + buffer-op wiring (adds `Send` to `CudaBuffer`) | landed (0.3.0) |
| 4     | `cuda_emitter.rs` — specialised-kernel code generator   | landed (0.4.0) |
| 5a    | `specialised_table.rs` module + cuda-compute Send/Sync audit | landed (0.5.0) |
| 5b    | Real `Dispatch` + `DispatchSpecialised` + `install_specialised_from_emitted` + `evict_specialised` + flip `supported_ops_bitset` | **this PR** (0.6.0) |
| 6     | MX05 hooks — `backend_id = 2` in image-gpu-core         | pending |
| 7     | Planner integration — cost-model coefficients           | pending |

## Phase 5b additions

This is the big phase.  matrix-cuda is now a fully functional
executor on NVIDIA hardware:

- New `pub mod dispatch` — walks a `ComputeGraph` and dispatches
  each op through the cached `Kernels` module.  Mirrors
  `matrix-metal::dispatch::run` in shape.
- `State` gains `kernels: Option<Kernels>` (lazy NVRTC compile on
  first `Dispatch`) and `specialised: SpecialisedTable`.
- `handle()` routes `Dispatch` and `DispatchSpecialised` through
  the real paths.  `Dispatch` returns `DispatchDone`;
  `DispatchSpecialised` looks up the handle in `specialised_table`
  and falls back to `NOT_IMPLEMENTED` (so the runtime can route to
  generic) if not installed.
- `install_specialised_from_emitted(handle, EmittedKernel)`
  NVRTC-compiles the source, looks up the function by entry-point,
  builds a closure that captures the module + function, and
  installs it.
- `evict_specialised(handle)` — completes the MX05 deopt loop on
  this backend.
- `supported_ops_bitset()` flipped on: V1 ops are now claimed
  (`0x00..=0x0D`, `0x15`, `0x1B`).

**What still doesn't change**:

- `matrix-cuda` is not yet registered anywhere — that wiring is
  Phase 6 (image-gpu-core).
- No planner cost-model calibration — Phase 7.
- No GPU CI runner — Phase 7.

## Phase 5a additions

- New `pub mod specialised_table` with `SpecialisedTable` — the
  per-executor `HashMap<u64 handle, Box<closure>>` of installed
  specialised kernels.  Same shape as `matrix-metal`'s table; the
  closure signature differs because CUDA closures capture their own
  `CudaFunction` and launch through `CudaDevice` directly.
- Companion `cuda-compute` v0.1.2 release adds `Sync` to
  `CudaModuleInner` / `CudaFunction` and both `Send + Sync` to
  `CudaLib` / `NvrtcLib`.  This makes `Kernels: Send + Sync`,
  unlocking the Phase 5b lift into `Mutex<State>`.
- New compile-only test `kernels_is_send_and_sync` that fails the
  build if cuda-compute ever regresses on its Send/Sync promise.

**Note**: Phase 5a is **purely additive** — no executor behaviour
changed.  `Dispatch` still returns `NOT_IMPLEMENTED`,
`supported_ops_bitset()` stays at `0`, and
`install_specialised_from_emitted` still errors.  Phase 5b wires
all of that in one coherent change.

## Phase 4 additions

- `pub mod cuda_emitter` — pure code-generator.  Takes a
  `matrix_profile::SpecKey` + 64-bit handle and returns an
  `EmittedKernel { source, entry_point, input_buffer_count,
  output_buffer_count }`.
- Supports the same SpecKey shapes as `matrix_metal::msl_emitter`:
  unary folded-input precomputed (`0x00..=0x06`), commutative binary
  with RHS constant (`0x07/0x09/0x0B/0x0C`), non-commutative binary
  with LHS- and RHS-folded variants (`0x08/0x0A/0x0D`), and MatMul
  with a folded RHS matrix (`0x15`, 2×2 or 4×4 only).
- Entry-point naming matches the MSL emitter: handles are
  zero-padded uppercase hex; per-handle modules are designed to
  coexist in the same per-executor cache (Phase 5).
- 24 new platform-independent tests (string comparison only).

**Note**: this PR adds the generator; Phase 5 will hand its output
to NVRTC compilation + the per-handle specialised-kernel table.

## Phase 3 additions

- `pub mod kernels` with:
  - `KERNELS_CUDA_C` — single CUDA C source string for all V1
    kernels (7 unary, 7 binary, MatMul).
  - `KERNEL_ENTRY_POINTS` — names exposed by the compiled module.
  - `Kernels::new(device)` — NVRTC-compiles + caches every
    function on first call.  ~100 ms one-time cost.
  - `launch_unary` / `launch_binary` / `launch_matmul` — direct
    kernel launch helpers (used by Phase 5 dispatch and by the
    device-gated unit tests).
- Executor `handle()` now serves `AllocBuffer` / `UploadBuffer` /
  `DownloadBuffer` / `FreeBuffer` through `BufferStore`
  (`Mutex<State>`).  `Dispatch` is still `NOT_IMPLEMENTED` —
  Phase 5.
- `cuda-compute` v0.1.1 adds `unsafe impl Send for CudaBuffer {}`
  so `BufferStore` can live inside the executor's mutex.

**Note**: `supported_ops_bitset()` stays at `0` until Phase 5
wires `Dispatch`, so the planner never routes an op to us we
can't yet execute.  `Kernels` is callable directly today.

## What this phase ships

- **Crate exists**, compiles on every platform, links cleanly.
- **`CudaExecutor::new()`** probes for CUDA via `cuda-compute`'s
  `CudaDevice::new(0)`.  Returns `Err(...)` on macOS, NVIDIA-less
  Linux, NVIDIA-less Windows — upstream registration silently skips
  the executor.
- **`profile()`** returns a placeholder `BackendProfile` with
  `supported_ops: 0`.  The planner's capability filter therefore
  routes every op away from us until Phase 5 flips the bits on.
- **`handle(req)`** returns `NOT_IMPLEMENTED` for `Dispatch` /
  `DispatchSpecialised` / `CancelJob`, replies `Alive` to
  `Heartbeat`, and echoes our `ExecutorId` for `Register` /
  `Shutdown`.
- **MX05 spec surface** (`install_specialised`,
  `install_specialised_from_emitted`, `specialised_count`,
  `evict_specialised`) exists as a contract-preserving no-op so
  Phase 6 can wire it in without changing call sites.

## What this phase does NOT ship

- Real GPU dispatch.  Lands in Phase 5.
- Wiring into `image-gpu-core`.  Lands in Phase 6.
- Planner cost-model integration.  Lands in Phase 7.
- A GPU CI runner.  Device tests will gate on `MATRIX_CUDA_TESTS=1`
  starting in Phase 5.

## Usage (Phase 1 surface)

```rust
use matrix_cuda::{CudaExecutor, profile};

let executor = match CudaExecutor::new() {
    Ok(e) => e,
    Err(msg) => {
        // No CUDA on this host — registration drops us.
        eprintln!("matrix-cuda unavailable: {}", msg);
        return;
    }
};

assert_eq!(profile().kind, "cuda");
assert_eq!(executor.specialised_count(), 0);
```

## Testing

Local: `cargo test -p matrix-cuda`.

CI: the unit tests run on every platform.  They are structured so
that the device-touching ones are gated on `CudaExecutor::new()`
succeeding — on hosts without an NVIDIA driver they silently no-op
rather than fail.

Phase 5 will introduce a `MATRIX_CUDA_TESTS=1` env-gated suite that
hits a real GPU.

## License

MIT (matches every other crate in this workspace).
