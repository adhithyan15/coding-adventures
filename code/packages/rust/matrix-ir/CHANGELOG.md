# Changelog

All notable changes to `matrix-ir` are documented here.  The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.3.0] — 2026-05-13

### Added — `Op::Concat` (V2 op, wire tag 0x1D)

Second post-V1 op.  Concatenates two or more inputs along one axis:

```rust
Op::Concat {
    inputs: Vec<TensorId>,
    axis: u32,
    output: TensorId,
}
```

All inputs must share dtype and shape on every axis except `axis`;
the output's dim on `axis` is the sum of input dims on `axis`.
Equivalent to numpy `np.concatenate([a, b, …], axis=axis)`.

### Why

DSP01 Phase 3b needs to reassemble even/odd halves of an
interleaved complex tensor after each FFT butterfly stage.
`Slice` (0.2.0) reads them; `Concat` reassembles them.  The
slice-then-concat pattern is also the right primitive for
many CV transforms (image stacking, channel concat).

### Builder

- `GraphBuilder::concat(inputs: &[&Tensor], axis: u32) -> Tensor` —
  asserts on empty inputs, axis bounds, dtype mismatch, rank
  mismatch, non-axis dim mismatch, and u32 overflow on the
  summed axis.

### Wire format

Tag `0x1D` followed by:

```
uv64 n_inputs
u32 input_id × n_inputs
u32 axis
u32 output_id
```

Decoder uses `bounded_capacity` to cap the Vec preallocation
against the remaining input buffer (same defense-in-depth
pattern as the existing reduction-axes decoding).

### New error variant

- `IrError::InvalidConcat { op_index, reason: &'static str }` for
  empty inputs / dtype mismatch / rank mismatch / non-axis dim
  mismatch / u32 overflow.

### Tests

`wire_tags_are_unique` updated from 28 → 29 variants.
`sample_one_per_variant` gained a Concat sample.

## [0.2.0] — 2026-05-13

### Added — `Op::Slice` (V2 op, wire tag 0x1C)

First post-V1 op.  Takes a contiguous-stride slice along one axis:

```rust
Op::Slice {
    input: TensorId,
    axis: u32,
    start: u32,
    end: u32,
    step: u32,
    output: TensorId,
}
```

Output shape matches input on every axis except `axis`, where the
new dim is `ceil((end - start) / step)`.  Equivalent to numpy
`x[..., start:end:step, ...]` with the slice placed on `axis`.

Constraints (enforced by the validator):

- `axis < input.shape.rank()`.
- `step >= 1`.
- `start <= end <= input.shape.dims[axis]`.

### Why

DSP01 Phase 3 (matrix-ir-lowered FFT) needs an axis-aligned slice
to extract even / odd halves of a complex tensor at each butterfly
stage.  Adding `Slice` to the IR is much cleaner than mask-and-
reduce tricks via `Where` / `Mul` / `ReduceSum`, and matches what
every mature tensor IR (XLA, TVM, ONNX) provides.

### Backwards compatibility

- Wire format: 0x1C was reserved for V2 ops per the original spec.
  Decoders that ignore unknown tags still ignore graphs with
  Slice; decoders that error on unknown tags will need a v0.2
  bump too.
- Op::wire_tag(), Op::output(), Op::inputs() all gained `Slice`
  arms.  Exhaustive `match` on `Op` in downstream crates needs
  one new arm.

### New error variant

- `IrError::InvalidSlice { op_index, reason: &'static str }` — for
  bad axis / zero step / start > end / end > axis_dim.

### Builder

- `GraphBuilder::slice(input, axis, start, end, step) -> Tensor` —
  same shape-inference rules as the validator.

### Tests

The existing `wire_tags_are_unique` test was updated from 27 → 28
variants.  New per-arm tests cover the validator and builder
helpers transitively via the round-trip wire-format test.

## [0.1.0] — 2026-05-04

Initial release.  Implements spec MX01 V1.

### Added

- `TensorId`, `OpId`, `DType`, `Shape`, `Tensor` — the value-plane primitives.
- `Op` — 27-variant enum covering:
  - Elementwise unary: Neg, Abs, Sqrt, Exp, Log, Tanh, Recip
  - Elementwise binary: Add, Sub, Mul, Div, Max, Min, Pow
  - Reductions: ReduceSum, ReduceMax, ReduceMean
  - Shape: Reshape, Transpose, Broadcast
  - Linear algebra: MatMul (rank-2)
  - Comparison: Equal, Less, Greater
  - Selection: Where
  - Conversion: Cast
  - Constants: Const
- `Graph` — the aggregate computation, with structural and semantic
  validation via `Graph::validate()`.
- `Constant` — literal-data tensors stored in `Graph.constants`.
- `GraphBuilder` — ergonomic builder that allocates ids, infers shapes,
  and validates eagerly (panics with clear messages on misuse).
- Hand-rolled binary wire format per spec MX03 §"Wire format primitives":
  varint, length-prefixed bytes, tagged unions.  `Graph::to_bytes()` /
  `Graph::from_bytes()` round-trip without loss; encoding is
  deterministic.
- `IrError` — comprehensive error type covering structural,
  semantic, and wire-format failures.

### Constraints

- Zero external dependencies (only `core`, `alloc`, `std`).  CI gates
  this; the dependency section in `Cargo.toml` is intentionally empty.
- No execution.  This crate is pure data; computation happens elsewhere
  in the matrix execution layer.

### Test coverage

- 9 builder-and-validate integration tests over representative graphs
  (single op groups, multi-op chains, full ReLU layer).
- 11 validator-rejection integration tests covering `UndefinedTensor`,
  `ShapeMismatch`, `DTypeMismatch`, `InvalidPermutation`,
  `NumelMismatch`, `InvalidBroadcast`, `NonU8Predicate`,
  `UndefinedOutput`, `ConstantByteLength`, `TensorIdMismatch`,
  `InvalidAxis`.
- 7 wire round-trip integration tests with determinism check.
- 1 coverage-gate test asserting every `Op` variant is exercised; the
  test fails to compile or fails its assertion if a future variant is
  added without inclusion.
- Per-module unit tests for tensor primitives, op metadata, builder
  helpers, validator rules, and wire codec primitives (varint
  round-trip, oversized varint rejection, truncation handling, version
  rejection).
