# @coding-adventures/sir-runtime-array

The **N-D array/matrix runtime** imported by Semantic-IR-emitted
TypeScript/JavaScript for the array-language domain (MATLAB/Octave), per
[`code/specs/SIR22-array-matrix-semantic-ir.md`](../../../specs/SIR22-array-matrix-semantic-ir.md).

## What it is

SIR22 adds array/matrix `Expr`/`Stmt` variants to Semantic IR — `ArrayLit`,
`Range`, `MatMul`, `ElementwiseOp`, `Transpose`, `IndexGet`, `IndexSet` — so
a compiled MATLAB/Octave program can build, index, and operate on arrays
**at runtime**. This package is what those nodes compile down to: it's
meant to be bound as `__SirArray` at the emitted call sites
(`__SirArray.matmul(...)`, `.elementwise(...)`, `.range(...)`,
`.indexGet(...)`/`.indexSet(...)`).

It mirrors [`array_runtime`](../../rust/array-runtime) — the Rust crate the
MATLAB/Octave *runtimes* (not the compiled-to-JS path) already use —
field-for-field and algorithm-for-algorithm, so a program run through
`matlab-runtime` and the same program compiled to JS via this package agree
on every result.

| Concern | This package | Rust equivalent |
|---|---|---|
| Value model | `NDArray` (`shape`, `data: Float64Array`) | `array_runtime::value::Array` |
| Elementwise ops | `elementwise(op, a, b)`, 13 `ElementwiseOpKind`s | `array_runtime::ops::{BinOp, elementwise}` (12 — no `Pow` yet) |
| Matrix product | `matmul(a, b)` | `array_runtime::ops::matmul` |
| Transpose | `transpose(a, conjugate?)` | `array_runtime::ops::transpose` |
| Range | `range(start, stop, step?)` | `matlab-runtime`'s `eval_colon` |
| Indexing | `indexGet`/`indexSet` | (SIR-level only — no Rust runtime equivalent yet) |

## Why this package exists before any backend consumes it

This mirrors exactly how `sir-runtime-symbolic` (SIR23's runtime) landed
*before* the Stream-B codegen that calls it. `semantic-ir-to-javascript` and
`semantic-ir-to-typescript` currently hard-reject `Feature::NDArrays`/
`Feature::MatrixOps` and hit a deferred `panic!` at every SIR22 `Expr` match
arm (see those crates' `emit.rs`) — real codegen is a first-wave-JS
follow-up PR, not part of this package. Building the runtime primitives
first means that follow-up PR only has to wire up calls, not design an
array representation from scratch.

## How emitted code will use it (once the backend PR lands)

```ts
import * as __SirArray from "@coding-adventures/sir-runtime-array";

// A = [1, 2; 3, 4]
const A = __SirArray.fromRows([
  [1, 2],
  [3, 4],
]);

// A * A  (MatMul)
const A2 = __SirArray.matmul(A, A);

// A .+ 1  (ElementwiseOp, scalar broadcast)
const A_plus_1 = __SirArray.elementwise("Add", A, __SirArray.scalar(1));

// A(1, :)  (MATLAB 1-based -> already-resolved 0-based IndexGet)
const row0 = __SirArray.indexGet(A, [{ kind: "scalar", value: 0 }, { kind: "whole" }]);
```

## Scope: what this package covers today

- **`NDArray`** — the dense, column-major `f64` value model (`ndarray.ts`),
  plus `scalar`/`fromVec`/`fromRows`/`zeros` constructors and `get`/`set`
  element accessors.
- **`elementwise`** — all 13 `ElementwiseOpKind`s the SIR22 spec defines
  (`array_runtime::ops::BinOp`'s 12, plus `Pow`, which Rust's crate hasn't
  ported yet), with the same scalar-broadcast rule (either operand may be a
  scalar; otherwise shapes must match exactly — full NumPy/MATLAB
  broadcasting is out of scope, matching the Rust reference).
- **`matmul`** / **`transpose`** — mirror the Rust reference exactly,
  including column-major indexing arithmetic. `transpose`'s `conjugate`
  flag (MATLAB `'` vs `.'`) is accepted for API-shape parity with the spec
  but is a no-op today — there is no `Complex` value type yet.
- **`range`** — MATLAB-style `start:step:stop` materialization, with the
  same inclusive-stop tolerance and length cap `matlab-runtime`'s own
  `eval_colon` uses.
- **`indexGet`/`indexSet`** — `A(i)`/`A(i,j)` read and in-place write,
  covering the SIR22 spec's `Scalar`/`Whole`/`Range` `IndexArg` shapes.
  `IndexSet` mutates in place rather than returning a new array, matching
  MATLAB assignment semantics and the spec's own "statement, not
  expression" treatment of it.

## Deliberately out of scope

The SIR22 spec's "APL addendum" `Expr` variants (`Reduce`/`Scan`/
`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/
`Catenate`) are **not** implemented here, even though `array_runtime::ops`
already has Rust reference implementations for `reduce`/`scan`/`outer` — no
frontend crate emits these nine `Expr` variants yet, so porting them now
would be speculative rather than filling a real gap. They are a natural,
cleanly-scoped follow-up once `apl-to-semantic-ir`'s own JS-backend
consumption needs them.

`Complex`/`Rational` scalar support (shared `SirType`s with SIR23) is also
out of scope for the same reason.

## Security: bounded allocation

`checkedShapeSize` enforces `MAX_ELEMENTS` (2²⁶, matching
`matlab-runtime`'s own `MAX_RANGE`) — a compiled program's array shapes and
range bounds are runtime values, potentially attacker-influenced, not fixed
at compile time, so an absurd shape or range fails with a clean `Error`
rather than exhausting memory. Every function that allocates a buffer sized
from caller-supplied numbers (`zeros`, `fromRows`, `matmul`'s `m * n`,
`range`'s incremental loop) validates *before* calling `new Float64Array`,
not after — validating only inside `ndarray`'s constructor would be too
late, since the allocation attempt itself can throw an uncaught
`RangeError` or stall on a huge request before a cap ever gets a chance to
reject anything cleanly. `checkedShapeSize` also rejects negative and
non-integer dimensions, closing a variant where two negative dimensions
multiply to a small, cap-passing positive product.

## Where it fits

`matlab-to-semantic-ir` / `octave-to-semantic-ir` → `semantic-ir` (SIR22
`Expr` variants) → *(follow-up)* `semantic-ir-to-typescript` /
`semantic-ir-to-javascript` → emitted code that imports this package. See
[`code/specs/HML01-math-to-semantic-ir.md`](../../../specs/HML01-math-to-semantic-ir.md).

## Development

```sh
npm install
npx tsc --noEmit
npx vitest run --coverage
```
