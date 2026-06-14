# MX00 — Matrix Execution Layer: Architecture Overview

## Status

Draft — V1 specification.  This document is the umbrella for the four
sub-specs MX01–MX04.  No code lands until all five specs are accepted.

## Why this layer exists

Today the repo has a real-GPU compute stack (`metal-compute`, `cuda-compute`,
`gpu-runtime`, `image-gpu-core`) that hand-writes one shader per backend per
operation.  That works, but it does not scale.

Three forces converge on a different shape:

1. **Neural-network track.**  NN layers (dense, conv, normalisation, attention)
   are tensor algebra.  The same primitives that make image transforms fast on
   a GPU make NN training fast on a GPU.  Writing them once, in one vocabulary,
   is the only way both tracks are maintainable.

2. **Backend explosion.**  Metal and CUDA are not enough.  We want to be able
   to target Vulkan, OpenGL compute, OpenCL, WebGPU, and eventually
   special-purpose ASICs.  Hand-writing every operation per backend is N×M
   work where N is operations and M is backends.  The IR collapses that to
   N + M.

3. **Memory transfer cost.**  GPUs are fast at compute and slow at host↔device
   transfers.  A scheduler that does not see the whole graph cannot fuse
   passes or avoid round trips.  The IR makes transfers visible so they can be
   minimised.

The matrix execution layer is the **narrow waist** between domain libraries
(image processing, neural networks, signal processing, BLAS) and hardware
backends (Metal, CUDA, Vulkan, OpenGL, OpenCL, WebGPU, CPU, future ASICs):

```
  image-ops     nn-layers    fft     blas     [your code]
       \      |      |      |      |
        v     v      v      v      v
       ───   MatrixIR  (the tensor algebra bytecode)   ───
                          |
                       planner
                          |
                          v
       ───   ComputeIR  (placed graph: devices, transfers)   ───
                          |
                  executor-protocol
                          |
                          v
        cpu     metal    cuda    vulkan    opengl    wgpu    asic
```

Domain libraries above emit `MatrixIR`.  The planner lowers `MatrixIR` to
`ComputeIR` by assigning ops to backends, weighing compute cost against
transfer cost.  Backends, near or far, are reached through the
`executor-protocol`, a transport-pluggable wire format that works in-process,
over a socket, over ZeroMQ, over NATS, or anywhere else bytes can flow.

The contract is:  *anything that crosses the boundary between the runtime
and an executor must be a serializable message*.  Nothing — closures, trait
objects, callbacks, borrowed references — crosses except as bytes.  The
local case is "the wire is a function call."  The remote case uses real
bytes.  The discipline is identical.

## The four specs

| Spec | Crate | Purpose |
|------|-------|---------|
| **MX01** | `matrix-ir` | Pure tensor algebra IR.  Domain libraries emit this. |
| **MX02** | `compute-ir` | Placed IR with explicit devices and `Transfer` ops. |
| **MX03** | `executor-protocol` | Wire format, message types, `Transport` trait. |
| **MX04** | `matrix-runtime` | Planner, backend registry, cost model. |

A backend (CPU, Metal, CUDA, …) is then a thin crate that implements the
executor side of the protocol.  V1 ships:

- `matrix-cpu` — reference executor, supports every op, the always-available
  fallback.
- One GPU executor (Metal **or** CUDA — implementer's choice, the other
  follows in V1.1).
- `matrix-transport-local` — in-process transport (function calls).

V1 is **deliberately small**.  Things explicitly out of scope and reserved
for later:

- Network transports (TCP, ZMQ, NATS) — *designed for*, not shipped.
- Multi-host / cluster execution.
- Multi-GPU on the same host.
- Fusion of consecutive ops into one kernel.
- Conv2d, pooling, FFT as structured primitives — composable from the V1
  op set, optimised structured forms come later.
- Dynamic shapes — V1 is static-shape only.
- Autograd — the IR makes a backward pass possible later, not in V1.

## Zero-dependency mandate

