# MX01 — `matrix-ir`: The Tensor Algebra IR

## Status

Draft — V1 specification.  Read [MX00](MX00-matrix-execution-overview.md) first.

## Purpose

`matrix-ir` is the upper IR — what domain libraries (image processing, NN
layers, signal processing, BLAS) emit when they want computation done.  It
describes *what* to compute as a directed acyclic graph of tensor algebra
operations.  It says nothing about *where* that computation happens; that is
`compute-ir`'s job (MX02).

This crate is **pure data** — types, builders, validators, serialisers.
It performs no computation, allocates no GPU memory, and depends on no
backend.  It can be compiled and exercised on any platform without a GPU.

## Design principles

1. **Tensor algebra only.**  The vocabulary contains shapes, dtypes, and
   mathematical operations.  No "image", no "batch", no "channel".  Domain
   labels live above this layer.

2. **Static shapes.**  Every tensor's shape is known at graph build time.
   Dynamic shapes are a V2 problem.

3. **SSA over tensors.**  Each tensor produced by an op gets a fresh
   `TensorId`.  Tensors are immutable values; ops never mutate.  This is
   what makes residency tracking and dead-code elimination tractable
   downstream.

4. **Closed under serialisation.**  Every value in the IR — `Tensor`,
   `Op`, `Graph` — has a defined wire form.  Backends, including remote
   ones, see the same bytes.

5. **No external dependencies.**  `core` + `alloc` + `std` only.

## Core types

```rust
pub struct TensorId(pub u32);
pub struct OpId(pub u32);

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
pub enum DType {
    F32,
    U8,
    I32,
}

#[derive(Clone, PartialEq, Eq, Debug)]
pub struct Shape {
    /// Dimensions in row-major / leftmost-major order.  Empty Vec means scalar.
    pub dims: Vec<u32>,
}

#[derive(Clone, Debug)]
pub struct Tensor {
    pub id:    TensorId,
    pub dtype: DType,
    pub shape: Shape,
}

/// A graph is a topologically-ordered sequence of ops plus their input
/// constants and declared inputs/outputs.
pub struct Graph {
    pub inputs:    Vec<Tensor>,        // tensors supplied by the caller at run time
    pub outputs:   Vec<TensorId>,      // tensors the caller wants back
    pub ops:       Vec<Op>,            // topologically ordered
    pub tensors:   Vec<Tensor>,        // every tensor referenced, including outputs of ops
    pub constants: Vec<Constant>,      // literal data baked into the graph
}

pub struct Constant {
    pub tensor: TensorId,
    pub bytes:  Vec<u8>,               // dtype-encoded little-endian
}
```

`f16` and `i64` are deferred to V2.  `bool` is represented as `U8` (0 or 1)
to keep the dtype set small.  This is consistent with how WebGPU and most
shader languages model booleans on tensors.

## V1 op set

The V1 op set is **27 operations**, chosen to span elementwise math, reductions,
shape manipulation, linear algebra, comparison, selection, type conversion,
and constants.  Anything an image filter, a neural-network layer, or a signal
processing pipeline does at the tensor level decomposes into this set.

### Elementwise unary (7)

```rust
Neg(Tensor) -> Tensor       // -x
Abs(Tensor) -> Tensor       // |x|
Sqrt(Tensor) -> Tensor      // sqrt(x)
Exp(Tensor) -> Tensor       // e^x
Log(Tensor) -> Tensor       // ln(x)
Tanh(Tensor) -> Tensor      // tanh(x)
Recip(Tensor) -> Tensor     // 1 / x
```

Output shape and dtype match input.  `Sqrt`, `Log`, `Recip` produce
implementation-defined results on out-of-domain inputs (NaN / Inf for
floats; backend-defined for ints, generally division-by-zero traps).

### Elementwise binary (7)

```rust
Add (Tensor, Tensor) -> Tensor
Sub (Tensor, Tensor) -> Tensor
Mul (Tensor, Tensor) -> Tensor
Div (Tensor, Tensor) -> Tensor
Max (Tensor, Tensor) -> Tensor
Min (Tensor, Tensor) -> Tensor
Pow (Tensor, Tensor) -> Tensor
```

