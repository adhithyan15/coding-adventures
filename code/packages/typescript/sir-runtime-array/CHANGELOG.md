# Changelog

All notable changes to this project will be documented in this file.

## [0.1.0] - 2026-07-14

### Added

- Initial release — the SIR22 N-D array/matrix runtime (HML01 Stream A,
  item 6), meant to be imported by Semantic-IR-emitted TypeScript/
  JavaScript as `__SirArray` for the MATLAB/Octave array domain, once a
  follow-up backend PR wires up the codegen call sites (currently both
  `semantic-ir-to-javascript` and `semantic-ir-to-typescript` hard-reject
  `Feature::NDArrays`/`Feature::MatrixOps` and hit a deferred `panic!` at
  every SIR22 `Expr` match arm — this package builds the runtime primitives
  that follow-up PR will call, mirroring exactly how `sir-runtime-symbolic`
  landed before SIR23's own Stream-B codegen).
- **`NDArray`** (`ndarray`, `scalar`, `fromVec`, `fromRows`, `zeros`,
  `ndims`, `isScalar`, `nrows`, `ncols`, `get`, `set`) — the dense,
  column-major `f64` value model, mirroring `array_runtime::value::Array`
  (`code/packages/rust/array-runtime/src/value.rs`) field-for-field,
  including its exact column-major indexing formula (`(r, c)` lives at flat
  offset `c * nrows + r`) and its "a vector `[n]` is `n×1`" row/column
  convention. Bounded by `MAX_ELEMENTS` (2²⁶, matching `matlab-runtime`'s
  own `MAX_RANGE`) so a compiled program's runtime-computed shape can't
  exhaust memory.
- **`elementwise(op, a, b)`** — all 13 `ElementwiseOpKind`s the SIR22 spec
  defines (`array_runtime::ops::BinOp`'s 12 — `Add`/`Sub`/`Mul`/`Div`/`Max`/
  `Min`/`Eq`/`Ne`/`Lt`/`Le`/`Ge`/`Gt` — plus `Pow`, which is in the SIR spec's
  "original cut" but hasn't been ported to Rust's `array-runtime` crate
  yet). Same scalar-broadcast rule as the Rust reference: either operand
  may be a scalar, otherwise shapes must match exactly. Comparisons produce
  APL-style `1`/`0`, never a native `boolean`.
- **`matmul(a, b)`** / **`transpose(a, conjugate?)`** — mirror
  `array_runtime::ops::matmul`/`transpose` exactly, including their
  column-major indexing arithmetic. `conjugate` (MATLAB `'` vs `.'`) is
  accepted for API-shape parity with the SIR22 spec's `Transpose` field but
  is currently a no-op — there is no `Complex` value type yet, matching
  `array-runtime`'s own real-only scope.
- **`range(start, stop, step?)`** — MATLAB-style `start:step:stop`
  materialization as a `1×n` row vector, with the same inclusive-stop
  tolerance (`1e-9`) and length cap (`MAX_ELEMENTS`) `matlab-runtime`'s own
  `eval_colon` (`code/packages/rust/matlab-runtime/src/eval.rs`) uses.
- **`indexGet(a, indices)`** / **`indexSet(a, indices, value)`** — `A(i)`/
  `A(i, j)` read and in-place write, covering the SIR22 spec's `IndexArg`
  shapes (`scalar`/`whole`/`range`). Scoped to 1 (linear, column-major) or 2
  (row/col) index arguments, matching this whole package's rank-≤-2 scope.
  `indexSet` mutates `a.data` in place — the SIR22 spec makes `IndexSet` a
  *statement*, not a pure expression, for exactly the reason MATLAB
  assignment (`A(i,j) = v`) rebinds one element of the existing array
  rather than producing a new one.
- Deliberately **not** implemented: the SIR22 spec's "APL addendum" `Expr`
  variants (`Reduce`/`Scan`/`OuterProduct`/`Shape`/`Reshape`/
  `IndexGenerator`/`IndexOf`/`Ravel`/`Catenate`) — no frontend crate emits
  these nine variants yet (per that spec's own note), so porting
  `array_runtime::ops`'s existing `reduce`/`scan`/`outer` Rust
  implementations now would be speculative. A natural follow-up once
  `apl-to-semantic-ir`'s own JS-backend consumption needs them.
- 53 tests, 100% line/branch/function coverage, including an adversarial
  regression test confirming `range`'s `MAX_ELEMENTS` cap actually trips
  (not just trusting the code that it would) on a runtime-computed bound
  that would otherwise materialize an unbounded array.

### Fixed

- **Allocate-before-validate ordering in `zeros`/`fromRows`/`matmul`** —
  found by this package's own `/security-review` before its first push.
  `zeros(rows, cols)` and `fromRows` computed `new Float64Array(rows *
  cols)` (or `nrows * ncols`) *before* the shared `ndarray()` constructor
  ever checked `MAX_ELEMENTS` — so an absurd `rows`/`cols` attempted the
  allocation first, either stalling on a huge request or throwing an
  uncaught `RangeError` instead of this package's own clean `Error`.
  `matmul` had the same gap one level up: `m`/`n` come from two
  *independent* operands, each individually under `MAX_ELEMENTS`, but nothing
  bounded their *product* — an outer-product-shaped call (e.g. `[2²⁶, 1] ·
  [1, 2²⁶]`) could still request a `2⁵²`-element output before any check
  ran. Fixed by extracting `checkedShapeSize(shape)` — validates
  negative/non-integer dimensions and the `MAX_ELEMENTS` cap *before*
  returning a safe element count — and calling it in all three places
  before the corresponding `new Float64Array(...)`, not after.
- **Negative/non-integer shape dimensions were not rejected** — `ndarray`
  only checked `n === data.length` and `n <= MAX_ELEMENTS`; a shape like
  `[-2, -50]` against a 100-element buffer computes `n = 100` (two
  negatives multiply positive), passing both checks and producing an
  `NDArray` with negative dimensions that `matmul`/`transpose` would later
  compute a negative allocation size from. `checkedShapeSize` (see above)
  closes this the same way it closes the allocation-ordering gap.
- **`resolvePositions`'s `IndexArg` dispatch had no `default` case** — a
  malformed `kind` (only reachable from a compiled-JS call site that
  crosses the TypeScript/JavaScript boundary this package's exported types
  can't police at runtime) fell through to `undefined` and surfaced as a
  confusing `TypeError` several calls downstream instead of a clean
  `Error` at the point of the actual mistake.
