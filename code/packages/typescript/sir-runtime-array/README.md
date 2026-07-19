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
| Reduce / scan | `reduce(op, a)` / `scan(op, a)` | `array_runtime::ops::{reduce,scan}` |
| Outer product | `outer(op, a, b)` | `array_runtime::ops::outer` |
| Shape / reshape | `shape(a)` / `reshape(shapeArg, target)` | `apl_runtime::builtins::{shape,reshape}` |
| Index generator / index-of | `indexGenerator(a)` / `indexOf(haystack, needle)` | `apl_runtime::builtins::{index_generator,index_of}` |
| Ravel / catenate | `ravel(a)` / `catenate(a, b)` | `apl_runtime::builtins::{ravel,catenate}` |

## Backend consumers

Both `semantic-ir-to-javascript` and `semantic-ir-to-typescript` now accept
`Feature::NDArrays`/`Feature::MatrixOps`/`Feature::ArrayColumnMajor` and emit
real codegen for the SIR22 base cut. `semantic-ir-to-typescript` imports this
package directly (`import * as __SirArray from "@coding-adventures/sir-runtime-array"`);
`semantic-ir-to-javascript` inlines its own plain-JS port of the same logic
instead (that backend always inlines runtime helpers rather than importing
packages) — see each crate's own `emit.rs`/`runtime.rs` and CHANGELOG for
the wiring. This package landed first (mirroring exactly how
`sir-runtime-symbolic`, SIR23's runtime, preceded its own Stream-B codegen)
so those backend PRs only had to wire up calls, not design an array
representation from scratch.

The SIR22 "APL addendum" (`reduce`/`scan`/`outer`/`shape`/`reshape`/
`indexGenerator`/`indexOf`/`ravel`/`catenate`, below) followed the same
pattern one release later: `semantic-ir-to-javascript` shipped real codegen
for all nine first (its "SIR22 APL-addendum codegen" PR), since
`apl-to-semantic-ir` genuinely emits these nine nodes for APL's `/`/`\`/
`∘.`/`⍴`/`⍳`/`,` glyphs. This package now carries the identical port, but
`semantic-ir-to-typescript` has **not** wired codegen for them yet — that
backend still rejects a module using any of these nine nodes via its own
`find_unimplemented_sir22_addendum_node` tree-walk, unchanged by this
package gaining them. Wiring `__SirArray.reduce(...)`/etc. call sites into
`semantic-ir-to-typescript`'s `emit.rs` is a separate, not-yet-started
follow-up.

## How emitted code uses it

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
- **`reduce`/`scan`** (`reduce.ts`) and **`outer`** (`outer.ts`) — the SIR22
  "APL addendum"'s three `ElementwiseOpKind`-parameterized adverbs (`+/A`,
  `+\A`, `A∘.×B`), mirroring `array_runtime::ops::{reduce,scan,outer}`
  exactly, including their column-major row/column-fold indexing (getting
  that indexing backwards silently transposes the result instead of
  throwing — see `reduce.ts`'s own doc comment).
- **`shape`/`reshape`** (`shape.ts`), **`indexGenerator`/`indexOf`**
  (`iota.ts`), and **`ravel`/`catenate`** (`ravel.ts`) — the SIR22
  addendum's six "bespoke" (non-`BinOp`-shaped) APL primitives (`⍴`, `⍳`,
  `,`), mirroring `apl_runtime::builtins::{shape,reshape,index_generator,
  index_of,ravel,catenate}` exactly. `reshape`'s row-major cyclic fill must
  be transposed into this package's column-major storage for a rank-2
  target (`shape.ts`'s own doc comment covers this in detail — it is the
  single easiest place in this whole package to introduce a silent
  wrong-answer bug); `indexGenerator` is 1-based (`⍳n` is `[1, …, n]`),
  unlike every other index in this package.

## Deliberately out of scope

`Complex`/`Rational` scalar support (shared `SirType`s with SIR23) is out of
scope — `transpose`'s `conjugate` flag is accepted for API-shape parity with
the spec but is a no-op today, matching `array-runtime`'s own real-only
scope. No operation in this package (including the nine APL-addendum
functions above) is defined beyond rank ≤ 2, matching every Rust reference's
own ceiling.

This package also does **not** include a display/auto-print formatting
helper (the equivalent of `apl_runtime::value::display`/
`semantic-ir-to-javascript`'s inlined `ArrayRt.display`). That JS backend
needed one because APL auto-prints a bare top-level expression and has no
bracket-indexing syntax to read a value back with instead — there was
nowhere else to route a computed array's display through.
`semantic-ir-to-typescript` does not have that problem today because it does
not yet consume any APL-sourced module at all (see "Backend consumers"
above) — building a display convention now, with no real consumer, would be
exactly the same "speculative, not filling a real gap" mistake this package
avoided by deferring the APL addendum itself for two releases.

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

The APL-addendum functions reuse this exact same `MAX_ELEMENTS` cap for two
allocation-adjacent gaps that are easy to miss because no single operand is
oversized: `indexOf`'s work is O(len(haystack) × len(needle)) — each length
can individually sit well under `MAX_ELEMENTS` while their *product* is
still absurd — and `catenate`'s combined output length is the *sum* of two
operands that can each individually be valid on their own (a script that
repeatedly does `A = catenate(A, A)` doubles the size every call with no
other ceiling). Both check the product/sum *before* any allocation or
scanning, on every call, exactly like every other bounded-allocation check
in this package.

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
