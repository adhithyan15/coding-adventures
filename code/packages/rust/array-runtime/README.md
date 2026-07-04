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
- **Reductions** `sum`, `mean`, `max`, `min` (fixed-operator, whole-array).
- **Generalized reduce/scan/outer-product** (AR-2) — `reduce`, `scan`, and
  `outer` each take an arbitrary `BinOp`, not a fixed one, motivated by APL's
  `/`, `\`, and `∘.` operators (see
  [`MA05-apl-language.md`](../../../specs/MA05-apl-language.md) §2):
  - `reduce(op, &a)` folds `a`'s last axis (`+/v` sums a vector to a scalar;
    on a `[r, c]` matrix, folds each row across its columns to a `[r]`
    vector).
  - `scan(op, &a)` is the same fold but keeps every intermediate result
    (same shape as `a` — a running total, for `op = Add`).
  - `outer(op, &a, &b)` applies `op` to every pair, producing rank
    `rank(a) + rank(b)` — two vectors `[m]`/`[n]` become a `[m, n]` matrix.
    `matmul` is `outer`'s `op = Mul` **-then-summed** special case; the raw
    product (no summing) is what `outer` adds.

```rust
use coding_adventures_array_runtime::{Array, ops, ops::BinOp};

let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
let c = ops::matmul(&a, &Array::eye(2)).unwrap(); // a · I == a
assert_eq!(c.data(), a.data());

// +/[1,2,3,4] = 10 (reduce); +\[1,2,3,4] = [1,3,6,10] (scan, every partial sum).
let v = Array::from_vec(vec![1.0, 2.0, 3.0, 4.0]);
assert_eq!(ops::reduce(BinOp::Add, &v).unwrap().data(), &[10.0]);
assert_eq!(ops::scan(BinOp::Add, &v).unwrap().data(), &[1.0, 3.0, 6.0, 10.0]);

// [1,2] ∘.× [10,100] = [[10,100],[20,200]] (outer product, no summing).
let p = Array::from_vec(vec![1.0, 2.0]);
let q = Array::from_vec(vec![10.0, 100.0]);
let table = ops::outer(BinOp::Mul, &p, &q).unwrap();
assert_eq!(table.shape(), &[2, 2]);
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

`plan_backend` takes the element `DType`, because the dtype affects *placement*,
not just the math: a backend with no `f64` kernel (the GPU profile advertises
`gflops_f64 = 0`) costs an `f64` op as ∞, so `f64` work stays on the CPU even when
the same `f32` op would be shipped to the accelerator.

```rust
use coding_adventures_array_runtime::{Kernel, BinOp, DType, plan_backend};

// A big f32 matmul is worth shipping to an accelerator…
assert_eq!(plan_backend(Kernel::MatMul, DType::F32, &[256, 256], &[256, 256], true).unwrap(), "gpu");
// …a tiny elementwise op is not…
assert_eq!(plan_backend(Kernel::Elementwise(BinOp::Add), DType::F32, &[2, 2], &[2, 2], true).unwrap(), "cpu");
// …and the same big matmul in f64 stays on the CPU (the GPU has no f64 kernel).
assert_eq!(plan_backend(Kernel::MatMul, DType::F64, &[256, 256], &[256, 256], true).unwrap(), "cpu");
```

### 4. End-to-end execution (`exec.rs`, MA-2 + MXF-3)

`execute()` doesn't just *plan* the graph — it **runs** it through
[`matrix-cpu`](../matrix-cpu)'s executor, returning real numeric results from the
same pipeline a GPU would use. It bridges two boundaries: **precision** and
**memory order** (column-major ↔ the executor's row-major — elementwise passes
through, `matmul` transposes at the edges).

As of **MX12 / MXF-3**, `matrix-ir` has a `DType::F64`, so `execute()` lowers
`f64` arrays to an **`F64`** graph and crosses the boundary as **8-byte** doubles
— **no `f64 → f32 → f64` round-trip**. The result is therefore **bit-exact** with
the `ops` reference path, even on values `f32` cannot represent (e.g. `1 + 2^-40`,
which the old `f32` path collapsed to `1.0`).

```rust
use coding_adventures_array_runtime::{Array, Kernel, execute, execute_sum, ops};

let a = Array::from_rows(vec![vec![1.0, 2.0], vec![3.0, 4.0]]).unwrap();
let b = Array::from_rows(vec![vec![5.0, 6.0], vec![7.0, 8.0]]).unwrap();

// Planned and executed on the CPU executor — [[19,22],[43,50]].
let c = execute(Kernel::MatMul, &a, &b).unwrap();
assert_eq!(c.get(0, 0), Some(19.0));

// Full f64 precision: a value f32 can't hold survives the executor round-trip,
// so the executed result matches the reference path bit-for-bit.
let x = 1.0 + 2f64.powi(-40); // distinct from 1.0 in f64, == 1.0 as f32
let p = Array::from_vec(vec![x]);
let one = Array::from_vec(vec![1.0]);
let executed = execute(Kernel::Elementwise(coding_adventures_array_runtime::BinOp::Sub), &p, &one).unwrap();
assert_eq!(executed.data()[0].to_bits(), ops::sub(&p, &one).unwrap().data()[0].to_bits());

// `execute_sum` runs an f64 whole-array reduction on the same path.
let total = execute_sum(&Array::from_vec(vec![1.0, 2.0, 3.0])).unwrap();
assert_eq!(total.data()[0], 6.0);
```

## Status

- **MA-1 (merged):** the value model, the CPU reference ops (exact `f64` results),
  and the full `matrix-ir` lowering + cost-planner integration — the backend
  choice is observable and tested.
- **MA-2 (merged):** `execute()` plans **and runs** the lowered graph on the CPU
  executor for elementwise + `matmul`, cross-checked against the reference path.
  The same path runs on a GPU executor the moment one is registered.
- **MXF-3 (this release):** the `f64` path. `execute()` lowers `f64` arrays to an
  `F64` graph and uses an 8-byte codec — no `f32` round-trip — so the executed
  result is **bit-exact** with the reference path. `execute_sum()` adds an `f64`
  whole-array reduction on the same path. `plan_backend`/`build_graph` now take an
  explicit `DType` (breaking signature change for `plan_backend`).
- **AR-2 (this release):** generalized `reduce`/`scan`/`outer` kernels,
  parameterized over `BinOp` rather than fixed to one operator — the
  prerequisite APL's `/`, `\`, and `∘.` operators need (`apl-runtime`, MA-4e).
  CPU-reference only for now, same as `matmul`/`transpose` before MA-2 wired
  them through `exec`.
- **Next (MXF-4):** R's `s-runtime` adopts this `f64` path for `%*%` and the
  matrix ops, replacing its hand-written loops; then executing scalar-broadcast /
  `transpose`, and registering real CUDA/Metal executor crates.

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
