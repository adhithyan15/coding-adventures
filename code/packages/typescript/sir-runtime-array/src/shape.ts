/**
 * `Shape`/`Reshape` — the SIR22 addendum's monadic and dyadic `⍴`
 * (APL shape-of / reshape). Ported 1:1 from `apl_runtime::builtins::
 * {shape,reshape}` (`code/packages/rust/apl-runtime/src/builtins.rs`),
 * previously ported once already into `semantic-ir-to-javascript`'s inlined
 * runtime (see that crate's "SIR22 APL-addendum codegen" PR) — this is the
 * same port made a third time, mechanically, into this package's own
 * module-per-concern layout.
 */
import { ndarray, checkedShapeSize, type NDArray } from "./ndarray.js";
import { flattenRowMajor } from "./ravel.js";

/**
 * Monadic `⍴` (shape-of) — `a`'s dimensions as a vector. Ported 1:1 from
 * `apl_runtime::builtins::shape`: a SCALAR has zero dimensions, so its shape
 * is the EMPTY vector (not a scalar!) — `⍴5` is `⍳0`-shaped, a length-0
 * vector, mirroring `shape.length === 0` exactly. A vector `[n]` has shape
 * `[n]` (one element); a matrix `[r, c]` has shape `[r, c]` (two elements).
 */
export function shape(a: NDArray): NDArray {
  const dims = Float64Array.from(a.shape);
  return ndarray([dims.length], dims);
}

/**
 * Dyadic `⍴` (reshape) — reinterpret `target`'s data under the new
 * dimensions `shapeArg`. Ported 1:1 from `apl_runtime::builtins::reshape`.
 * `shapeArg` must itself be a scalar or vector (rank ≤ 1) of non-negative
 * integers, and the target shape it describes is itself capped at rank ≤ 2
 * (this package's ceiling — a longer target shape is a clean error, not a
 * silent truncation). `target`'s elements are ravelled (`flattenRowMajor`,
 * `./ravel.js`) then cyclically repeated or truncated to fill the target
 * shape's element count.
 *
 * **CRITICAL correctness trap** (the single easiest place in this whole
 * addendum to introduce a silent wrong-answer bug): the cyclic fill happens
 * in ROW-major order (APL's reshape fills the *last* axis fastest, the same
 * convention `ravel` uses), but this package's storage is COLUMN-major
 * (`ndarray.ts`'s own doc comment) — so for a rank-2 target, the row-major
 * `filled` sequence must be TRANSPOSED into column-major storage
 * (`data[col * r + row] = filled[row * c + col]`) before calling `ndarray`.
 * Handing `filled` straight to `ndarray` would silently reshape
 * column-major instead of APL's row-major convention — a wrong answer that
 * still LOOKS plausible (the right multiset of values, in the wrong
 * positions), which is exactly why this needs calling out rather than
 * trusting a future refactor to rediscover it.
 */
export function reshape(shapeArg: NDArray, target: NDArray): NDArray {
  if (shapeArg.shape.length > 1) {
    throw new Error(`reshape: shape argument must be a scalar or vector (got rank ${shapeArg.shape.length})`);
  }
  const dims = Array.from(shapeArg.data, (x) => {
    if (!(Number.isInteger(x) && x >= 0)) {
      throw new Error(`reshape: shape elements must be non-negative integers, got ${x}`);
    }
    return x;
  });
  if (dims.length > 2) {
    throw new Error(`reshape: reshape to rank > 2 is not yet supported (target shape ${JSON.stringify(dims)})`);
  }
  const total = checkedShapeSize(dims);
  const source = flattenRowMajor(target);
  if (total > 0 && source.length === 0) {
    throw new Error("reshape: cannot reshape an empty source into a non-empty shape");
  }
  const filled = new Float64Array(total);
  for (let k = 0; k < total; k++) {
    filled[k] = source[k % source.length];
  }
  // Rank ≤ 1: row-major and column-major coincide, so `filled` is already
  // in the right order for `ndarray` (which expects column-major data).
  if (dims.length <= 1) {
    return ndarray(dims, filled);
  }
  // Rank 2: transpose the row-major fill into column-major storage — see
  // this function's own doc comment above for why this direction, and only
  // this direction, is correct.
  const [r, c] = dims;
  const data = new Float64Array(total);
  for (let row = 0; row < r; row++) {
    for (let col = 0; col < c; col++) {
      data[col * r + row] = filled[row * c + col];
    }
  }
  return ndarray(dims, data);
}
