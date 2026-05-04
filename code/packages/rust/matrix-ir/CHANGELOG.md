# Changelog

All notable changes to `matrix-ir` are documented here.  The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

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
