# Changelog

All notable changes to `matrix-cpu` are documented here.

## [0.1.0] — 2026-05-04

Initial release.  First executor crate of the matrix execution layer.

### Added

- `CpuExecutor` — owns buffer store + kernel cache; processes the full
  set of `ExecutorRequest` variants from `executor-protocol`.
- `BufferStore` — HashMap-backed `BufferId → Vec<u8>` map.  Bounds-checked
  reads and writes with `checked_add` for offset+len.
- Per-op evaluators (`src/eval.rs`):
  - Elementwise unary (Neg, Abs, Sqrt, Exp, Log, Tanh, Recip) — 27 ops
    × 3 dtypes
  - Elementwise binary (Add, Sub, Mul, Div, Max, Min, Pow)
  - MatMul on F32/U8/I32 with row-major layout
  - Reductions (Sum, Max, Mean) along arbitrary axes with keep_dims
  - Shape ops (Reshape, Transpose with permutation, Broadcast)
  - Comparisons (Equal, Less, Greater) producing U8 output
  - Where (per-element predicate selection)
  - Cast across F32 ↔ U8 ↔ I32 (saturating clamps for out-of-range)
- `dispatch::run()` — walks `ComputeGraph.ops` in order, executes each
  Compute op, copies bytes for Transfer ops, allocates/frees buffers.
- `profile()` — default `BackendProfile` for CPU executors.
- `register()` — convenience that registers CPU with a `Runtime`.
- `local_transport()` — wraps a fresh CpuExecutor in a LocalTransport.

### Test coverage: 27 tests passing

- 13 unit tests (buffer store, eval helpers, dtype conversion)
- 14 integration tests covering:
  - Direct request/response (alloc/upload/download, heartbeat,
    shutdown, cancel)
  - Single-op dispatch (Add, MatMul, ReduceSum, Where, Less)
  - Multi-input dispatch with constants
  - Local transport pipeline (alloc → upload → dispatch → download)
  - Per-dtype unary smoke tests

### Constraints

- Zero external dependencies.  Only matrix-ir, compute-ir,
  executor-protocol, matrix-runtime (path-only).
- Single-threaded execution; mutex-guarded internal state for thread
  safety of `Arc<CpuExecutor>`.
- IEEE-754 float semantics; wrapping integer arithmetic; saturating
  clamps for cross-dtype Cast.

### Out of scope (V1, deferred to V2)

- Multi-threaded / SIMD evaluation
- Real per-op timing measurements
- Async-aware cancel
- Cooperative streams / overlap with transfers