All five new crates have **no external dependencies**.  They use only
`core`, `alloc`, and `std`.  No `serde`, no `bincode`, no `postcard`, no
`tokio`, no `async-std`, no `futures`, no `libloading`, no `bytemuck`.

Concrete consequences:

- **Wire format** is hand-rolled (varint + length-prefixed bytes + tagged
  unions).  Format spec is written in MX03 and is implementation-agnostic;
  a Python or JavaScript client could implement the same bytes from the
  spec alone.
- **Async** uses a hand-rolled minimal `block_on` (~50 lines, single-threaded
  poll loop with a no-op waker) for the local transport.  Future network
  transports get their own minimal poll loops; we do not depend on a
  general-purpose async runtime.
- **Hashing** for kernel cache keys uses `std::collections::hash_map::DefaultHasher`
  (SipHash, in std).
- **GPU bindings** reuse the existing `objc-bridge` and `cuda-compute`
  `DynLib` machinery, both already zero-dep.

Every new crate's `Cargo.toml` has an empty `[dependencies]` section.  CI
enforces this with a check that fails the build if any of the five crates
gain an external dependency.

## Design discipline: information does not leak

Several things stay strictly above the matrix execution layer and are not
allowed to influence its design:

- **"Image" and "channel"** — an image is a rank-3 `[H, W, 4]` u8 tensor.
  The matrix layer sees a rank-3 u8 tensor.  Channels-last vs channels-first
  is the image library's decision.
- **"Batch" and "feature"** — a neural-network activation is a tensor.  The
  matrix layer sees a tensor.  Batch dimension is the NN library's
  decision.
- **"Time" and "frequency"** — a spectrogram is a rank-2 tensor.  The
  matrix layer sees a rank-2 tensor.

This is the narrow-waist principle: the IR's vocabulary is *only* tensor
algebra.  Domain semantics live above the line.  Hardware semantics live
below the line.  Information flows through the IR as shape and dtype, not
as labels.

## Correctness model

Every operation in the V1 op set has a **CPU reference implementation** in
`matrix-cpu`.  The cross-backend test harness runs the same `MatrixIR`
graph on every available backend and asserts that results match within a
declared tolerance.  Tolerances are dtype-specific:

- f32 results: `|a - b| <= 1e-5 * max(|a|, |b|, 1)` (relative + absolute mix)
- f16 results: `|a - b| <= 1e-3 * max(|a|, |b|, 1)`
- u8 / i32 / i64 results: bit-exact

When an op is added to the IR, three things happen in lockstep:

1. The op gains a CPU reference in `matrix-cpu`.
2. Each existing backend gains a lowering for the op (or returns
   `Unsupported`, which falls through to CPU).
3. A test fixture exercising the op is added to the cross-backend harness.

This is the rule that keeps the layer honest as it grows.

## Execution model

A graph runs in three phases:

```
build:   user code constructs a MatrixIR graph
plan:    runtime lowers MatrixIR to ComputeIR using backend profiles
run:     runtime ships ComputeIR to executors via the transport
```

The build phase is synchronous and cheap.  The plan phase is synchronous
and runs the cost model.  The run phase is asynchronous because a network
transport will eventually need it; for V1 it is async-typed but local
execution returns immediately-ready futures.

## Open questions deferred to implementation

These do not block specification acceptance but will need decisions when
code starts:

1. **Trait surface for an executor** — should the executor implement a
   trait directly, or only the message protocol?  Leaning protocol-only,
   because that enforces the discipline.
2. **Buffer aliasing** — when can two `BufferId`s refer to the same
   memory?  V1 says never.  V2 may relax for in-place ops.
3. **Versioning** — the wire format includes a version byte.  How do we
   handle version mismatches?  V1: hard error.  V2: per-message
   compatibility shims.

## Reading order

To understand the layer end-to-end, read the specs in order:

1. **MX00** (this document) — context and constraints.
2. **MX01** — what `MatrixIR` is and what ops it has.
3. **MX02** — how `ComputeIR` differs and what placement means.
4. **MX03** — how runtime and executors talk to each other.
5. **MX04** — how the planner works and how backends register.

Each sub-spec assumes the reader has read this overview and the specs
before it.
