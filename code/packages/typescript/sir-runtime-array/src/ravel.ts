/**
 * `Ravel`/`Catenate` — the SIR22 addendum's monadic and dyadic `,`
 * (APL ravel / catenate). Ported 1:1 from `apl_runtime::builtins::
 * {ravel,catenate}` (`code/packages/rust/apl-runtime/src/builtins.rs`),
 * previously ported once already into `semantic-ir-to-javascript`'s inlined
 * runtime (see that crate's "SIR22 APL-addendum codegen" PR) — this is the
 * same port made a third time, mechanically, into this package's own
 * module-per-concern layout.
 */
import { ndarray, checkedShapeSize, nrows, ncols, type NDArray } from "./ndarray.js";

/**
 * Flatten (rank ≤ 2, this package's ceiling) `a` to ROW-major order — the
 * *last* axis varies fastest. `a` itself stores data COLUMN-major
 * (`ndarray.ts`'s own doc comment: element `(row, col)` lives at flat offset
 * `col * nrows + row`), so a matrix must be walked "row, then column" to
 * produce true row-major order — simply returning the raw column-major
 * buffer (`a.data`) would silently ravel in the WRONG order.
 *
 * Reads `a.data` directly with that same column-major formula, rather than
 * through `get` (whose `number | undefined` return would force an
 * always-false bounds check on every element here, since `row`/`col` are
 * derived from `a`'s own shape and so are always in range) — mirrors how
 * `matmul.ts`/`transpose.ts` already index `.data` directly for the same
 * reason.
 *
 * Always returns a FRESH `Float64Array` (never `a.data` itself, even in the
 * rank ≤ 1 no-op case) — mirrors `apl_runtime::builtins::flatten` returning
 * an owned `Vec`, not a borrow, so a caller mutating the ravelled result
 * never accidentally mutates `a`'s own buffer.
 *
 * Exported (not file-private) so `./shape.js`'s `reshape` can reuse this
 * exact row-major walk rather than duplicating it a second time.
 */
export function flattenRowMajor(a: NDArray): Float64Array {
  const { shape } = a;
  if (shape.length <= 1) {
    return Float64Array.from(a.data);
  }
  if (shape.length === 2) {
    const r = nrows(a);
    const c = ncols(a);
    const out = new Float64Array(r * c);
    let k = 0;
    for (let row = 0; row < r; row++) {
      for (let col = 0; col < c; col++) {
        out[k++] = a.data[col * r + row]; // column-major storage, row-major output order
      }
    }
    return out;
  }
  // Unreachable in practice (this package's rank ≤ 2 ceiling) — total
  // rather than throwing, mirroring the Rust reference's own fallback
  // (`_ => a.data().to_vec()`).
  return Float64Array.from(a.data);
}

/**
 * Monadic `,` (ravel) — flatten `a` to a rank-1 vector, in row-major order
 * (see `flattenRowMajor`'s own doc comment for the column-major-storage
 * vs. row-major-order subtlety). Ported 1:1 from
 * `apl_runtime::builtins::ravel`.
 */
export function ravel(a: NDArray): NDArray {
  const flat = flattenRowMajor(a);
  return ndarray([flat.length], flat);
}

/**
 * Dyadic `,` (catenate) — supports scalar-scalar, scalar-vector,
 * vector-scalar, vector-vector (all producing a vector), and
 * matrix-matrix-with-equal-row-counts (column/last-axis catenate, producing
 * `[r, ca + cb]`). Any other rank combination is a clean "not yet
 * supported" error. Ported 1:1 from `apl_runtime::builtins::catenate`.
 *
 * SECURITY: the combined-length cap check happens ONCE, up front, before
 * any rank-dispatch below — mirroring the Rust reference's own structure —
 * because neither operand alone need be oversized for the RESULT to be: a
 * script that repeatedly catenates a value with itself (`A←A,A`) doubles
 * the size every call with no other ceiling, and each individual `catenate`
 * call only ever sees its own two (individually already-valid) operands, so
 * the check must re-run on *every* call rather than being satisfied once
 * and cached.
 */
export function catenate(a: NDArray, b: NDArray): NDArray {
  checkedShapeSize([a.data.length + b.data.length]);
  const ra = a.shape.length;
  const rb = b.shape.length;
  if (ra === 0 && rb === 0) {
    return ndarray([2], Float64Array.of(a.data[0], b.data[0]));
  }
  if (ra === 0 && rb === 1) {
    const out = new Float64Array(1 + b.data.length);
    out[0] = a.data[0];
    out.set(b.data, 1);
    return ndarray([out.length], out);
  }
  if (ra === 1 && rb === 0) {
    const out = new Float64Array(a.data.length + 1);
    out.set(a.data, 0);
    out[a.data.length] = b.data[0];
    return ndarray([out.length], out);
  }
  if (ra === 1 && rb === 1) {
    const out = new Float64Array(a.data.length + b.data.length);
    out.set(a.data, 0);
    out.set(b.data, a.data.length);
    return ndarray([out.length], out);
  }
  if (ra === 2 && rb === 2) {
    const r = nrows(a);
    if (r !== nrows(b)) {
      throw new Error(`catenate: matrix catenate needs equal row counts (${r} vs ${nrows(b)})`);
    }
    const ca = ncols(a);
    const cb = ncols(b);
    const outLen = checkedShapeSize([r, ca + cb]);
    const data = new Float64Array(outLen);
    for (let row = 0; row < r; row++) {
      for (let col = 0; col < ca; col++) {
        data[col * r + row] = a.data[col * r + row]; // a has the same r rows
      }
      for (let col = 0; col < cb; col++) {
        data[(ca + col) * r + row] = b.data[col * r + row]; // b has the same r rows
      }
    }
    return ndarray([r, ca + cb], data);
  }
  throw new Error(`catenate: catenate of rank ${ra} and rank ${rb} is not yet supported`);
}