Both inputs must have the same dtype and the same shape.  Implicit
broadcasting is **not allowed** in V1 — callers must insert explicit
`Broadcast` ops.  This keeps the IR's semantics narrow and shifts the
broadcasting decision (and the memory cost it implies) up to the
domain library or the optimiser, where it belongs.

### Reductions (3)

```rust
ReduceSum (Tensor, axes: Vec<u32>, keep_dims: bool) -> Tensor
ReduceMax (Tensor, axes: Vec<u32>, keep_dims: bool) -> Tensor
ReduceMean(Tensor, axes: Vec<u32>, keep_dims: bool) -> Tensor
```

Reduces over the given axes.  If `keep_dims` is false, the reduced axes
are removed from the shape; if true, they are kept as size-1.  Empty `axes`
means reduce-all (scalar output unless `keep_dims`).

`ReduceMean` for integer dtypes uses integer division (truncation toward
zero); use `Cast` to f32 first if you want fractional results.

### Shape manipulation (3)

```rust
Reshape (Tensor, new_shape: Shape) -> Tensor
Transpose(Tensor, perm: Vec<u32>) -> Tensor
Broadcast(Tensor, target_shape: Shape) -> Tensor
```

`Reshape` requires `new_shape.numel() == input.shape.numel()`.  `Transpose`
requires `perm` to be a permutation of `0..rank`.  `Broadcast` requires
that each dimension of the input either equals the corresponding target
dimension or is 1 (in which case it is replicated).

### Linear algebra (1)

```rust
MatMul(a: Tensor, b: Tensor) -> Tensor
```

Both inputs must be rank-2 with matching inner dimensions: `a` is `[m, k]`,
`b` is `[k, n]`, output is `[m, n]`.  Both must have the same float dtype
(f32 in V1).  Higher-rank batched matmul is composed by reshaping or by
loop-emitting in the domain library, **not** baked into V1.

### Comparison (3)

```rust
Equal  (Tensor, Tensor) -> Tensor   // dtype = U8, value 0 or 1
Less   (Tensor, Tensor) -> Tensor   // dtype = U8, value 0 or 1
Greater(Tensor, Tensor) -> Tensor   // dtype = U8, value 0 or 1
```

Inputs must have matching shapes and dtypes.  Output dtype is always U8.

### Selection (1)

```rust
Where(predicate: Tensor, true_value: Tensor, false_value: Tensor) -> Tensor
```

`predicate` must have dtype U8.  All three inputs must have matching
shapes.  `true_value` and `false_value` must have matching dtype, which
becomes the output dtype.  Equivalent to a per-element `predicate ? t : f`.

### Type conversion (1)

```rust
Cast(Tensor, dtype: DType) -> Tensor
```

Numerical conversion.  Float-to-int truncates toward zero; int-to-float
is exact for U8 and I32 within f32 range.

### Constants (1)

```rust
Const(constant_id: u32) -> Tensor
```

Materialises a tensor from the graph's `constants` table.  The shape and
dtype are taken from the constant's declared `Tensor`.  Literal data lives
in `Graph.constants` rather than being inlined into ops, both to keep ops
small and to allow the planner to share constant buffers between ops.

## Op enum

All 27 ops are encoded as a single Rust enum:

