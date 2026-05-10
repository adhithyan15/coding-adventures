# matrix-cpu

CPU reference executor for the matrix execution layer.  The
always-available safety-net executor — supports every `matrix-ir` op
on every dtype using straight-line Rust.

This is the first executor crate in the layer, completing the V1
end-to-end path:

```
   matrix-ir   →   matrix-runtime planner   →   compute-ir   →   matrix-cpu (or future Metal/CUDA/...)
```

See:
- [`code/specs/MX00-matrix-execution-overview.md`](../../specs/MX00-matrix-execution-overview.md)
- [`code/specs/MX04-compute-runtime.md`](../../specs/MX04-compute-runtime.md) §"CPU executor"
- [`code/specs/MX03-executor-protocol.md`](../../specs/MX03-executor-protocol.md) §"Backend implementation guide"

## What it does

- Implements the `executor-protocol` `ExecutorRequest` → `ExecutorResponse`
  contract via [`CpuExecutor::handle()`](src/lib.rs).
- Owns a `BufferStore` (HashMap of `BufferId` → `Vec<u8>`) for tensors.
- Walks `ComputeGraph.ops` and evaluates each `Compute` op via
  straight-line Rust (`src/dispatch.rs`).
- Provides per-op evaluators (`src/eval.rs`) for all 27 matrix-ir ops
  on F32/U8/I32 dtypes, with IEEE-754 float semantics and
  saturating/wrapping integer arithmetic.
- Exposes a `local_transport()` helper that wraps a `CpuExecutor` in
  a `LocalTransport` for use by `matrix-runtime`.

## Op coverage

All 27 V1 ops × 3 dtypes:

| Group | Ops | F32 | U8 | I32 |
|-------|-----|-----|-----|-----|
| Elementwise unary | Neg, Abs | ✓ | ✓ | ✓ |
| Elementwise unary (float-only) | Sqrt, Exp, Log, Tanh, Recip | ✓ | — | — |
| Elementwise binary | Add, Sub, Mul, Max, Min | ✓ | ✓ | ✓ |
| Elementwise binary (float-only) | Div, Pow | ✓ | — | — |
| Reductions | ReduceSum, ReduceMax, ReduceMean | ✓ | ✓ | ✓ |
| Shape | Reshape, Transpose, Broadcast | ✓ | ✓ | ✓ |
| LinAlg | MatMul | ✓ | ✓ | ✓ |
| Comparison | Equal, Less, Greater | ✓ | ✓ | ✓ (output U8) |
| Selection | Where | ✓ | ✓ | ✓ |
| Conversion | Cast | F32↔U8↔I32 |
| Constants | Const | ✓ | ✓ | ✓ |

Float-only ops produce IEEE-754 results (NaN/Inf for out-of-domain
input).  Integer ops use wrapping arithmetic for arithmetic variants
and saturating clamps for cross-dtype `Cast` operations.

## Worked example

```rust
use compute_ir::BufferId;
use executor_protocol::{block_on, ExecutorRequest, ExecutorResponse, Transport};
use matrix_cpu::local_transport;

let t = local_transport();

// Allocate, upload, dispatch, download.
let buf_a = match block_on(t.request(ExecutorRequest::AllocBuffer { bytes: 12 })).unwrap() {
    ExecutorResponse::BufferAllocated { buffer } => buffer,
    _ => unreachable!(),
};
// ... build a ComputeGraph, dispatch, download ...
```

See `tests/integration.rs` for the full pipeline.

## Buffer model

`BufferStore` is a HashMap of `BufferId → Vec<u8>`:

- **Tensor layout**: dtype-encoded little-endian, row-major.
- **Bounds**: `read` and `write` use `checked_add` for offset+len to
  prevent overflow; out-of-range accesses return `Err` rather than
  panicking.
- **Lifetime**: `AllocBuffer` adds, `FreeBuffer` removes.  Idempotent.

## Tests: 27 passing

- 13 unit tests (BufferStore, eval helpers, dtype conversion)
- 14 integration tests:
  - `alloc_upload_download_round_trip`
  - `heartbeat_returns_alive_with_profile`
  - `shutdown_returns_shutting_down`
  - `cancel_returns_cancelled`
  - `dispatch_add_f32`, `dispatch_matmul_f32`
  - `dispatch_with_constant`
  - `dispatch_reduce_sum`
  - `dispatch_where_chooses_per_predicate`
  - `dispatch_comparison_yields_u8`
  - `neg_f32`, `abs_i32` per-dtype unary smoke tests
  - `local_transport_ferries_requests`
  - `local_transport_full_pipeline`

## Zero dependencies

```
$ cargo tree -p matrix-cpu
matrix-cpu v0.1.0
├── compute-ir v0.1.0
├── executor-protocol v0.1.0
├── matrix-ir v0.1.0
└── matrix-runtime v0.1.0
```

Only the upstream matrix-execution-layer crates as path deps.

## Out of scope (V1)

- **Multi-threading** — V1 is single-threaded.  V2 may parallelise
  across ops or use Rayon for elementwise ops.
- **SIMD** — V1 uses scalar arithmetic.  V2 may auto-vectorise.
- **Async dispatch** — V1 is synchronous (cancel is a no-op).
- **Runtime profiling** — `OpTiming.ns` is currently 0 for CPU; V2
  may use `std::time::Instant` to measure per-op time.

## Security

Buffer-store bounds-checks every read and write with `checked_add`.
Dispatch validates every op's inputs and returns `Err` on missing
buffers, wrong sizes, or out-of-range indices rather than panicking.
