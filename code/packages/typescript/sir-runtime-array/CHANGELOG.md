# Changelog

All notable changes to this project will be documented in this file.

## [0.2.0] - 2026-07-17

This package went from "built but unconsumed" to actually imported —
`semantic-ir-to-typescript`'s SIR22 codegen (HML01 Stream A, item 7 TS half)
now wires real call sites into it as `__SirArray`. Auditing this package
against the equivalent fixes already made in `semantic-ir-to-javascript`'s
own inlined port of this same logic (that crate's 0.36.0 CHANGELOG entry)
found the identical latent bugs here, never triggered before because
nothing called this package yet.

### Fixed

- **`NaN` silently bypassed the linear (1-argument) `indexGet`/`indexSet`
  bounds check**, causing a silent wrong read and a silently-dropped write.
  `get(a, r, c)`'s 2-argument bounds check is an AND-form
  (`r >= 0 && r < nrows(a)`), which correctly falls through to "out of
  bounds" for `r = NaN` (every relational comparison with `NaN` is `false`,
  so the whole AND is `false`). `resolvePositions`'s linear path instead had
  no check at all before this release. `NDArray` index values come from the
  *compiled program's own runtime arithmetic* (e.g. `0/0`), not just a
  hand-built edge case. Fixed by validating every resolved position is a
  real, finite integer once, in a new `assertValidPosition` helper inside
  `resolvePositions` — the single choke point both `indexGet` and `indexSet`
  route through — rather than re-deriving a NaN-safe bounds check at each
  call site.
- **`range()` silently returned an empty vector instead of erroring on a
  `NaN` `start`/`stop`/`step`.** Same root cause: the loop condition is
  `false` on the very first check whenever a bound is `NaN`, so `range`
  returned a valid-looking `[1, 0]`-shaped empty array with no error. Fixed
  with an explicit `Number.isFinite` check on all three arguments before the
  loop runs.
- **`set(a, r, c, value)`'s bounds check had the same NaN-unsafe OR-form.**
  Not reachable with an unvalidated `NaN` through any current call path
  (every caller resolves positions through `assertValidPosition` first), but
  it is part of this module's exported public surface, so a future direct
  caller — or a refactor of `indexSet` that skips `resolvePositions` — would
  silently reintroduce the bug. Fixed by writing the check as the negation
  of `get`'s AND-form (`!(r >= 0 && ...)`) rather than an OR-form, matching
  how `get` was already written.
- **`elementwise(op, a, b)` assumed both operands were already `NDArray`.**
  `matlab-to-semantic-ir`'s lowering emits a *bare* (unwrapped) scalar
  operand for `.* ./ .\` and for `* /` when exactly one side is provably
  scalar (e.g. `A .* 2` — the `2` arrives as a plain number literal, not an
  `ArrayLit`/scalar-array constructor). Fixed with a new `toArrayValue`
  coercion, applied to both operands before either is read as `.data`/
  `.shape`; `elementwise`'s signature widens to `(op, a: number | NDArray, b:
  number | NDArray)` accordingly.

Ten new regression tests (NaN scalar `indexGet`/`indexSet`, a non-integer
scalar index, `set`'s direct NaN case, three `range` NaN-bound cases, and
three bare-number `elementwise` operand cases), each confirmed to fail
without its fix and pass with it. `npm run build` (`tsc`) and `npm test`
(`vitest`) both clean.

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
- 57 tests, 100% line/branch/function coverage, including an adversarial
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
- **The same allocate-before-validate gap in `indexGet`/`indexSet`'s
  2-index sub-array path** — found on the *second* `/security-review`
  round, after the first three fixes above. `rows.length`/`cols.length`
  (from `resolvePositions` on each index argument) are each individually
  bounded — by `a`'s own dimensions for a `whole` selection, or by a
  `range` NDArray's own `MAX_ELEMENTS` cap for a `range` selection — but
  nothing bounded their *product*, so a `range`-selected row list and a
  `range`-selected column list, each independently legitimate (up to
  `MAX_ELEMENTS` positions), could still multiply to an absurd output size
  before `indexGet`'s `new Float64Array(rows.length * cols.length)` or
  `indexSet`'s `broadcastValues(value, rows.length * cols.length)` ever
  allocated it — the exact outer-product-shaped gap the `matmul` fix
  above closed, reopened one function over. Fixed the same way:
  `checkedShapeSize([rows.length, cols.length])` before either allocation.
- **`fromVec` and `applyOp` were missing the same two guard classes,
  found by a third, systematic sweep of every `Float64Array` call site
  in the package** (checking each one's size argument back to its
  source, not just spot-checking what earlier rounds had already
  touched):
  - `fromVec(values)` called `Float64Array.from(values)` before
    validating `values.length` — `Float64Array.from` accepts any bare
    `{ length: N }` array-like, not just a real `number[]`, so a caller
    could request an `N`-sized allocation while paying for none of the
    `N` elements themselves (the same allocate-before-validate class
    fixed three times above, one level lower). Now calls
    `checkedShapeSize([values.length])` first.
  - `applyOp` (in `elementwise.ts`) had no `default` case in its
    `ElementwiseOpKind` switch — an unrecognised `op` fell through to an
    implicit `undefined`, which array assignment silently coerces to
    `NaN`, corrupting data instead of failing loudly (the same missing-
    `default` class `resolvePositions` was fixed for above). Now throws
    a clean `Error` for an unrecognised `op`.
- **`ndarray()` never checked that `data` was really a `Float64Array`** —
  found by a fourth, final confirmation sweep. `NDArray` is a plain
  structural interface, not a class, and every other function in this
  package sizes its own allocations from an existing `NDArray`'s
  `data.length`, trusting it was already validated by `ndarray()` — an
  unenforced invariant a compiled-JS caller could violate by handing back
  an `NDArray`-shaped object whose `data` is a plain array or other
  array-like instead of a real `Float64Array`. `ndarray()` now asserts
  `data instanceof Float64Array` before anything else. 57 tests (up from
  56), 100% coverage maintained — this closed the review; four rounds of
  `/security-review`, converging from 5 findings down to 1 defense-in-depth
  item with no demonstrated live exploit path.
