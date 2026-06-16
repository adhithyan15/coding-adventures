# Changelog

All notable changes to `array-runtime` are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.1.0] — 2026-06-16

Initial release — **MA-1**, Wave 0 of the historical math-languages roadmap
([`HML00`](../../../specs/HML00-historical-math-languages-roadmap.md), spec
[`MA00`](../../../specs/MA00-array-runtime.md)). The shared N-D numeric-array
substrate that the array/matrix-language frontends (MATLAB first) sit on.

### Added — the `Array` value model (`value.rs`)

A dense, rectangular, **column-major** `f64` array (`data: Vec<f64>`,
`shape: Vec<usize>`). Element `(r, c)` of an `[nrows, ncols]` matrix lives at
flat index `c * nrows + r` — Fortran/MATLAB order, chosen so a future MATLAB
frontend maps directly (`reshape`, linear indexing, `[a; b]` literals).

- Constructors: `scalar`, `from_vec` (1-D), `from_rows` (transposes row-major
  input into column-major store), `from_shape` (validated), `zeros`, `ones`,
  `filled`, `eye`.
- Accessors: `shape`, `data`, `ndims`, `len`, `is_empty`, `is_scalar`, `nrows`,
  `ncols`, `get(r, c)`.
- `Display`: MATLAB-style right-aligned rows; integer-valued doubles print
  without a trailing `.0`.

### Added — CPU reference operations (`ops.rs`)

Correct, dependency-free implementations that produce values today:

- Elementwise `add`/`sub`/`mul`/`div` with scalar broadcasting (either operand
  may be scalar; otherwise shapes must match). NaN/Inf propagate naturally.
- Linear algebra: `matmul` (`[m,k]·[k,n] → [m,n]`, column-major) and `transpose`.
- Reductions: `sum`, `mean`, `max`, `min`.

### Added — GPU dispatch by lowering (`accel.rs`)

The cost-based backend decision. An op lowers to a `matrix-ir::Graph` via
`GraphBuilder` and is planned by `matrix-runtime::Runtime`, which places each op
on the cheapest available backend (CPU/CUDA/Metal) from a FLOP + transfer cost
model.

- `Kernel` (`Elementwise(BinOp)` / `MatMul`) and `plan_backend(kernel, a, b,
  with_gpu) -> String` returns the executor *kind* the planner chose
  (`"cpu"`/`"gpu"`), so the dispatch decision is observable and tested.
- `gpu_profile()` registers a synthetic accelerator (≈100× CPU throughput, real
  host↔device transfer cost) to exercise the cost model. `matrix-ir` works in
  `f32`/`u32`-dim `Shape`; `array-runtime` works in `f64` and converts at the
  lowering boundary.

### Deferred to MA-2

Routing actual *execution* of the planned `ComputeGraph` through registered
executors. `matrix-runtime` currently plans placement but does not yet
orchestrate execution end-to-end; when it does, compute switches from the CPU
reference path to the planned graph with no public API change.

### Tests

24 tests (23 unit + 1 doctest): the value model (shapes, column-major storage,
constructors, bounds, `Display`), every reference op (including broadcasting,
NaN/Inf, dimension-mismatch errors, transpose involution), and the dispatch
decision (CPU-only fallback, small op stays on CPU, large matmul moves to GPU).
