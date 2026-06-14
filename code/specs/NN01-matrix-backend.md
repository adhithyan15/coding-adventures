# NN01: Matrix Backend Interface and Bytecode Lowering

Status: draft

## Purpose

NN01 describes the first optimized execution path for Neural Graph VM bytecode:
lowering scalar forward bytecode into matrix-oriented operations that can run on
different backends.

The important boundary is this:

- The graph describes structure.
- NN00 bytecode describes portable execution.
- NN01 matrix plans describe vectorized execution.
- A backend performs the actual matrix work.

The VM must not be hard-wired to one matrix implementation. The current
TypeScript matrix package is the reference CPU backend, but future backends
should be able to target WebGPU, native GPU libraries, browser SIMD, WASM, or
hardware simulators without changing neural-network graph authoring code.

## Layering

```text
neural-network graph primitives
  -> NN00 bytecode compiler
    -> scalar reference interpreter
    -> NN01 matrix plan compiler
      -> MatrixBackend interface
        -> TypeScript Matrix CPU backend
        -> WebGPU backend
        -> native/WASM backend
        -> accelerator simulator backend
```

The scalar interpreter remains the correctness reference. Matrix lowering is an
execution optimization, not the source of truth.

## Backend Contract

Backends implement a small shape-preserving matrix API. The VM owns neural
semantics; the backend owns storage, kernels, device placement, and transfer
costs.

```text
MatrixBackend<M>
  from_rows(rows) -> M
  to_rows(matrix) -> number[][]
  column(values) -> M
  constant(value, rows, cols = 1) -> M
  add(left, right) -> M
  scale(matrix, scalar) -> M
  dot(left, right) -> M
  map(matrix, fn) -> M
  to_column(matrix) -> number[]
```

`M` is deliberately opaque. A CPU backend can use a simple `number[][]`, the
current TypeScript `Matrix`, or a packed typed array. A GPU backend can use a
device buffer handle. The VM should only pass values back to the backend
interface.

## Matrix Plan

A matrix plan is a backend-neutral lowering of NN00 forward bytecode. It groups
the scalar op stream into higher-level vector operations while preserving source
metadata for visualization and debugging.

Initial v0 matrix plan opcodes:

| Opcode | Meaning |
| --- | --- |
| `LOAD_INPUT_MATRIX` | Load one runtime input as a column matrix. Scalars broadcast to the current batch size. |
| `LOAD_CONST_MATRIX` | Load a scalar constant as a repeated column matrix. |
| `WEIGHTED_SUM_MATRIX` | Compute `sum(source_matrix * edge_weight)` for a graph node. |
| `ACTIVATE_MATRIX` | Apply an elementwise activation. |
| `STORE_OUTPUT_MATRIX` | Copy a column matrix into a named output. |

The compiler must retain the `sourceNode` and `sourceEdge` relationships already
present in NN00 bytecode. A visualizer should be able to show both the original
graph edge and the matrix operation it became.

## Batch Semantics

The first matrix runner treats each runtime input as a column:

```text
scalar input:      x = 4       -> [[4]]
batched input:     x = [4, 8]  -> [[4], [8]]
constant/bias:     b = 1       -> [[1], [1]]
```

All array inputs in a single run must have the same length. Scalar inputs are
broadcast to that length. This lets the same compiled plan run one sample or a
small batch without changing the graph.

## Backend Swapping Rules

The VM runtime may depend on the backend interface, not on a concrete matrix
class. Concrete adapters are leaf code:

- `TypeScriptMatrixBackend` adapts the existing `matrix` package.
- A future `WebGpuMatrixBackend` can keep values on the GPU.
- A future `ComputeUnitMatrixBackend` can run against the compute-unit simulator.

Backends may fuse work internally, but the visible contract must preserve the
same outputs as the scalar interpreter for the same bytecode and inputs.

## Non-Goals for v0

- Training and backpropagation lowering.
- Automatic dense-layer fusion.
- Device scheduling.
- Graph-language parsing.
- Hidden layer architecture search.

Those can build on the same contract later. v0 only proves that bytecode can
lower to swappable matrix operations for forward inference.
