/**
 * `Reduce`/`Scan` — the SIR22 "APL addendum"'s two adverbs that fold an
 * array with an arbitrary `ElementwiseOpKind` along its one axis. Ported
 * 1:1 from `array_runtime::ops::{reduce,scan}`
 * (`code/packages/rust/array-runtime/src/ops.rs`), which is itself the
 * primary reference this file mirrors — that Rust source (and its own test
 * suite) is the authority for every edge case documented below. This same
 * port was already made once, into `semantic-ir-to-javascript`'s inlined
 * `runtime.rs` string (see that crate's "SIR22 APL-addendum codegen" PR),
 * and is ported again here mechanically, reusing this package's own
 * `applyOp` (`./elementwise.js`) rather than re-deriving a second op-dispatch
 * switch.
 */
import { ndarray, type NDArray } from "./ndarray.js";
import { applyOp, type ElementwiseOpKind } from "./elementwise.js";

/**
 * `+/A` (APL reduce, dyadic-op monadic-adverb) — fold `a` with `op` along
 * its one axis:
 *
 * - rank 0 (scalar): nothing to fold, returns `a` itself.
 * - rank 1 (vector `[n]`): a left fold across all `n` elements
 *   (`op(op(op(v0, v1), v2), …)`). An EMPTY vector is a clean error — unlike
 *   `sum`/`mean` (which have a built-in identity, `0`), `reduce` is generic
 *   over any `op`, and guessing an identity (is it `0` for `Add`, `1` for
 *   `Mul`, `-Infinity` for `Max`?) would be silently wrong for most of them.
 * - rank 2 (matrix `[r, c]`): folds EACH ROW independently across its `c`
 *   columns, producing a `[r]` vector (one folded value per row). Storage
 *   here is column-major (`ndarray.ts`'s own doc comment: element
 *   `(row, col)` lives at flat offset `col * r + row`), so the row loop
 *   below reads `d[row]` as the seed (column 0, at flat offset `0 * r +
 *   row === row`) and then walks `d[col * r + row]` for `col = 1..c`.
 *   **Getting `row` and `col` swapped in that formula silently
 *   TRANSPOSES the result instead of throwing** — this is the single
 *   easiest place to introduce a wrong-answer bug when editing this
 *   function, called out explicitly here (and in the Rust reference's own
 *   doc comment) for exactly that reason.
 */
export function reduce(op: ElementwiseOpKind, a: NDArray): NDArray {
  const { shape } = a;
  if (shape.length === 0) {
    return a;
  }
  if (shape.length === 1) {
    const n = shape[0];
    if (n === 0) {
      throw new Error("reduce: cannot fold an empty vector (no identity element for an arbitrary op)");
    }
    const d = a.data;
    let acc = d[0];
    for (let i = 1; i < n; i++) {
      acc = applyOp(op, acc, d[i]);
    }
    return ndarray([], Float64Array.of(acc));
  }
  if (shape.length === 2) {
    const [r, c] = shape;
    if (c === 0) {
      throw new Error("reduce: cannot fold an empty row (no identity element for an arbitrary op)");
    }
    const d = a.data;
    const out = new Float64Array(r);
    for (let row = 0; row < r; row++) {
      let acc = d[row]; // column-major: (row, 0) lives at plain `row`
      for (let col = 1; col < c; col++) {
        acc = applyOp(op, acc, d[col * r + row]);
      }
      out[row] = acc;
    }
    return ndarray([r], out);
  }
  throw new Error(`reduce: rank > 2 not yet supported (shape ${JSON.stringify(shape)})`);
}

/**
 * `+\A` (APL scan) — the same fold as `reduce`, but keeping EVERY
 * intermediate result instead of only the last; the output has the same
 * shape as `a`. Ported 1:1 from `array_runtime::ops::scan`. An empty axis is
 * NOT an error here (unlike `reduce`): there is simply nothing to scan, and
 * the (empty) output shape already says so — `scan`'s rank-1 branch below
 * degrades to an empty loop and returns a `[0]`-shaped empty vector rather
 * than throwing.
 *
 * The rank-2 branch has the identical column-major indexing trap `reduce`
 * documents above: each row is scanned independently across its columns via
 * `d[col * r + row]`/`out[col * r + row]`, never `d[row * c + col]`.
 */
export function scan(op: ElementwiseOpKind, a: NDArray): NDArray {
  const { shape } = a;
  if (shape.length === 0) {
    return a;
  }
  if (shape.length === 1) {
    const n = shape[0];
    const d = a.data;
    const out = new Float64Array(n);
    let acc = 0;
    let started = false;
    for (let i = 0; i < n; i++) {
      acc = started ? applyOp(op, acc, d[i]) : d[i];
      started = true;
      out[i] = acc;
    }
    return ndarray([n], out);
  }
  if (shape.length === 2) {
    const [r, c] = shape;
    const d = a.data;
    const out = new Float64Array(d.length);
    for (let row = 0; row < r; row++) {
      let acc = 0;
      let started = false;
      for (let col = 0; col < c; col++) {
        const x = d[col * r + row]; // column-major
        acc = started ? applyOp(op, acc, x) : x;
        started = true;
        out[col * r + row] = acc;
      }
    }
    return ndarray([r, c], out);
  }
  throw new Error(`scan: rank > 2 not yet supported (shape ${JSON.stringify(shape)})`);
}
