/**
 * sir-runtime-array — the SIR22 N-D array/matrix runtime imported by
 * Semantic-IR-emitted TypeScript/JavaScript, bound as `__SirArray` at call
 * sites (see `code/specs/SIR22-array-matrix-semantic-ir.md` and this
 * package's README for the exact call shapes a future backend PR wires
 * up). A compiled MATLAB/Octave program's `ArrayLit`, `Range`, `MatMul`,
 * `ElementwiseOp`, `Transpose`, `IndexGet`, and `IndexSet` IR nodes all
 * become calls into this package at runtime.
 *
 * ## Why this package exists before any backend consumes it
 *
 * This mirrors exactly how `sir-runtime-symbolic` (SIR23's runtime) landed
 * *before* the Stream-B JS/TS codegen that calls it: `semantic-ir-to-javascript`
 * and `semantic-ir-to-typescript` currently hard-reject `Feature::NDArrays`/
 * `Feature::MatrixOps` and hit a deferred `panic!` at every SIR22 `Expr`
 * match arm (see those crates' `emit.rs`) — real codegen (`__SirArray.matmul(...)`,
 * `.elementwise(...)`, `.range(...)`, `.indexGet(...)`/`.indexSet(...)`) is a
 * first-wave-JS follow-up PR, not part of this package. Building the runtime
 * primitives first means that follow-up PR only has to *wire up calls*, not
 * design an array representation from scratch.
 *
 * ## Scope: what this package covers today
 *
 * - **`NDArray`** (`./ndarray`) — the dense, column-major `f64` value model,
 *   plus `scalar`/`fromVec`/`fromRows`/`zeros` constructors and `get`/`set`
 *   element accessors. Mirrors `array_runtime::value::Array`
 *   (`code/packages/rust/array-runtime/src/value.rs`) field-for-field.
 * - **`elementwise`** (`./elementwise`) — the 13 `ElementwiseOpKind`s the
 *   SIR22 spec defines (`array_runtime::ops::BinOp`'s 12, plus `Pow`, which
 *   Rust's crate hasn't ported yet), with the same scalar-broadcast rule.
 * - **`matmul`** / **`transpose`** (`./matmul`, `./transpose`) — mirror
 *   `array_runtime::ops::matmul` / `transpose` exactly, including their
 *   column-major indexing arithmetic.
 * - **`range`** (`./range`) — MATLAB-style `start:step:stop` materialization,
 *   with the same inclusive-stop tolerance and length cap `matlab-runtime`'s
 *   own `eval_colon` uses.
 * - **`indexGet`/`indexSet`** (`./indexing`) — `A(i)`/`A(i,j)` read and
 *   in-place write, covering the SIR22 spec's `Scalar`/`Whole`/`Range`
 *   `IndexArg` shapes.
 *
 * ## Deliberately out of scope
 *
 * The SIR22 spec's "APL addendum" `Expr` variants (`Reduce`/`Scan`/
 * `OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/`IndexOf`/`Ravel`/
 * `Catenate`) are **not** implemented here, even though `array_runtime::ops`
 * already has Rust reference implementations for `reduce`/`scan`/`outer` —
 * no frontend crate emits these nine `Expr` variants yet (per that spec's
 * own "No frontend crate consumes any of these nine variants yet" note), so
 * porting them now would be speculative rather than filling a real gap.
 * They are a natural, cleanly-scoped follow-up once `apl-to-semantic-ir`'s
 * own JS-backend consumption needs them.
 *
 * `Complex`/`Rational` scalar support (shared `SirType`s with SIR23) is also
 * out of scope — `transpose`'s `conjugate` flag is accepted for API-shape
 * parity with the spec but is a no-op today, matching `array-runtime`'s own
 * real-only scope.
 */

export { ndarray, checkedShapeSize, scalar, fromVec, fromRows, zeros, ndims, isScalar, nrows, ncols, get, set, MAX_ELEMENTS, type NDArray } from "./ndarray.js";
export { elementwise, type ElementwiseOpKind } from "./elementwise.js";
export { matmul } from "./matmul.js";
export { transpose } from "./transpose.js";
export { range } from "./range.js";
export { indexGet, indexSet, type IndexArg } from "./indexing.js";
