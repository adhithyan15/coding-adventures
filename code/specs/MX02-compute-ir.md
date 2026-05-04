# MX02 — `compute-ir`: The Placed Compute Graph

## Status

Draft — V1 specification.  Read [MX00](MX00-matrix-execution-overview.md)
and [MX01](MX01-matrix-ir.md) first.

## Purpose

`compute-ir` is the **lower IR** — what the planner produces from
`MatrixIR` and what executors consume.  It carries the same dataflow
shape as `MatrixIR` but adds three things that `MatrixIR` deliberately
lacks:

1. **Placement.**  Every op carries an `ExecutorId` saying which executor
   runs it.
2. **Residency.**  Every tensor carries a `Residency`: which executor's
   memory holds it, under what `BufferId`.
3. **Explicit transfers.**  When an op needs an input that lives on a
   different executor than the op runs on, a `Transfer` op moves it.
   Transfers are first-class graph nodes, not implicit machinery.

This split is the load-bearing piece of the layer's design.  By forcing
memory movement to be an op in the graph rather than a side-effect of
execution, the cost of every transfer is *visible* — to the planner that
inserts them, to the user that inspects the graph, and to the executor
that performs them.

This crate is **pure data**, like `matrix-ir`.  It defines structures and
serialisation; it does not execute graphs.  Execution lives in
`compute-runtime` and the executors.

## Relationship to `matrix-ir`

`compute-ir` reuses `matrix-ir`'s `DType`, `Shape`, `Tensor`, and the
27-op vocabulary.  What it adds is the placement and transfer machinery
wrapped around them.  Visually:

```
MatrixIR::Op::Add { lhs, rhs, output }
                       |
                  planner lowers
                       v
ComputeIR::PlacedOp {
    op: Op::Add { lhs, rhs, output },
    executor: ExecutorId(2),       // "run this on executor #2"
    estimated_ns: 1_400,           // planner's cost estimate, kept for telemetry
}
```

Plus, between ops that span executors, the planner inserts:

```
ComputeIR::PlacedOp {
    op: TransferOp::Transfer { src_buffer, dst_buffer, bytes },
    executor: ExecutorId::Runtime, // transfers are coordinated by the runtime
    estimated_ns: 8_000,
}
```

## Core types

```rust
pub struct ExecutorId(pub u32);    // 0 is reserved for the CPU fallback.
pub struct BufferId(pub u64);      // unique within an executor.
pub struct KernelId(pub u64);      // unique within an executor.

#[derive(Clone, Debug)]
pub struct Residency {
    pub executor: ExecutorId,
    pub buffer:   BufferId,
}

/// A tensor in the placed graph carries its dtype, shape, and current
/// residency.  Residency may change across ops as the planner inserts
/// transfers.
#[derive(Clone, Debug)]
pub struct PlacedTensor {
    pub id:        TensorId,           // same id space as matrix-ir
    pub dtype:     DType,
    pub shape:     Shape,
    pub residency: Residency,
}

#[derive(Clone, Debug)]
pub enum PlacedOp {
    /// A normal compute op, lowered as-is from MatrixIR.
    Compute {
        op:            matrix_ir::Op,
        executor:      ExecutorId,
        estimated_ns:  u64,
    },

    /// Move bytes between executors.  src and dst residencies must
    /// reference the same logical tensor (same dtype, same shape).
    Transfer {
        tensor:        TensorId,
        src:           Residency,
        dst:           Residency,
        bytes:         u64,
        estimated_ns:  u64,
    },

    /// Allocate a buffer for a tensor whose residency was just decided.
    /// Issued before the first op that reads/writes the buffer.
    Alloc {
        residency:     Residency,
        bytes:         u64,
    },

    /// Release a buffer.  Issued after the tensor's last use.
    Free {
        residency:     Residency,
    },
}

#[derive(Clone, Debug)]
pub struct ComputeGraph {
    pub format_version: u32,           // wire format version, currently 1.

    /// Inputs the runtime will receive at run time, with residency assigned
    /// (typically the host / executor 0).
    pub inputs:   Vec<PlacedTensor>,

    /// Outputs the runtime will return, with the residency they end on.
    /// The runtime inserts download transfers if a caller wants them on
    /// the host.
    pub outputs:  Vec<PlacedTensor>,

    /// Constants, with residency assigned.  May be replicated across
    /// executors when the planner expects multiple readers.
    pub constants: Vec<PlacedConstant>,

    /// Topologically ordered ops.  The order is honoured during execution.
    pub ops: Vec<PlacedOp>,

    /// Per-tensor metadata, indexed by TensorId.  Useful for inspection
    /// and for downloading specific intermediates during debugging.
    pub tensors: Vec<PlacedTensor>,
}

#[derive(Clone, Debug)]
pub struct PlacedConstant {
    pub tensor:    TensorId,
    pub bytes:     Vec<u8>,
    pub residency: Residency,           // where the constant must be uploaded.
}
```

