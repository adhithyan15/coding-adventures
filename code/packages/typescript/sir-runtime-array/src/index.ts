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
 * - **`reduce`/`scan`** (`./reduce`) and **`outer`** (`./outer`) — the SIR22
 *   "APL addendum"'s three `ElementwiseOpKind`-parameterized adverbs (`+/A`,
 *   `+\A`, `A∘.×B`), mirroring `array_runtime::ops::{reduce,scan,outer}`
 *   exactly, including their column-major row/column-fold indexing.
 * - **`shape`/`reshape`** (`./shape`), **`indexGenerator`/`indexOf`**
 *   (`./iota`), and **`ravel`/`catenate`** (`./ravel`) — the SIR22 addendum's
 *   six "bespoke" (non-`BinOp`-shaped) APL primitives (`⍴`, `⍳`, `,`),
 *   mirroring `apl_runtime::builtins::{shape,reshape,index_generator,
 *   index_of,ravel,catenate}` exactly, including `reshape`'s row-major-fill
 *   transposed into column-major storage and `indexGenerator`'s 1-based
 *   (not 0-based) result.
 *
 * ## The SIR22 "APL addendum" is now implemented
 *
 * `Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/`IndexGenerator`/
 * `IndexOf`/`Ravel`/`Catenate` were deferred when this package first shipped
 * (0.1.0) — no frontend crate emitted these nine `Expr` variants yet at the
 * time. That is no longer true: `apl-to-semantic-ir` genuinely lowers APL's
 * `/` (reduce), `\` (scan), `∘.` (outer product), `⍴`, `⍳`, and `,` glyphs
 * to these nine nodes, and `semantic-ir-to-javascript` (the sibling backend,
 * which inlines its own copy of this same logic rather than importing this
 * package) already ported all nine into real codegen — see that crate's
 * "SIR22 APL-addendum codegen" PR. This package now carries the identical
 * port, so a future `semantic-ir-to-typescript` codegen PR wiring
 * `__SirArray.reduce(...)`/`.scan(...)`/`.outer(...)`/`.shape(...)`/
 * `.reshape(...)`/`.indexGenerator(...)`/`.indexOf(...)`/`.ravel(...)`/
 * `.catenate(...)` call sites only has to *wire up calls*, exactly the same
 * "runtime lands before its codegen consumer" pattern this whole package
 * followed for the base cut. That wiring is a separate, not-yet-started
 * follow-up — `semantic-ir-to-typescript` itself still rejects a module
 * using any of these nine nodes via its own
 * `find_unimplemented_sir22_addendum_node` tree-walk, unchanged by this
 * package gaining them.
 *
 * ## Deliberately out of scope
 *
 * `Complex`/`Rational` scalar support (shared `SirType`s with SIR23) is out
 * of scope — `transpose`'s `conjugate` flag is accepted for API-shape
 * parity with the spec but is a no-op today, matching `array-runtime`'s own
 * real-only scope. No operation in this package (including the nine above)
 * is defined beyond rank ≤ 2, matching every Rust reference's own ceiling.
 * A display/auto-print formatter (`apl_runtime::value::display`'s
 * equivalent) is also not included here — unlike `semantic-ir-to-javascript`
 * (which needs one because APL auto-prints a bare top-level expression and
 * has no bracket-indexing syntax to read a value back with instead),
 * `semantic-ir-to-typescript` does not yet consume any APL-sourced module at
 * all, so there is no real consumer to build a display convention for yet —
 * adding one now would be exactly the same "speculative, not filling a real
 * gap" mistake this package originally avoided by deferring the nine
 * addendum nodes themselves.
 */

export { ndarray, checkedShapeSize, scalar, fromVec, fromRows, zeros, ndims, isScalar, nrows, ncols, get, set, MAX_ELEMENTS, type NDArray } from "./ndarray.js";
export { elementwise, type ElementwiseOpKind } from "./elementwise.js";
export { matmul } from "./matmul.js";
export { transpose } from "./transpose.js";
export { range } from "./range.js";
export { indexGet, indexSet, type IndexArg } from "./indexing.js";
export { reduce, scan } from "./reduce.js";
export { outer } from "./outer.js";
export { shape, reshape } from "./shape.js";
export { indexGenerator, indexOf } from "./iota.js";
export { ravel, catenate } from "./ravel.js";
