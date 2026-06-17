# array-runtime

The shared **N-dimensional numeric-array substrate** for the historical
array/matrix languages — MATLAB first, then Octave, Scilab, APL, J, IDL. It is
**Wave 0** of the historical math-languages roadmap
([`HML00`](../../../specs/HML00-historical-math-languages-roadmap.md); full spec:
[`MA00`](../../../specs/MA00-array-runtime.md)).

The array languages differ mostly in *syntax*; underneath they share one value
model — dense, rectangular, column-major numeric arrays with broadcasting,
reshaping, linear algebra, and reductions. `array-runtime` builds that model
once so each language frontend is a thin lexer/parser/runtime on top — exactly
as R became a thin frontend over the shared S evaluator.

## What it provides

### 1. The `Array` value model (`value.rs`)

A dense, **column-major** (Fortran/MATLAB order) `f64` array. Element `(r, c)` of
an `[nrows, ncols]` matrix lives at flat index `c * nrows + r`. Column-major is
deliberate: MATLAB's `reshape`, linear indexing, and `[a; b]` literal semantics
all assume it.

```rust
use coding_adventures_array_runtime::Array;

let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap(); // 2x2
let i = Array::eye(2);                                                   // identity
let z = Array::zeros(3, 3);
println!("{a}"); // MATLAB-style aligned rows
```

Constructors: `scalar`, `from_vec`, `from_rows`, `from_shape`, `zeros`, `ones`,
`filled`, `eye`. Accessors: `shape`, `ndims`, `nrows`, `ncols`, `len`, `get`,
`data`, plus a MATLAB-ish `Display`.

### 2. CPU reference operations (`ops.rs`)

Correct, dependency-free implementations that produce values **today**:

- **Elementwise** `add`/`sub`/`mul`/`div` with scalar broadcasting (NaN/Inf
  propagate naturally).
- **Linear algebra** `matmul` (`[m,k]·[k,n] → [m,n]`) and `transpose`.
- **Reductions** `sum`, `mean`, `max`, `min`.

```rust
use coding_adventures_array_runtime::{Array, ops};

let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
let c = ops::matmul(&a, &Array::eye(2)).unwrap(); // a · I == a
assert_eq!(c.data(), a.data());
```

### 3. GPU dispatch by lowering (`accel.rs`)

The hardware story is **dispatch by lowering, not by syntax**. An op lowers to a
[`matrix-ir`](../matrix-ir) tensor-algebra graph and is handed to
[`matrix-runtime`](../matrix-runtime)'s cost-based planner, which places each op
on the cheapest available backend — **CPU, CUDA, or Metal** — from a FLOP +
host↔device-transfer cost model. CPU is the always-available fallback. So a large
`matmul` runs on the GPU exactly when its FLOPs beat the transfer overhead, and a
small one stays on the CPU — with **no language-level GPU code and no "use GPU"
keyword**.

```rust
use coding_adventures_array_runtime::{Kernel, BinOp, plan_backend};

// A big matmul is worth shipping to an accelerator…
assert_eq!(plan_backend(Kernel::MatMul, &[256, 256], &[256, 256], true).unwrap(), "gpu");
// …a tiny elementwise op is not.
assert_eq!(plan_backend(Kernel::Elementwise(BinOp::Add), &[2, 2], &[2, 2], true).unwrap(), "cpu");
```

### 4. End-to-end execution (`exec.rs`, MA-2)

`execute()` doesn't just *plan* the graph — it **runs** it through
[`matrix-cpu`](../matrix-cpu)'s executor, returning real numeric results from the
same pipeline a GPU would use. It bridges two boundaries: precision (`f64` ↔
`f32`, since `matrix-ir` has no `f64` dtype yet) and memory order (column-major ↔
the executor's row-major — elementwise passes through, `matmul` transposes at the
edges).

```rust
use coding_adventures_array_runtime::{Array, Kernel, BinOp, execute};

let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();

// Planned and executed on the CPU executor — [[19,22],[43,50]].
let c = execute(Kernel::MatMul, &a, &b).unwrap();
assert_eq!(c.get(0, 0), Some(19.0));
```

## Status

- **MA-1 (merged):** the value model, the CPU reference ops (exact `f64` results),
  and the full `matrix-ir` lowering + cost-planner integration — the backend
  choice is observable and tested.
- **MA-2 (this release):** `execute()` plans **and runs** the lowered graph on the
  CPU executor for elementwise + `matmul`, cross-checked against the reference
  path. The same path runs on a GPU executor the moment one is registered.
- **Next:** executing `transpose`/reductions, scalar-broadcast execution, and
  registering real CUDA/Metal executor crates.

## Where it sits in the stack

```
matlab-runtime / octave-runtime / apl-runtime  (future frontends)
                     │
                array-runtime          ← this crate (shared value model + ops + dispatch)
                     │
        matrix-ir → matrix-runtime → executors (cpu / cuda / metal)
```

## Testing

```sh
cargo test -p coding-adventures-array-runtime
```