`ExecutorId(0)` is by convention the CPU executor and is always present.
Other ids are assigned by the registry as backends register themselves.
The runtime is not itself an executor; transfers are coordinated by the
runtime but executed by the source and destination executors via
`Download` and `Upload` protocol messages (see MX03).

## Lowering: `MatrixIR` → `ComputeIR`

The planner (specified in MX04) takes a `MatrixIR::Graph` and a registry
of available executors with their `BackendProfile`s, and produces a
`ComputeIR::ComputeGraph`.  The lowering is conceptually a four-pass
algorithm:

1. **Capability filter.**  For each op, compute the set of executors that
   support it (correct dtype, correct shape rank, correct op kind).  If
   none do, fall back to `ExecutorId(0)` (CPU).

2. **Cost minimisation.**  For each op in topological order, pick the
   executor that minimises the sum of:
   - Compute cost (op kind × input size × FLOPS rate).
   - Transfer-in cost for any inputs not already resident on that
     executor.
   - Transfer-out cost (deferred — only counted if the next consumer
     would otherwise force it).
   - Kernel launch overhead.

   This is greedy per-op; it is not globally optimal but is cheap and
   produces good plans for the workloads we care about.

3. **Transfer insertion.**  Walk the placed ops in order.  For each input
   whose current residency does not match the op's executor, insert a
   `Transfer` op before it and update the input's residency.

4. **Lifetime annotation.**  Compute first-use and last-use of each
   tensor.  Insert `Alloc` before first use and `Free` after last use.
   This is purely an optimisation; executors that ignore `Free` still
   produce correct results, just with more memory pressure.

The cost model and the planner's algorithm are the subject of MX04.  This
spec only defines the output shape — a `ComputeGraph` that any executor
can consume.

## Validation

`ComputeGraph::validate()` enforces:

### Structural

1. The underlying `Op`s satisfy `matrix-ir`'s validation (shape and dtype
   contracts).
2. Every `TensorId` referenced by `PlacedOp` exists in `tensors`.
3. Every `BufferId` referenced is preceded by an `Alloc` and not
   followed by a `Free` until after its last use.
4. Every `Transfer.src` matches the most recent residency assigned to
   that `TensorId`; every `Transfer.dst` becomes the new residency.
5. Every input to a `Compute` op has a residency equal to the op's
   `executor`.

### Cost annotations

1. Every `estimated_ns` is finite and non-negative.  (The planner is
   allowed to estimate `0` for ops whose cost is below its resolution
   threshold; this is informational.)

The validator runs in O(N) over the op list and is invoked by the runtime
before executing any graph.  Invalid graphs are a planner bug — they
should never reach an executor.

## The `Transfer` op in detail

A transfer moves a tensor between two residencies.  In the wire protocol,
it decomposes into two cooperating executor-protocol messages:

- The runtime sends `DownloadBuffer(buffer_id)` to the source executor,
  receives `BufferData`.
- The runtime sends `UploadBuffer(...)` to the destination executor.

In V1 the runtime acts as the data path between executors.  This is a
deliberate simplification: it keeps the executor protocol simple (no
peer-to-peer mode), at the cost of an extra hop through the runtime's
host memory.  V2 may add direct executor-to-executor transfer when both
support it (for example two CUDA executors on different GPUs).

## Replicated constants

Constants are read-only.  When the planner predicts that multiple
executors will read the same constant, it can place a copy on each via
multiple `PlacedConstant` entries with different `residency` values.  V1
ships a simple policy: a constant is replicated if and only if multiple
executors read it directly.  More refined policies (size-aware,
hot/cold) are V2.

