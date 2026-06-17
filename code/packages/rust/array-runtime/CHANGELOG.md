# Changelog

All notable changes to `array-runtime` are documented here. The format is based
on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/).

## [0.2.0] — 2026-06-16

**MA-2 — end-to-end execution.** MA-1 planned the lowered graph and produced
values from the CPU reference path; MA-2 closes the loop by **running** the
planned graph through `matrix-cpu`'s `CpuExecutor`.

### Added — the `exec` module

- `execute(kernel, &a, &b) -> Result<Array, String>`: plans the lowered
  `matrix-ir` graph and executes the placed `ComputeGraph` on the CPU executor,
  returning real numeric results — the same pipeline a GPU would use. Covers
  elementwise `add`/`sub`/`mul`/`div` (equal shapes) and `matmul` (`[m,k]·[k,n]`).
- Internal `run_graph_on_cpu` orchestrator (allocate one buffer per planner
  buffer-id → rewrite residencies → upload constants/inputs → `Dispatch` →
  download outputs), adapted from the established Rust/Python binding loop.
- Two boundary conversions: **precision** (`f64` ↔ `f32` little-endian bytes,
  since `matrix-ir` has no `f64` dtype) and **memory order** (column-major ↔
  row-major; elementwise passes through, `matmul` transposes operands in and the
  result out). `MAX_TOTAL_BUFFER_BYTES` (4 GiB) caps a crafted graph's footprint
  before any allocation.
- `execute` re-exported at the crate root.

Every executed result is cross-checked against the `ops` reference path in tests,
so the two can't silently diverge. `transpose`, the reductions, scalar-broadcast
execution, and real GPU executor crates remain for follow-ups.

### Tests

38 tests (37 unit + 1 doctest); the `exec` suite proves elementwise + `matmul`
execute and agree with the reference path (incl. a `matmul` layout round-trip
`a · I == a` and non-square shapes), plus orchestrator validation paths.

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

### Hardened (pre-merge security review)

Defensive overflow handling so crafted dimensions can't wrap to an
under-sized buffer or a mis-costed dispatch decision:

- `Array::from_shape` computes the element count with checked multiplication
  and rejects shapes whose product overflows `usize` (previously an unchecked
  `product()` could wrap to a small count that passed the length check).
- `Array::filled` (and `zeros`/`ones`/`eye` through it) and `Array::from_rows`
  size their backing buffer with `checked_mul` — a deterministic error/panic on
  overflow instead of a release-mode wrap that would under-allocate.
- `ops::matmul` checks the `m * n` output size; `ops::transpose` allocates from
  the input's element count (which already fit in memory).
- `accel::shape_of` converts `usize` dims to `matrix-ir`'s `u32` with a
  *checked* cast, rejecting dims past `u32::MAX` rather than silently truncating
  (which would make the planner cost the wrong op and return a wrong backend).

### Tests

28 tests (27 unit + 1 doctest): the value model (shapes, column-major storage,
constructors, bounds, `Display`, overflow rejection), every reference op
(including broadcasting, NaN/Inf, dimension-mismatch errors, transpose
involution), and the dispatch decision (CPU-only fallback, small op stays on
CPU, large matmul moves to GPU, oversized dim rejected).