```rust
pub enum Op {
    // Elementwise unary
    Neg   { input: TensorId, output: TensorId },
    Abs   { input: TensorId, output: TensorId },
    Sqrt  { input: TensorId, output: TensorId },
    Exp   { input: TensorId, output: TensorId },
    Log   { input: TensorId, output: TensorId },
    Tanh  { input: TensorId, output: TensorId },
    Recip { input: TensorId, output: TensorId },

    // Elementwise binary
    Add { lhs: TensorId, rhs: TensorId, output: TensorId },
    Sub { lhs: TensorId, rhs: TensorId, output: TensorId },
    Mul { lhs: TensorId, rhs: TensorId, output: TensorId },
    Div { lhs: TensorId, rhs: TensorId, output: TensorId },
    Max { lhs: TensorId, rhs: TensorId, output: TensorId },
    Min { lhs: TensorId, rhs: TensorId, output: TensorId },
    Pow { lhs: TensorId, rhs: TensorId, output: TensorId },

    // Reductions
    ReduceSum  { input: TensorId, axes: Vec<u32>, keep_dims: bool, output: TensorId },
    ReduceMax  { input: TensorId, axes: Vec<u32>, keep_dims: bool, output: TensorId },
    ReduceMean { input: TensorId, axes: Vec<u32>, keep_dims: bool, output: TensorId },

    // Shape
    Reshape   { input: TensorId, new_shape: Shape, output: TensorId },
    Transpose { input: TensorId, perm: Vec<u32>, output: TensorId },
    Broadcast { input: TensorId, target_shape: Shape, output: TensorId },

    // LinAlg
    MatMul { a: TensorId, b: TensorId, output: TensorId },

    // Comparison
    Equal   { lhs: TensorId, rhs: TensorId, output: TensorId },
    Less    { lhs: TensorId, rhs: TensorId, output: TensorId },
    Greater { lhs: TensorId, rhs: TensorId, output: TensorId },

    // Selection
    Where { predicate: TensorId, true_value: TensorId, false_value: TensorId, output: TensorId },

    // Conversion
    Cast { input: TensorId, dtype: DType, output: TensorId },

    // Constants
    Const { constant: u32, output: TensorId },
}
```

Each op carries the `output: TensorId` it produces, even though it could
be inferred positionally.  This is for two reasons:

1. **Round-trip stability.**  Serialising and deserialising preserves the
   exact `TensorId` numbering, which is what tests assert against.
2. **Validation locality.**  A validator can check op-by-op without
   walking back through the graph to recompute output ids.

## Builder API

Constructing graphs by hand is verbose and error-prone.  `matrix-ir`
exposes a builder that allocates tensor ids, infers shapes, validates as
it goes, and produces a `Graph`:

```rust
pub struct GraphBuilder { /* ... */ }

impl GraphBuilder {
    pub fn new() -> Self;

    pub fn input(&mut self, dtype: DType, shape: Shape) -> Tensor;
    pub fn constant(&mut self, dtype: DType, shape: Shape, bytes: Vec<u8>) -> Tensor;

    pub fn add(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor;
    pub fn mul(&mut self, lhs: &Tensor, rhs: &Tensor) -> Tensor;
    pub fn matmul(&mut self, a: &Tensor, b: &Tensor) -> Tensor;
    // ... one method per op ...

    pub fn output(&mut self, t: &Tensor);
    pub fn build(self) -> Result<Graph, IrError>;
}
```

Example — a simple `y = relu(x @ w + b)` layer:

```rust
let mut g = GraphBuilder::new();
let x = g.input(DType::F32, Shape::from(&[1, 784]));
let w = g.input(DType::F32, Shape::from(&[784, 128]));
let b = g.input(DType::F32, Shape::from(&[1, 128]));

let xw    = g.matmul(&x, &w);
let xwb   = g.add(&xw, &b);
let zero  = g.constant(DType::F32, Shape::from(&[1, 128]), vec![0u8; 128 * 4]);
let y     = g.max(&xwb, &zero);   // ReLU = max(x, 0)

g.output(&y);
let graph = g.build().unwrap();
```

Note how the graph is shape-explicit (no implicit broadcasting; `b` is
already `[1, 128]` to match `xw`).

## Validation

`Graph::validate()` runs a structural and semantic check:

### Structural rules

1. Every `TensorId` referenced by an op exists in `Graph.tensors`.
2. Every constant referenced by `Const` exists in `Graph.constants` and
   the constant's `Tensor.id` matches its index.
3. The `output` of each op equals the next unallocated `TensorId` at the
   time the op runs (single-assignment).
