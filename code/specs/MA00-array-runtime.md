# MA00 — array-runtime: the shared N-D array substrate for numeric languages

## Status

Active spec. `array-runtime` is **Wave 0** of the historical math-languages
roadmap ([`HML00`](HML00-historical-math-languages-roadmap.md)): the N-dimensional
numeric-array value model that every numerical/array-language frontend (MATLAB,
Octave, Scilab, APL, J, IDL) sits on — the array-family analogue of what
`r-vector`/`s-runtime` are to the statistical-language family. It is built once
and reused by all of them.

## §1 Why a shared array substrate

The array languages differ mostly in *syntax*; underneath they share one value
model — dense, rectangular, column-major numeric arrays with broadcasting,
reshaping, linear algebra, and reductions. Building that once means each
frontend (MATLAB first) is a thin lexer/parser/runtime over `array-runtime`,
exactly as R was a thin frontend over the shared S evaluator.

## §2 Modern hardware: GPU dispatch by lowering, not by syntax

Per [`HML00`](HML00-historical-math-languages-roadmap.md) §4, array operations
do **not** hand-roll GPU code. They lower to **`matrix-ir`** (a tensor-algebra
DAG) and submit the graph to **`matrix-runtime`**, whose cost-based planner
assigns each op to the cheapest available backend — **CPU, CUDA, or Metal** —
from a FLOP + dtype + host↔device-transfer cost model, with CPU as the
always-available fallback. So `A * B` runs on the GPU exactly when the matrices
are large enough to beat transfer overhead, and on the CPU otherwise, with no
language-level GPU code and no "use GPU" keyword.

### Delivery status (MA-1 → MA-2)

- **MA-1 (merged):** the `Array` value model + a correct CPU **reference**
  implementation of the core ops (so results are exact today), **and** the
  `matrix-ir` lowering + `matrix-runtime` cost-planner integration — i.e. each
  op can be lowered to a `matrix-ir` graph and *planned*, and the planner's
  backend choice is observable and tested (a large `matmul` with a GPU profile
  registered plans onto the GPU; a small one stays on the CPU).
- **MA-2 (this PR) — execution closed:** `array-runtime` now **plans and runs**
  the lowered graph through `matrix-cpu`'s `CpuExecutor`, returning real numeric
  results from the same pipeline a GPU would use (see §5.1 and the `exec`
  module). `matrix-runtime`'s public API exposes `plan()` but no end-to-end
  `run()`; the executor-driving loop (allocate buffers → upload → `Dispatch` →
  download) is the well-trodden one the Rust/Python and Node bindings already
  use, so `array-runtime` orchestrates it directly. `execute()` covers
  **elementwise** (`add`/`sub`/`mul`/`div` on equal shapes) and **`matmul`**;
  every executed result is cross-checked against the reference path in tests so
  the two cannot silently diverge.
- **Still deferred:** executing `transpose` and the reductions (trivial /
  axis-aware lowering), scalar-broadcast execution, and registering real GPU
  executor crates (CUDA/Metal). The reference path in `ops` remains the exact
  `f64` answer until a `matrix-ir` `f64` dtype lands; `execute()` matches it to
  `f32` precision.

This staging keeps every PR correct and mergeable while building the GPU path
incrementally.

## §3 The value model

```rust
pub struct Array {
    data: Vec<f64>,       // column-major (Fortran/MATLAB order)
    shape: Vec<usize>,    // dims; [] = scalar, [n] = vector, [r, c] = matrix
}
```

- **Column-major** storage, matching MATLAB/Fortran (so a future MATLAB frontend
  is a direct map and `reshape`/linear-indexing match).
- Constructors: `scalar`, `from_vec` (1-D), `from_rows`/`from_cols` (2-D),
  `zeros`, `ones`, `eye`, `filled`.
- Accessors: `shape`, `ndims`, `nrows`, `ncols`, `len`, `get(idx)`, `data()`.
- `Display`: MATLAB-style aligned rows.

## §4 Operations (CPU reference; GPU-dispatch-ready)

