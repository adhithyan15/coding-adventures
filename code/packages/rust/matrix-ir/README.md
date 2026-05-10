# matrix-ir

Pure tensor algebra IR — the upper IR of the matrix execution layer.

This is the implementation of spec **MX01**.  See:

- [`code/specs/MX00-matrix-execution-overview.md`](../../specs/MX00-matrix-execution-overview.md) — architecture
- [`code/specs/MX01-matrix-ir.md`](../../specs/MX01-matrix-ir.md) — the IR contract

## What it is

`matrix-ir` is a pure data-plane crate that defines:

- `TensorId`, `OpId`, `DType`, `Shape`, `Tensor` — the value plane
- `Op` — a 27-variant enum covering elementwise math, reductions, shape
  ops, matmul, comparison, selection, conversion, and constants
- `Graph`, `Constant` — the aggregate computation
- `GraphBuilder` — an ergonomic builder that allocates ids, infers
  shapes, and validates eagerly
- `Graph::validate()` — structural and semantic checks
- `Graph::to_bytes()` / `Graph::from_bytes()` — versioned binary wire
  format that round-trips without loss

There is **no execution** in this crate.  Execution lives in
`compute-runtime` and the executor crates (CPU, Metal, CUDA, …).
`matrix-ir` is pure data — easy to reason about, easy to test, easy
to serialise to a remote executor.

## Where it sits

```
  domain libraries:   image-ops    nn-layers    fft    blas
                          \  |       |     |    |
                           v  v      v     v    v
                       ───   matrix-ir   ───              ← this crate
                                  |
                              planner
                                  |
                                  v
                       ───   compute-ir   ───
                                  |
                          executor-protocol
                                  |
                                  v
                          cpu / metal / cuda / wgpu / asic
```

Domain libraries (image processing, neural-network layers, signal
processing, BLAS) construct `matrix-ir` graphs to describe what they
want computed.  The runtime lowers them to a placed `compute-ir` and
ships them to executors.

## A worked example

```rust
use matrix_ir::{DType, GraphBuilder, Shape};

let mut g = GraphBuilder::new();
let x = g.input(DType::F32, Shape::from(&[1, 4]));
let w = g.input(DType::F32, Shape::from(&[4, 2]));
let b = g.input(DType::F32, Shape::from(&[1, 2]));
let zero = g.constant(DType::F32, Shape::from(&[1, 2]), vec![0u8; 8]);

let xw    = g.matmul(&x, &w);
let xwb   = g.add(&xw, &b);
let y     = g.max(&xwb, &zero);    // ReLU = max(x, 0)
g.output(&y);

let graph = g.build().unwrap();
graph.validate().unwrap();

// Serialise for transport over a wire (or for caching)
let bytes = graph.to_bytes();
let same  = matrix_ir::Graph::from_bytes(&bytes).unwrap();
assert_eq!(graph, same);
```

## V1 op set (27 ops)

| Group | Ops |
|------|------|
| Elementwise unary | Neg, Abs, Sqrt, Exp, Log, Tanh, Recip |
| Elementwise binary | Add, Sub, Mul, Div, Max, Min, Pow |
| Reductions | ReduceSum, ReduceMax, ReduceMean |
| Shape | Reshape, Transpose, Broadcast |
| Linear algebra | MatMul (rank-2 only in V1) |
| Comparison | Equal, Less, Greater (output dtype is U8) |
| Selection | Where |
| Conversion | Cast |
| Constants | Const |

V1 dtypes: `F32`, `U8`, `I32`.  `F16` and `I64` are reserved in the
wire format for V2.

## Design principles

1. **Tensor algebra only.**  No "image", "channel", "batch", "feature".
   Domain labels live above this layer.  An RGBA image is a `[H, W, 4]`
   u8 tensor; whether dim 2 means "channels" is the image library's
   business.
2. **Static shapes.**  V1 only.  V2 introduces symbolic shapes.
3. **SSA over tensors.**  Each tensor produced by an op gets a fresh
   `TensorId`; tensors are immutable.  This makes downstream residency
   tracking and dead-code elimination tractable.
4. **Closed under serialisation.**  Every value has a defined wire
   form.  Local and remote executors see the same bytes.
5. **Zero external dependencies.**  Only `core`, `alloc`, `std`.

## Testing

```
cargo test -p matrix-ir
```

Test methodology (per spec MX01 §"Test methodology"):

1. Builder + validate over representative graphs (one per op group plus
   combinations).
2. Validator rejection over deliberately malformed graphs, asserting
   the right `IrError` variant fires.
3. Wire round-trip — every test graph is serialised, deserialised, and
   asserted equal to the original; encoding is asserted deterministic.
4. Coverage gate — meta-test asserts every `Op` variant is exercised.

## Out of scope (V1)

Reserved for later versions:

- Dynamic shapes
- f16 / bf16 / i64 / complex64
- Conv2d / Pool / Softmax / LayerNorm as primitives (composed from V1
  ops in the meantime)
- Custom-kernel escape hatch
- Autograd
- Implicit broadcasting on binary ops

See spec MX01 §"Out of scope" for the migration plan for each.
