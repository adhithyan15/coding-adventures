# matrix-runtime

The brain of the matrix execution layer.  Planner, executor registry,
and cost-model-driven placement.

This is the implementation of spec **MX04** (originally proposed as
`compute-runtime`; renamed because that name was already taken by the
G05 GPU-runtime simulator).  See:

- [`code/specs/MX04-compute-runtime.md`](../../specs/MX04-compute-runtime.md) — contract
- [`code/specs/MX00-matrix-execution-overview.md`](../../specs/MX00-matrix-execution-overview.md) — architecture

## What this crate does

Lowers `matrix_ir::Graph` to `compute_ir::ComputeGraph` by routing each
op to the cheapest available executor.

```
   matrix-ir   →   [matrix-runtime planner]   →   compute-ir   →   executors
```

The planner is a four-pass algorithm:

1. **Capability filter** — for each op, find executors that support
   it (op kind + dtype).  Fall back to CPU if none.
2. **Greedy cost minimisation** — pick the executor with the lowest
   `compute + transfer-in` cost for each op in topological order.
3. **Transfer insertion** — insert `PlacedOp::Transfer` whenever an
   input's residency doesn't match its consumer's executor.
4. **Lifetime annotation** — `Alloc` before first use, `Free` after
   last use.

## Worked example

```rust
use matrix_ir::{DType, GraphBuilder, Shape};
use matrix_runtime::{Runtime, BackendProfile};

let cpu = BackendProfile {
    kind: "cpu".to_string(),
    supported_ops: 0xFFFF_FFFF, supported_dtypes: 0x07,
    gflops_f32: 40, gflops_u8: 40, gflops_i32: 40,
    host_to_device_bw: 100, device_to_host_bw: 100, device_internal_bw: 100,
    launch_overhead_ns: 0, transport_latency_ns: 0,
    on_device_mib: 8 * 1024, max_tensor_rank: 16, max_dim: u32::MAX,
};
let mut rt = Runtime::new(cpu);

let gpu = BackendProfile {
    kind: "gpu".to_string(),
    gflops_f32: 5_000, /* …125× faster… */
    host_to_device_bw: 10, /* …slow PCIe… */
    ..rt.executors()[0].profile.clone()
};
rt.register("gpu", gpu);

// Build a graph
let mut g = GraphBuilder::new();
let a = g.input(DType::F32, Shape::from(&[4096, 4096]));
let b = g.input(DType::F32, Shape::from(&[4096, 4096]));
let c = g.matmul(&a, &b);
g.output(&c);
let g = g.build().unwrap();

// Plan — large matmul ships to GPU
let placed = rt.plan(&g).unwrap();
print!("{}", placed.dump());
```

## Cost-model-driven routing

Each backend exposes a `BackendProfile` (re-exported from
`executor-protocol`):

- Compute throughput (GFLOPS per dtype)
- Transfer bandwidth (host↔device)
- Launch overhead, transport latency
- Capability bitsets (op kind, dtype)

The planner naturally:

- Keeps small ops on CPU (transfer cost > GPU speedup).
- Ships large ops to GPU (GPU speedup > transfer cost).
- Falls back to CPU when a backend can't handle an op (capability
  mismatch).
- Skips unhealthy executors.

These behaviours are all consequences of one cost function — no
special-case logic.

## Inspectable

`ComputeGraph::dump()` shows every transfer with its estimated cost.
The planner's reasoning is visible:

```
ComputeGraph (format v1, 2 inputs, 1 outputs, 11 ops)
  inputs:
    t0: f32 [4096, 4096]   @ exec 0 buf 0
    t1: f32 [4096, 4096]   @ exec 0 buf 1
  ops:
    [00]  alloc            buf 2 @ exec 1   (64.0 MiB)
    [01]  transfer t0      exec 0 buf 0  ->  exec 1 buf 2   (64.0 MiB, ≈ 6.4 ms)
    [02]  alloc            buf 3 @ exec 1   (64.0 MiB)
    [03]  transfer t1      exec 0 buf 1  ->  exec 1 buf 3   (64.0 MiB, ≈ 6.4 ms)
    [04]  alloc            buf 4 @ exec 1   (64.0 MiB)
    [05]  compute  matmul  exec 1 t0 t1 -> t2   (≈ 27 ms)
    ...
```

## V1 scope

V1 ships **the planner** and **the registry**.  The end-to-end execution
path (driving `ComputeGraph`s through transports to executors and
collecting outputs) lands when the first executor crate (`matrix-cpu`)
arrives.  The `Runtime` API exposes:

- `Runtime::new(cpu_profile)` — bootstrap with CPU fallback
- `Runtime::register(kind, profile)` — add an executor
- `Runtime::plan(graph)` — lower MatrixIR to ComputeIR
- `Runtime::set_healthy(id, healthy)` — mark unhealthy executors
- `Runtime::update_profile(id, profile)` — refresh on `ProfileUpdated`
- `Runtime::executors()` — inspect the registry

## Testing

```
cargo test -p matrix-runtime
```

Test methodology (per spec MX04 §"Test methodology"):

- **Synthetic-profile tests** — register mock executors with hand-set
  profiles, plan small graphs, assert placement decisions.
- **CPU-only round-trip** — assert no transfers are inserted for
  CPU-only graphs.
- **Multi-executor planning** — small ops stay on CPU, large ops ship
  to GPU.
- **Capability fallback** — i32 op falls back to CPU when GPU lacks
  i32 support.
- **Health tracking** — unhealthy executor is skipped.
- **Cost-model monotonicity** — slower PCIe means GPU is harder to
  reach.

## Zero dependencies

```
$ cargo tree -p matrix-runtime
matrix-runtime v0.1.0
├── compute-ir v0.1.0
│   └── matrix-ir v0.1.0
├── executor-protocol v0.1.0
│   ├── compute-ir v0.1.0
│   └── matrix-ir v0.1.0
└── matrix-ir v0.1.0
```

Only the upstream matrix-execution-layer crates as path deps.

## Out of scope (V1)

Reserved for later:

- **End-to-end `run()`** — wires transports into the runtime; lands with `matrix-cpu`.
- **Graph caching** — re-planning every call is fine for V1; V2 caches `(graph_hash, registry_hash)`.
- **Auto-tuning** — V2 microbenchmarks per (op, shape, dtype).
- **Speculative execution** — V2 runs small graphs on multiple executors.
- **Within-graph parallelism** — V1 executes ops in topological order.
