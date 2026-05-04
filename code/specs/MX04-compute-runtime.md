# MX04 — `compute-runtime`: Planner, Registry, Cost Model

## Status

Draft — V1 specification.  Read [MX00](MX00-matrix-execution-overview.md),
[MX01](MX01-matrix-ir.md), [MX02](MX02-compute-ir.md), and
[MX03](MX03-executor-protocol.md) first.

## Purpose

`compute-runtime` is the **brain** of the matrix execution layer.  It:

1. **Owns the registry** of available executors and their capabilities.
2. **Lowers `MatrixIR` to `ComputeIR`** by running the planner.
3. **Drives execution** by sending `ExecutorRequest`s through transports
   and collecting `ExecutorResponse`s.
4. **Exposes the runtime API** that domain libraries call.

It depends only on `matrix-ir`, `compute-ir`, and `executor-protocol` —
plus `core`, `alloc`, and `std`.  It depends on no specific executor or
transport implementation; those are looked up through the registry.

## Public API

```rust
pub struct Runtime { /* ... */ }

impl Runtime {
    /// Construct a runtime with the always-available CPU executor.
    /// Additional executors are added via `register`.
    pub fn new() -> Self;

    /// Register an executor reachable through the given transport.
    /// Returns the assigned ExecutorId.
    pub fn register(
        &mut self,
        transport: Box<dyn Transport>,
    ) -> Result<ExecutorId, RuntimeError>;

    /// Plan a graph: lower MatrixIR to ComputeIR using the current
    /// registry.  Pure; no execution.
    pub fn plan(&self, graph: &matrix_ir::Graph) -> Result<compute_ir::ComputeGraph, PlanError>;

    /// Plan and execute.  Returns output tensors as host-side bytes
    /// keyed by output TensorId.
    pub fn run(
        &self,
        graph:  &matrix_ir::Graph,
        inputs: HashMap<TensorId, Vec<u8>>,
    ) -> Result<HashMap<TensorId, Vec<u8>>, RuntimeError>;

    /// Inspect the registry.
    pub fn executors(&self) -> &[RegisteredExecutor];
}

pub struct RegisteredExecutor {
    pub id:       ExecutorId,
    pub kind:     String,            // "cpu", "metal", "cuda", ...
    pub profile:  BackendProfile,
    pub healthy:  bool,
}
```

The split between `plan` and `run` exists so users can inspect
placement decisions without paying for execution.  This is a teaching
affordance and a debugging affordance.

## `BackendProfile`

A profile is what a backend tells the registry about itself.  It is
data — no behaviour — so it can travel over the wire (executors
running in another process advertise themselves through `Register`
messages, as defined in MX03).

```rust
pub struct BackendProfile {
    pub kind: String,                       // "cpu" | "metal" | "cuda" | ...

    /// Capability bitset: which Op variants are supported, indexed by
    /// the V1 op tags from MX03.  A 0 bit means "fall back to CPU".
    pub supported_ops: u32,                 // 27 ops fit in 32 bits.

    /// Which dtypes are supported.  Bit i corresponds to DType discriminant i.
    pub supported_dtypes: u8,

    /// Compute throughput, peak, in floating-point ops per nanosecond.
    pub gflops_f32: u32,                    // 1 = 1 GFLOPS, scaled.
    pub gflops_u8:  u32,
    pub gflops_i32: u32,

    /// Memory bandwidth, in bytes per nanosecond.
    pub host_to_device_bw: u32,             // bytes / ns
    pub device_to_host_bw: u32,
    pub device_internal_bw: u32,            // for on-device tensor ops

    /// Fixed overhead per dispatched op, in nanoseconds.
    pub launch_overhead_ns: u32,

    /// Network latency for the transport carrying this executor, in
    /// nanoseconds.  0 for in-process transports.
    pub transport_latency_ns: u32,

    /// Working-set capacity, in megabytes.
    pub on_device_mib: u32,

    /// Maximum tensor rank this backend supports.  4 is conservative for
    /// most real backends; unlimited (= 16) is fine for CPU.
    pub max_tensor_rank: u8,

    /// Maximum size of any single dimension.  GPUs typically cap dispatches
    /// at 65535 threadgroups in some axis; this surfaces that limit.
    pub max_dim: u32,
}
```

