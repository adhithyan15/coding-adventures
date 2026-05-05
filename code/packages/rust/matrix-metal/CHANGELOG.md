# Changelog — matrix-metal

## 0.3.0 — 2026-05-05

### Added

- **`Op::Broadcast` support.**  Completes the shape-op trio (Reshape
  in #2077, Transpose in #2122, Broadcast now).  General N-D
  axis-replication kernel up to rank 4 (matching this backend's
  advertised `max_tensor_rank`).  Capability bitset now includes
  tag 0x13.

  The MSL kernel walks the output linearly: for each output element,
  decomposes the linear index into an output multi-index using the
  target dims, then builds the input multi-index by clamping each
  size-1 input axis to index 0 and copying every non-broadcast axis
  through.  Re-flattens with the input dims and reads.  Memory
  access is **read-fan-in** — many output threads read the same
  input element when broadcasting along a hot axis, which Metal
  handles well via its texture cache on Apple Silicon.

  The args struct (rank, output numel, in_dims[4], out_dims[4]) is
  encoded as 40 bytes (rounded to 48 for MSL alignment) and passed
  via `set_bytes`.

  Edge cases:
    - Rank 0 (scalar) is a no-op single-element copy.
    - Rank > 4 returns an Err.
    - Empty output (numel = 0) returns Ok without dispatching.
    - Input rank ≠ output rank returns an Err.
    - Input dim ≠ 1 and ≠ output dim is enforced at the matrix-ir
      validator level; the kernel doesn't re-check.

### Tests (2 new)

- `broadcast_row_to_matrix_on_gpu` — `(1, 3) → (4, 3)` broadcast on axis 0.
- `broadcast_column_to_matrix_on_gpu` — `(3, 1) → (3, 4)` broadcast on axis 1.

Total: 10 integration tests (was 8).

### Notes

- All three V1 shape ops (Reshape, Transpose, Broadcast) now run on
  Metal.  The capability filter no longer routes any pure shape op
  to CPU, so future graphs that mix elementwise + matmul + arbitrary
  shape ops can stay end-to-end on GPU under uniform-Metal placement.
- ML-style "bias add" (`out = x + bias` where bias broadcasts across
  the batch axis) is now expressible end-to-end on Metal.

## 0.2.0 — 2026-05-05

### Added

- **`Op::Transpose` support.**  General N-D permutation kernel up to
  rank 4 (matching this backend's advertised `max_tensor_rank`).
  Capability bitset now includes tag 0x12.

  The MSL kernel walks the output linearly: for each output element,
  it decomposes the linear index into an output multi-index using
  the output dims, reverses the permutation to get the input
  multi-index, then re-flattens with the input dims.  Cost per
  element is O(rank) divides + O(rank) multiplies.  Memory access
  is non-coalesced for non-trivial permutations — that's the price
  of generality.  V2 could special-case the rank-2 matrix-transpose
  path with a tiled shared-memory kernel; V1 keeps the kernel small.

  The args struct (rank, output numel, perm[4], in_dims[4],
  out_dims[4]) is encoded as 56 bytes (rounded to 64 for MSL
  alignment) and passed via `set_bytes`.

  Edge cases:
    - Rank 0 (scalar) is a no-op memcpy.
    - Rank > 4 returns an Err (the planner shouldn't route those to
      us once it sees `max_tensor_rank: 4`, but the dispatch defends
      in depth).
    - Empty output (numel = 0) returns Ok without dispatching.

### Tests (2 new)

- `transpose_2x3_to_3x2_on_gpu` — rank-2 matrix transpose with `perm = [1, 0]`.
- `transpose_3d_perm_021_on_gpu` — rank-3 with `perm = [0, 2, 1]` (swaps the last two axes only); confirms the kernel's permutation logic generalises beyond the rank-2 case.

Total tests: 8 integration (was 6).

### Notes

- `Op::Broadcast` (tag 0x13) is still V2 work.  Broadcasting needs
  proper stride logic — strides become non-trivial when broadcasting
  across multiple axes, and the kernel needs to know which axes are
  size-1-broadcast vs ordinary.  Out of scope for this PR.

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