## Wire format

`ComputeGraph` is serialisable to bytes via the same primitives MX03
defines for the executor protocol.  This is what the runtime ships to a
remote executor as part of a `Dispatch` message.  Round-trip equality is
guaranteed:

```
ComputeGraph::to_bytes() -> Vec<u8>
ComputeGraph::from_bytes(&[u8]) -> Result<ComputeGraph, IrError>
```

The format is versioned (`format_version` field).  Mismatched versions
are a hard error in V1.

## Inspection

A goal of having an explicit placed graph is that it should be readable.
The crate exposes:

```rust
impl ComputeGraph {
    /// Pretty-print the graph as text, one op per line, with executor
    /// assignments and estimated costs.  Useful for debugging the planner.
    pub fn dump(&self) -> String;
}
```

Sample output:

```
ComputeGraph (format v1, 4 inputs, 1 output, 11 ops)
  inputs:
    t0: f32 [1, 784]   @ executor 0 buf 1
    t1: f32 [784, 128] @ executor 0 buf 2
    t2: f32 [1, 128]   @ executor 0 buf 3
  ops:
    [00]  alloc            buf 4 @ exec 1   (3.0 KiB)
    [01]  transfer t1      exec 0 buf 2  ->  exec 1 buf 4   (≈ 80 µs)
    [02]  alloc            buf 5 @ exec 1   (3.1 KiB)
    [03]  transfer t0      exec 0 buf 1  ->  exec 1 buf 5   (≈ 0.8 µs)
    [04]  compute  matmul  exec 1 t0 t1 -> t3                 (≈ 12 µs)
    [05]  alloc            buf 6 @ exec 1   (512 B)
    [06]  transfer t2      exec 0 buf 3  ->  exec 1 buf 6   (≈ 5.1 µs)
    [07]  compute  add     exec 1 t3 t2 -> t4                 (≈ 1.0 µs)
    [08]  compute  max     exec 1 t4 c0 -> t5                 (≈ 1.0 µs)  // ReLU
    [09]  transfer t5      exec 1 buf 7  ->  exec 0 buf 8   (≈ 5.1 µs)
    [10]  free             buf 4, 5, 6, 7 @ exec 1
  outputs:
    t5: f32 [1, 128]   @ executor 0 buf 8
```

This is a teaching artifact as much as a debugging tool.  A user looking
at the dump can see exactly where their compute time goes — and can
weigh adding a transfer-amortising `Broadcast` placement, or choosing a
different executor, against the visible numbers.

## Test methodology

`compute-ir` ships:

1. **Hand-built graphs** for each op variant, validated.
2. **Wire round-trip** for every test graph.
3. **`dump()` golden tests** — small graphs whose pretty-printed form is
   asserted byte-for-byte against checked-in fixtures.  This catches
   accidental layout changes.

Planner-driven tests live in `compute-runtime` (MX04), where they have a
mock registry to drive cost-based decisions deterministically.

## Out of scope (V1)

- **In-place ops / aliasing.**  Two `BufferId`s referring to the same
  bytes.  V1 says every op produces a fresh buffer.  V2 adds an
  `aliases: BufferId` field to `Alloc` to enable in-place updates where
  safe.
- **Streams / async dispatch.**  V1 executes ops in declared order with
  full synchronisation.  V2 introduces dependency edges so independent
  ops can run concurrently.
- **Multi-host coordination.**  V2.
- **Fusion groups.**  V2 introduces a `FusionGroup { ops: Vec<usize> }`
  variant that asks the executor to lower a sub-DAG to one kernel.

## Open questions

1. Should `Alloc` and `Free` be implicit (the executor manages buffers
   itself based on first/last use) or explicit (the runtime issues them)?
   V1 says explicit because it makes memory pressure visible in the
   graph.  Worth revisiting if explicit lifetime tracking becomes a
   maintenance burden.
2. Is `BufferId` 64-bit overkill?  Probably yes; 32-bit gives 4B buffers
   per executor lifetime which is plenty.  Keeping 64-bit anyway because
   a wire format change to widen later would be painful.
3. Should `Transfer` carry a checksum?  Useful for network transports
   where corruption is possible.  V1 does not; transports that need it
   add it at their layer.
