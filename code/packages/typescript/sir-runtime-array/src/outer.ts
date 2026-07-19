/**
 * `OuterProduct` — the SIR22 addendum's `A∘.×B` (APL outer product). Ported
 * 1:1 from `array_runtime::ops::outer`
 * (`code/packages/rust/array-runtime/src/ops.rs`), and previously ported
 * once already into `semantic-ir-to-javascript`'s inlined runtime (see that
 * crate's "SIR22 APL-addendum codegen" PR) — this is the same port made a
 * third time, mechanically, into this package's own module-per-concern
 * layout.
 */
import { ndarray, checkedShapeSize, type NDArray } from "./ndarray.js";
import { applyOp, type ElementwiseOpKind } from "./elementwise.js";

/**
 * `A∘.×B` — apply `op` to every pair `(aᵢ, bⱼ)`, producing a result of rank
 * `rank(a) + rank(b)`:
 *
 * - scalar ⊗ scalar → scalar (`op(a, b)`).
 * - scalar ⊗ vector `[n]` (or vector ⊗ scalar) → vector `[n]` (the scalar
 *   broadcasts, exactly like `elementwise`'s own scalar case).
 * - vector `[m]` ⊗ vector `[n]` → matrix `[m, n]`, element `(i, j) =
 *   op(a[i], b[j])`.
 *
 * Scoped to `rank(a) ≤ 1` and `rank(b) ≤ 1` — the vector⊗vector case
 * already reaches this package's rank-2 ceiling, matching
 * `array_runtime::ops::outer`'s identical scope — a higher-rank operand is a
 * clean "not yet supported" error rather than silently-wrong output.
 *
 * SECURITY: `m`/`n` are two INDEPENDENT operand lengths, each individually
 * already validated (≤ `MAX_ELEMENTS`, since each came from an existing,
 * already-constructed `NDArray`), but nothing bounds their *product* on its
 * own — the exact outer-product-shaped allocation gap `matmul`/`indexGet`
 * elsewhere in this package guard against. `checkedShapeSize([m, n])`
 * validates the `[m, n]` output shape *before* allocating `out`, not after.
 */
export function outer(op: ElementwiseOpKind, a: NDArray, b: NDArray): NDArray {
  const as = a.shape;
  const bs = b.shape;
  if (as.length === 0 && bs.length === 0) {
    return ndarray([], Float64Array.of(applyOp(op, a.data[0], b.data[0])));
  }
  if (as.length === 0 && bs.length === 1) {
    const x = a.data[0];
    return ndarray([bs[0]], Float64Array.from(b.data, (y) => applyOp(op, x, y)));
  }
  if (as.length === 1 && bs.length === 0) {
    const y = b.data[0];
    return ndarray([as[0]], Float64Array.from(a.data, (x) => applyOp(op, x, y)));
  }
  if (as.length === 1 && bs.length === 1) {
    const m = as[0];
    const n = bs[0];
    const outLen = checkedShapeSize([m, n]);
    const ad = a.data;
    const bd = b.data;
    const out = new Float64Array(outLen);
    for (let j = 0; j < n; j++) {
      for (let i = 0; i < m; i++) {
        out[j * m + i] = applyOp(op, ad[i], bd[j]); // column-major
      }
    }
    return ndarray([m, n], out);
  }
  throw new Error(`outer: operands of rank > 1 not yet supported (shapes ${JSON.stringify(as)}, ${JSON.stringify(bs)})`);
}
