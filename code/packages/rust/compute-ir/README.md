# compute-ir

Placed compute graph — the lower IR of the matrix execution layer.

This is the implementation of spec **MX02**.  See:

- [`code/specs/MX00-matrix-execution-overview.md`](../../specs/MX00-matrix-execution-overview.md) — architecture
- [`code/specs/MX01-matrix-ir.md`](../../specs/MX01-matrix-ir.md) — the upstream IR
- [`code/specs/MX02-compute-ir.md`](../../specs/MX02-compute-ir.md) — this crate's contract

## What it is

`compute-ir` is the placed graph the planner produces from `matrix-ir`.
It carries the same dataflow shape as `matrix-ir` plus three things
the upper IR deliberately lacks:

1. **Placement** — every op carries an `ExecutorId`.
2. **Residency** — every tensor carries a `(ExecutorId, BufferId)`.
3. **Explicit transfers** — `PlacedOp::Transfer` is a first-class
   graph node, not implicit machinery.

Plus `Alloc` / `Free` for explicit buffer lifetime.

## Where it sits

```
   matrix-ir   (the upper IR)
        |
    planner    (compute-runtime)
        |
        v
   compute-ir   (this crate — placed graph)
        |
   executor-protocol
        |
        v
   cpu / metal / cuda / wgpu / asic
```

The runtime lowers `matrix-ir::Graph` to `ComputeGraph`, ships it to
executors via the protocol, and waits for results.

## Why explicit transfers matter

GPUs are fast at compute and slow at host↔device transfers.  When the
graph's structure is implicit, a scheduler can't see fusion
opportunities or amortise transfer costs.  By making transfers
first-class:

- The planner inserts them based on a cost model.
- The user can inspect them with `ComputeGraph::dump()`.
- Backends just execute what the placed IR says — no clever
  driver-level decisions about when to copy.

## A worked example

```rust
use compute_ir::{
    BufferId, ComputeGraph, ExecutorId, OpTiming, PlacedOp, PlacedTensor, Residency, CPU_EXECUTOR,
};
use matrix_ir::{DType, Op, Shape, TensorId};

let cpu_in  = Residency { executor: CPU_EXECUTOR, buffer: BufferId(0) };
let gpu_in  = Residency { executor: ExecutorId(1), buffer: BufferId(10) };
let gpu_out = Residency { executor: ExecutorId(1), buffer: BufferId(11) };
let cpu_out = Residency { executor: CPU_EXECUTOR, buffer: BufferId(20) };

let g = ComputeGraph {
    format_version: compute_ir::WIRE_FORMAT_VERSION,
    inputs: vec![PlacedTensor {
        id: TensorId(0), dtype: DType::F32, shape: Shape::from(&[3]),
        residency: cpu_in,
    }],
    outputs: vec![PlacedTensor {
        id: TensorId(1), dtype: DType::F32, shape: Shape::from(&[3]),
        residency: cpu_out,
    }],
    constants: vec![],
    ops: vec![
        PlacedOp::Alloc { residency: gpu_in, bytes: 12 },
        PlacedOp::Transfer {
            tensor: TensorId(0), src: cpu_in, dst: gpu_in, bytes: 12,
            timing: OpTiming { estimated_ns: 5_000 },
        },
        PlacedOp::Alloc { residency: gpu_out, bytes: 12 },
        PlacedOp::Compute {
            op: Op::Neg { input: TensorId(0), output: TensorId(1) },
            executor: ExecutorId(1),
            timing: OpTiming { estimated_ns: 500 },
        },
        PlacedOp::Alloc { residency: cpu_out, bytes: 12 },
        PlacedOp::Transfer {
            tensor: TensorId(1), src: gpu_out, dst: cpu_out, bytes: 12,
            timing: OpTiming { estimated_ns: 5_000 },
        },
        PlacedOp::Free { residency: gpu_in },
        PlacedOp::Free { residency: gpu_out },
    ],
    tensors: vec![/* ... */],
};

g.validate().unwrap();
print!("{}", g.dump());
```

## Inspection: `dump()`

A goal of having an explicit placed graph is that it should be
readable.  `ComputeGraph::dump()` produces output like:

```
ComputeGraph (format v1, 1 inputs, 1 outputs, 8 ops)
  inputs:
    t0: f32 [3]   @ exec 0 buf 0
  ops:
    [00]  alloc            buf 10 @ exec 1   (12 B)
    [01]  transfer t0      exec 0 buf 0  ->  exec 1 buf 10   (12 B, ≈ 5.0 µs)
    [02]  alloc            buf 11 @ exec 1   (12 B)
    [03]  compute  neg      exec 1 t0 -> t1                 (≈ 500 ns)
    [04]  alloc            buf 20 @ exec 0   (12 B)
    [05]  transfer t1      exec 1 buf 11  ->  exec 0 buf 20   (12 B, ≈ 5.0 µs)
    [06]  free             buf 10 @ exec 1
    [07]  free             buf 11 @ exec 1
  outputs:
    t1: f32 [3]   @ exec 0 buf 20
```

This is a teaching artifact as much as a debugging tool.  A user
looking at the dump can see exactly where the compute time goes.

## Validation

`ComputeGraph::validate()` checks:

- Format version is current.
- Tensor table positions match declared `TensorId`s.
- Every constant's bytes length equals `shape × dtype`.
- Every `Compute` op's inputs are resident on the op's executor at the
  moment that op runs.
- Every `Transfer.src` matches the tensor's current residency.
- Every `Free` follows an `Alloc` of the same residency (no
  free-without-alloc).
- No double `Alloc` of the same residency without an intervening `Free`.

Invalid graphs are a planner bug.  The runtime calls `validate()`
before executing.

## Wire format

`ComputeGraph::to_bytes()` / `ComputeGraph::from_bytes()` round-trip
the placed graph as bytes.  Format follows the same primitives as
`matrix-ir`'s wire format (varint, length-prefixed bytes, tagged
unions) — see spec MX03.  Encoding is deterministic; round-trip is
exact.

## Security

The decoder accepts untrusted bytes (a remote executor could be
malicious or compromised).  Hardening:

- All length-prefixed `Vec::with_capacity` allocations are bounded
  against remaining buffer bytes.
- `Reader::need` uses `checked_add` to prevent overflow on 32-bit.
- `bytes()` rejects `u64` lengths exceeding `usize::MAX` explicitly.
- Tests assert no panic across truncation at every byte offset and
  1024 random-byte fuzz iterations.
- `from_bytes` is documented as structural-only; callers are
  instructed to size-cap input and call `validate()`.

## Testing

```
cargo test -p compute-ir
```

Test methodology (per spec MX02 §"Test methodology"):

- Hand-built graphs for representative shapes (CPU-only, cross-executor).
- Validator acceptance and rejection across all error variants.
- Wire round-trip with determinism check.
- Decoder hardening tests (amplification, truncation, fuzz).
- `dump()` smoke tests (full byte-for-byte golden tests deferred until
  the format stabilises).

## Zero dependencies

```
$ cargo tree -p compute-ir
compute-ir v0.1.0
└── matrix-ir v0.1.0
```

Only `matrix-ir` (path dependency, also zero-dep).  No external crates.

## Out of scope (V1)

Reserved for later versions:

- Buffer aliasing (`Alloc` declares `aliases: BufferId`)
- Streams / async dispatch
- Multi-host coordination
- Fusion groups (a `FusionGroup` variant asking executors to lower a
  sub-DAG to one kernel)

See spec MX02 §"Out of scope" for migration plans.