- **Elementwise** `add/sub/mul/div` with **broadcasting** (scalar↔array, and
  equal-shape), NaN/Inf propagating naturally.
- **Linear algebra**: `matmul` (2-D `[m,k] · [k,n] → [m,n]`), `transpose`.
- **Reductions**: `sum`, `mean`, `max`, `min` (whole-array; axis reductions
  follow).
- **Shape**: `reshape`.

Each op has a CPU reference body **and** a `matrix-ir` lowering (§5). The
reference body is what produces values today; the lowering is what the planner
uses to decide hardware placement.

## §5 Lowering + planning (the GPU brain)

`accel` lowers an op to a `matrix-ir::Graph` via `matrix_ir::GraphBuilder` and
plans it with `matrix_runtime::Runtime`:

```text
Array op  ──▶  matrix_ir::GraphBuilder (add/mul/matmul/…)  ──▶  Graph
                                                                  │
                                  matrix_runtime::Runtime::plan(&graph)
                                                                  │
                                                                  ▼
                                              compute_ir::ComputeGraph
                                              (each op placed on cpu/cuda/metal
                                               by the cost model)
```

`plan_backend(op, shapes) -> &'static str` returns the executor *kind* the
planner chose ("cpu"/"gpu"), so the dispatch decision is testable. The runtime
is constructed with `matrix_cpu::profile()` (CPU always present); a GPU profile
can be registered to exercise the cost model. (`matrix-ir` uses `f32`/`Shape`
with `u32` dims; `array-runtime` works in `f64` and converts at the lowering
boundary.)

## §5.1 Execution (MA-2 — the loop closed)

`exec::execute(kernel, &a, &b) -> Array` runs the lowered graph for real. It
plans the graph (§5), then drives the placed `ComputeGraph` through
`matrix-cpu`'s `CpuExecutor` via the executor protocol:

```text
build_graph ──▶ Runtime::plan ──▶ ComputeGraph
                                       │  allocate one CpuExecutor buffer per
                                       │  planner buffer-id, rewrite residencies
                                       ▼
        AllocBuffer · UploadBuffer(constants, inputs) · Dispatch · DownloadBuffer
                                       │
                                       ▼
                              f32 output bytes ──▶ Array
```

Two boundary conversions: **precision** (`f64` ↔ `f32` little-endian bytes, since
`matrix-ir` has no `f64` dtype yet) and **memory order** (`array-runtime` is
column-major, `matrix-cpu`'s kernels are row-major — elementwise is positional
so passes through untouched; `matmul` transposes each operand into row-major in,
and the result back into column-major out). A `MAX_TOTAL_BUFFER_BYTES` cap (4
GiB) rejects crafted giant-shape graphs before any allocation. The same path
runs on a GPU executor the moment one is registered — execution and the §5
dispatch decision are the same pipeline.

## §6 Crate layout & dependencies

```
array-runtime/
  src/{lib.rs, value.rs, ops.rs, accel.rs, exec.rs}
```

```toml
[dependencies]
matrix-ir = { path = "../matrix-ir" }
matrix-runtime = { path = "../matrix-runtime" }
compute-ir = { path = "../compute-ir" }
executor-protocol = { path = "../executor-protocol" }
matrix-cpu = { path = "../matrix-cpu" }
```

## §7 Roadmap fit

- **MA-1** *(merged)*: value model + CPU reference ops + matrix-ir lowering /
  cost-planner integration (this spec).
- **MA-2** *(this PR)*: execution through the CPU executor — `execute()` plans
  and runs the lowered graph end-to-end for elementwise + `matmul`, cross-checked
  against the reference path (§5.1). Same crate, additive API.
- **MA-3+**: the MATLAB frontend (`matlab-lexer`/`matlab-parser`/`matlab-runtime`
  + the `matlab`/`octave` binaries) on top of `array-runtime`; then APL, etc.,
  per [`HML00`](HML00-historical-math-languages-roadmap.md).

## §8 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MX00`](MX00-matrix-execution-overview.md), [`MX01`](MX01-matrix-ir.md),
`matrix-runtime`, `executor-protocol`, `matrix-cpu`.