The cost model is **deliberately small**.  Real GPU performance is
shape-dependent and cache-dependent, and a faithful model would require
microbenchmarks per (op, shape, dtype) triple.  V1 takes the position
that a coarse model with good defaults beats no model, and that the
planner only needs ordinal correctness — pick the cheapest backend, not
the exact nanosecond count.

The CPU executor defaults to a measured profile from the host machine
(detected at `Runtime::new` startup via a 100 ms calibration loop).
Other executors return a built-in profile from their crate, possibly
refined at startup with on-device microbenchmarks.

## The planner algorithm

Inputs:

- `Graph` (from `matrix-ir`)
- `[(ExecutorId, BackendProfile, healthy)]` registry snapshot

Output:

- `ComputeGraph` (from `compute-ir`)

Algorithm:

```
fn plan(graph, registry):
    # Pass 1: capability filter — for each op, set of executors that can run it
    candidates: HashMap<OpIndex, Vec<ExecutorId>> = ...
    for (i, op) in graph.ops:
        candidates[i] = registry
            .filter(profile.supports_op(op.tag))
            .filter(profile.supports_dtype(op.dtype))
            .filter(profile.supports_shape(op.output_shape))
            .ids()
        if candidates[i].is_empty():
            candidates[i] = [CPU_EXECUTOR]      # always available

    # Pass 2: greedy cost minimisation, in topological order
    placement: HashMap<OpIndex, ExecutorId> = ...
    residency: HashMap<TensorId, ExecutorId> = ...   # initially: inputs on host (cpu)

    for (i, op) in graph.ops:
        best_cost = INF
        best_exec = candidates[i][0]
        for exec_id in candidates[i]:
            cost = compute_cost(op, exec_id, residency, registry)
            if cost < best_cost:
                best_cost = cost
                best_exec = exec_id
        placement[i] = best_exec
        residency[op.output] = best_exec

    # Pass 3: transfer insertion
    placed_ops: Vec<PlacedOp> = ...
    current_residency: HashMap<TensorId, ExecutorId> = inputs_residency.clone()

    for (i, op) in graph.ops:
        target = placement[i]
        for input in op.inputs():
            if current_residency[input] != target:
                placed_ops.push(Transfer { input, src: current_residency[input], dst: target, ... })
                current_residency[input] = target
        placed_ops.push(Compute { op, executor: target, estimated_ns: ... })
        current_residency[op.output] = target

    # Pass 4: lifetime annotation
    for (i, op) in placed_ops:
        for input in op.inputs():
            if last_use(input) == i:
                placed_ops.push_after(i, Free { residency: ... })
    insert_allocs_at_first_use(placed_ops)

    return ComputeGraph { ops: placed_ops, ... }
```

`compute_cost` is the core cost function:

```
fn compute_cost(op, exec_id, residency, registry):
    p = registry[exec_id].profile

    # Compute cost
    flops = estimate_flops(op)
    compute_ns = flops * 1_000_000_000 / p.gflops_for(op.dtype)
    compute_ns += p.launch_overhead_ns
    compute_ns += p.transport_latency_ns

    # Transfer cost
    transfer_ns = 0
    for input in op.inputs():
        if residency[input] != exec_id:
            bytes = input.shape.numel() * input.dtype.size_bytes()
            transfer_ns += bytes / bandwidth(residency[input], exec_id, registry)

    return compute_ns + transfer_ns
```

`estimate_flops` is a small lookup table per op kind:

