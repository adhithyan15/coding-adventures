# Changelog — matrix-metal

## 0.1.1 — 2026-05-04

### Added

- **`Op::Reshape` support.**  Reshape is metadata-only in SSA — same
  numel, different shape — so the implementation is a same-size memcpy
  from the input buffer to the output buffer (going through
  `BufferStore`'s host-side read/write, which on Apple Silicon's
  unified memory is essentially `memcpy`).  Capability bitset now
  advertises tag 0x11 alongside the elementwise ops, MatMul, and
  Const.  `Op::Transpose` (0x12) and `Op::Broadcast` (0x13) need real
  data movement / index expansion and remain V2 work.

  Why this matters: it lets `image-gpu-core`'s sepia /
  colour-matrix graphs (which always reshape `pixels` and the matrix
  before `MatMul`) qualify for uniform-Metal placement under MX04's
  pass 2b.  Without Reshape support those graphs would always have a
  capability hole that prevented uniform placement and forced a
  CPU-only re-plan in the consumer.

### Fixed

- **Dispatch no longer fails on a strict `executor != our_id` check.**
  V0.1's dispatch handler aborted if the placed op's `executor` field
  didn't match `MetalExecutor`'s `our_id`, but the runtime never
  actually called `MetalExecutor::set_our_id`, so `our_id` stayed at
  `u32::MAX` and every dispatch routed by a multi-executor runtime
  failed.  V1 single-transport-per-executor doesn't need the strict
  check anyway — if our `handle()` was called, the dispatch was for
  us — so the check is now just `executor != CPU_EXECUTOR`.  Real
  routing-correctness checking is V2 work that needs the runtime to
  push the assigned id into each executor at registration time.

### Tests

- New `reshape_preserves_bytes_on_gpu` integration test confirms
  Reshape round-trips a 6-element f32 vector into a 2×3 shape with
  byte-identical contents.

## 0.1.0 — 2026-05-04

Initial release.  First specialised executor for the matrix execution
layer.

### Added

- `MetalExecutor` — implements the executor-protocol contract on
  Apple Metal.  Mutex-guarded internal state with poison recovery.
- V1 op support: F32 elementwise unary (Neg, Abs, Sqrt, Exp, Log,
  Tanh, Recip), F32 elementwise binary (Add, Sub, Mul, Div, Max, Min,
  Pow), F32 MatMul (rank-2), Const.
- `BufferStore<MetalBuffer>` — bounds-checked HashMap keyed by
  `BufferId`.  Mirrors `matrix-cpu::BufferStore` API.
- MSL kernel library (`src/kernels.rs`) compiled once at executor
  startup.  Pipelines cached by entry-point name.
- `local_transport()` and `register()` helpers.
- `profile()` advertises capability bitset (16 ops × F32 only) and
  cost model defaults sized for Apple Silicon (5 TFLOPS f32, 50 GB/s
  unified memory, 5 µs launch overhead).
- Up-front graph validation (16 MiB per-tensor cap, byte_size overflow
  check) — same hardening as `matrix-cpu`.
- Non-Apple platforms compile a stub that always returns
  `DEVICE_LOST` from `handle()` and `Err` from `MetalExecutor::new()`.

### Tests

5 integration tests pass on real Metal hardware:
- `neg_f32_on_gpu` — elementwise unary
- `add_f32_on_gpu` — elementwise binary
- `matmul_2x2_on_gpu` — `[[1,2],[3,4]] × [[5,6],[7,8]] = [[19,22],[43,50]]`
- `local_transport_heartbeat` — protocol round-trip via LocalTransport
- `dispatch_rejects_oversized_tensor` — validation guard

### Constraints

- Zero external Cargo dependencies.  Only path deps to `matrix-ir`,
  `compute-ir`, `executor-protocol`, `matrix-runtime`, `metal-compute`.
- F32 only in V1 — every other dtype falls back to `matrix-cpu` via
  the planner's capability filter.
