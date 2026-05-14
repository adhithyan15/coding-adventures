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
| 2     | `BufferStore` over `cuMemAlloc` / `cuMemcpy*`           | **this PR** (0.2.0) |
| 3     | `kernels.rs` — generic NVRTC-compiled kernels (also adds `Send` to `CudaBuffer` and wires the buffer store into `handle()`) | pending |
| 4     | `cuda_emitter.rs` — specialised-kernel code generator   | pending |
| 5     | `specialised_table.rs` + real `Executor` impl           | pending |
| 6     | MX05 hooks — `backend_id = 2` in image-gpu-core         | pending |
| 7     | Planner integration — cost-model coefficients           | pending |

## Phase 2 additions

- `pub mod buffers` — `BufferStore` over `cuda-compute`'s
  `CudaDevice::alloc` / `upload` / `download`.  Same shape as
  `matrix-metal::BufferStore`, suitable for the dispatch wiring that
  Phase 3 will add.
- Re-exported as `matrix_cuda::BufferStore`.
- 14 new unit tests (most device-gated; silent pass on non-NVIDIA
  hosts).

**Note**: the `handle()` dispatch surface still returns
`NOT_IMPLEMENTED` for `AllocBuffer` / `UploadBuffer` /
`DownloadBuffer` / `FreeBuffer`.  Wiring the store through the
executor's `Mutex<State>` requires `CudaBuffer: Send`, which
`cuda-compute` doesn't ship yet — both changes land together in
Phase 3.

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
