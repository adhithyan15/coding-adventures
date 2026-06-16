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

### What this PR (MA-1) delivers vs. defers

- **Delivered now:** the `Array` value model + a correct CPU **reference**
  implementation of the core ops (so results are exact today), **and** the
  `matrix-ir` lowering + `matrix-runtime` cost-planner integration — i.e. each
  op can be lowered to a `matrix-ir` graph and *planned*, and the planner's
  backend choice is observable and tested (a large `matmul` with a GPU profile
  registered plans onto the GPU; a small one stays on the CPU).
- **Deferred (next MA item):** routing actual *execution* of the planned
  `ComputeGraph` through the registered executors. `matrix-runtime`'s public API
  currently *plans* (cost-based placement) but does not yet orchestrate
  execution end-to-end, and the GPU executor crates are not yet registered. Once
  that execution path lands, `array-runtime` switches its compute from the CPU
  reference path to the planned executors with **no API change** — the lowering
  and dispatch decision are already wired here. Until then the reference path
  guarantees correct results and the planner integration guarantees the dispatch
  decision is right.

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

## §6 Crate layout & dependencies

```
array-runtime/
  src/{lib.rs, value.rs, ops.rs, accel.rs}
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

- **MA-1** *(this PR)*: value model + CPU reference ops + matrix-ir lowering /
  cost-planner integration (this spec).
- **MA-2**: execution through the registered executors (CPU first), replacing the
  reference compute with the planned `ComputeGraph` run — same API.
- **MA-3+**: the MATLAB frontend (`matlab-lexer`/`matlab-parser`/`matlab-runtime`
  + the `matlab`/`octave` binaries) on top of `array-runtime`; then APL, etc.,
  per [`HML00`](HML00-historical-math-languages-roadmap.md).

## §8 References

Internal: [`HML00`](HML00-historical-math-languages-roadmap.md),
[`MX00`](MX00-matrix-execution-overview.md), [`MX01`](MX01-matrix-ir.md),
`matrix-runtime`, `executor-protocol`, `matrix-cpu`.