4. Ops are topologically ordered: every input of op N is produced by an
   op `<N` or is a graph input or constant.
5. Every `output` listed in `Graph.outputs` has been produced by some op.

### Semantic rules (per op)

For each op, the validator checks that input shapes and dtypes satisfy
the op's contract and that the declared output `Tensor` matches what the
op would produce.  Mismatches raise `IrError::ShapeMismatch { op_index,
expected, actual }` or `IrError::DTypeMismatch { ... }`.

Examples:

- `Add { lhs, rhs, output }`: lhs and rhs must have identical shape and
  dtype; output must equal that shape and dtype.
- `MatMul { a, b, output }`: both rank-2; `a.shape[1] == b.shape[0]`;
  output shape `[a.shape[0], b.shape[1]]`; output dtype matches inputs.
- `Reshape { input, new_shape, output }`: `new_shape.numel() ==
  input.shape.numel()`; output dtype matches input.

The validator is exhaustive — every op variant has a check.  Adding an
op without adding a check is a compile error because the validator
match expression has no default arm.

### Constant rules

For each `Constant`:

1. `bytes.len() == tensor.shape.numel() * tensor.dtype.size_bytes()`.
2. Bytes are little-endian dtype-encoded.

## Wire format

`Graph` is serialisable to a versioned binary blob.  The layout is defined
in MX03 §"Wire format primitives" and §"Graph serialisation"; this spec
guarantees only that the bytes round-trip:

```
Graph::to_bytes() -> Vec<u8>
Graph::from_bytes(&[u8]) -> Result<Graph, IrError>
```

The wire format is **versioned**.  V1 produces format version 1; readers
that see a higher version error rather than silently reinterpret.

## Test methodology

`matrix-ir` ships its own test suite covering:

1. **Builder + validate**: construct ~30 representative graphs (one per op
   plus combinations) and assert `validate()` accepts them.
2. **Validator rejection**: construct deliberately malformed graphs (shape
   mismatch, dangling tensor id, non-permutation transpose, …) and assert
   the validator names the right error.
3. **Wire round-trip**: every graph in the suite is serialised, deserialised,
   and asserted equal to the original.
4. **Coverage gate**: a meta-test asserts that the suite touches every
   variant of `Op`, fails the build if a variant is missed.

Coverage target: **100%** of the public API.  This crate is small and
pure; nothing should be untested.

## Out of scope (for V1)

Reserved for later versions, with notes on how V1 leaves room:

- **Dynamic shapes** — V2 introduces `Dim::Static(u32)` vs `Dim::Symbolic(SymId)`.
  V1's `Shape` becomes a special case where every `Dim` is `Static`.
- **f16, bf16, i64, complex64** — adding to `DType` is additive.  Existing
  ops gain new dtype combinations as backends grow capability.
- **Conv2d, Pool, Softmax, LayerNorm, Attention as primitives** — V2.  V1
  composes them from the existing op set.  Backends can pattern-match later.
- **CustomKernel escape hatch** — a Op variant carrying per-backend source.
  Deliberately omitted from V1 to force the IR to express enough.
- **Autograd** — V2 adds a transformation pass that builds a backward graph
  from a forward graph.  V1's SSA structure makes this clean.
- **Implicit broadcasting on binary ops** — V2.  V1 forces explicit
  `Broadcast` to keep the IR's semantics minimal.

## Open questions

1. Should the builder validate eagerly (each `add()` call returns
   `Result<Tensor, IrError>`) or only at `build()` time?  Eager catches
   errors at the call site; lazy keeps the API ergonomic.  V1 leans
   **eager via panics** in debug, lazy `build()` in release — matches how
   most builders in this repo work.
2. Should `Const` carry the bytes inline (in the Op) or by reference into
   `Graph.constants`?  Currently by reference; the alternative inflates
   `Op` and complicates serialisation.  Sticking with by-reference.
3. Is `Pow` in V1 worth it?  It is the only V1 op whose backend lowering
   is annoying (transcendental, often emits a library call).  Keeping it
   because gamma correction in image processing needs it.