| Op | flops |
|----|------|
| Add, Sub, Mul, Div, Max, Min | `numel(output)` |
| Pow | `5 * numel(output)` (log + mul + exp roughly) |
| Sqrt, Recip, Tanh, Exp, Log | `5 * numel(output)` |
| Neg, Abs, Cast | `numel(output)` |
| MatMul | `2 * m * k * n` |
| ReduceSum, ReduceMax, ReduceMean | `numel(input)` |
| Reshape, Transpose, Broadcast | `numel(output)` (memory cost dominates) |
| Equal, Less, Greater | `numel(output)` |
| Where | `numel(output)` |
| Const | `0` (just an upload) |

Transfer cost uses `BackendProfile.host_to_device_bw` for runtime↔executor
transfers and (when the same executor is involved) `device_internal_bw`.
For executor-to-executor transfers, V1 routes through the runtime's host
memory, so cost is `bytes / source.device_to_host_bw + bytes / dest.host_to_device_bw`.

### Worked example (the example from MX00)

A 4096x4096 f32 matmul with both inputs in host memory:

```
CPU (40 GFLOPS, 0 transfer cost — already on host):
    flops = 2 * 4096^3 = 137 GFLOPS
    compute_ns = 137e9 / 40 = 3.4e9 ns = 3.4 s
    transfer_ns = 0
    total = 3.4 s

Metal (5 TFLOPS f32, host→device 10 GB/s):
    flops = 137 GFLOPS
    compute_ns = 137e9 / 5000 = 27e6 ns = 27 ms
    transfer_ns = 2 * 64 MiB / 10 GB/s = 13.4 ms (in)
                  + 64 MiB / 10 GB/s = 6.7 ms (out, paid by next consumer)
    total ≈ 40 ms

Winner: Metal.
```

A 1024x1024 f32 add:

```
CPU:
    flops = 1M, compute_ns = 25 µs, transfer = 0
    total = 25 µs

Metal:
    flops = 1M, compute_ns = 200 ns + 1 µs launch
    transfer_ns = 2 * 4 MiB / 10 GB/s = 800 µs
    total ≈ 800 µs

Winner: CPU.
```

Both decisions fall out of one cost function, no special cases.

## Registry mechanics

The registry maintains `(ExecutorId, Box<dyn Transport>, BackendProfile,
healthy)` for each executor.  Lookups are O(1).  Transports report
events through `Transport::events()`; the runtime's event loop reacts
to:

- **`BufferLost`** — drop residency tracking for the affected buffer.
  If a graph in flight depended on it, the graph fails with
  `RuntimeError::BufferLost` and the caller can retry.
- **`ProfileUpdated`** — refresh the profile in the registry.  Future
  plans will see the new numbers.
- **`ShuttingDown`** — mark the executor unhealthy.  Plans skip it.

The runtime polls `Heartbeat` periodically (default: 1 second).  An
executor that misses three consecutive heartbeats is marked unhealthy
and skipped.  V1 does not auto-reconnect; the user calls `register`
again with a fresh transport.

## Async runtime

`Runtime::run` is synchronous in V1's API surface but built on async
internals.  The protocol's `Transport::request` returns a `Future`,
which `Runtime::run` resolves with a hand-rolled `block_on`:

```rust
fn block_on<F: Future>(mut f: F) -> F::Output {
    let waker = noop_waker();
    let mut cx = Context::from_waker(&waker);
    let mut pinned = unsafe { Pin::new_unchecked(&mut f) };
    loop {
        match pinned.as_mut().poll(&mut cx) {
            Poll::Ready(out) => return out,
            Poll::Pending    => {
                // V1 single-threaded: spin-yield until the LocalTransport completes
                std::thread::yield_now();
            }
        }
    }
}
```

This is sufficient because `LocalTransport` resolves immediately.  Network
transports will require a real reactor; that lands in the transport
crates themselves, not in `compute-runtime`.

## CPU executor

`compute-cpu` is a separate crate, but its profile and registration are
specified here because the runtime depends on it.  At `Runtime::new`:

1. Construct a `CpuExecutor` with a `LocalTransport` wrapper.
2. Run a 100 ms calibration: time `1024 * 1024` f32 adds and a 256x256
   f32 matmul.  Derive `gflops_f32`, `host_to_device_bw`, etc.
3. `register(transport)` the CPU executor.  By convention it gets
   `ExecutorId(0)`.

The CPU executor supports **every op** in the V1 set on **every dtype**
(f32, u8, i32).  This is what makes it the safety net.  Its
implementation is straight-line Rust, single-threaded in V1; SIMD and
multi-threading are V2 optimisations.

## Errors

```rust
pub enum RuntimeError {
    NoExecutorAvailable,
    BufferLost { buffer: BufferId },
    Plan(PlanError),
    Transport(TransportError),
    Executor { code: ErrorCode, message: String },
    InvalidGraph(matrix_ir::IrError),
    InvalidPlan(compute_ir::IrError),
}

pub enum PlanError {
    UnsupportedOp { op_index: u32, reason: String },
    UnsupportedDtype { op_index: u32, dtype: DType },
    InconsistentShape { op_index: u32, expected: Shape, actual: Shape },
}
```

## Test methodology

`compute-runtime` ships:

1. **Synthetic-profile tests** — register mock executors with hand-set
   profiles, plan small graphs, assert placement decisions match
   expectations.  Drives the planner without any real GPU.
2. **CPU-only round-trip** — `run` a graph through only the CPU
   executor, assert outputs match a hand-computed reference.
3. **Multi-executor planning** — register CPU + a mock GPU, vary the
   GPU's profile, assert the planner crosses the expected threshold
   between "stay on CPU" and "ship to GPU" as the cost model varies.
4. **Transfer insertion** — graphs that mix supported and unsupported
   ops; assert the planner inserts transfers and falls back to CPU for
   unsupported ops.
5. **Heartbeat / unhealthy** — kill a mock executor mid-flight; assert
   subsequent plans skip it and existing plans error cleanly.
6. **`plan` golden tests** — small graphs whose `ComputeGraph::dump()`
   form is asserted byte-for-byte against fixtures.  Catches accidental
   planner regressions.

Coverage target: **95%+**.  The async machinery is harder to cover
exhaustively but the planner and registry must be.

## Backend implementation guide

For an executor crate (e.g. `matrix-cpu`, `matrix-metal`):

1. Implement the `ExecutorRequest` handler — a single function
   `handle(req) -> response` with internal state for buffers and
   compiled kernels.
2. Construct a `BackendProfile` describing capabilities and costs.
3. Provide a `register(runtime: &mut Runtime)` helper that wraps the
   handler in a `LocalTransport` and calls `runtime.register`.

The backend never sees `MatrixIR` directly — only `ComputeGraph`s in
`Dispatch` requests.  This means a backend can be added without
touching `matrix-ir`, only `compute-ir`.

## Out of scope (V1)

- **Graph caching.**  Re-planning every call is fine for V1.  V2 caches
  `(graph_hash, registry_hash) -> ComputeGraph`.
- **Cost-model auto-tuning.**  V2 may run microbenchmarks per (op,
  shape, dtype) triple and refine `BackendProfile` over time.
- **Speculative execution.**  V2 may run small graphs on multiple
  executors in parallel and use the first to finish.
- **Job parallelism within a single graph.**  V1 executes ops in
  declared topological order.  V2 dispatches independent ops
  concurrently.

## Open questions

1. Should the planner emit warnings (e.g. "this graph is dominated by
   transfers; consider keeping intermediates on the GPU") as a
   first-class output of `plan`?  Useful but additive; can land in V2.
2. Is the calibration step worth its 100 ms cost on every `Runtime::new`?
   Alternative: a one-time calibration cached in `~/.cache/`.  V1
   chooses the simple path: re-calibrate every startup.
3. How does `register` handle two executors with overlapping
   capabilities (e.g. iGPU + dGPU)?  V1: both register, both are
   considered, planner picks per-op.  No special preference logic.
