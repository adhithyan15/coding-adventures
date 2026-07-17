import { ndarray, MAX_ELEMENTS, type NDArray } from "./ndarray.js";

/**
 * Tolerance for the inclusive-stop boundary check, matching
 * `matlab-runtime`'s own `eval_colon` exactly (`code/packages/rust/matlab-runtime/src/eval.rs`)
 * — a floating step (e.g. `1:0.1:2`) can drift a few ULPs short of `stop` by
 * the final iteration, and MATLAB's `a:step:b` is inclusive of `b`.
 */
const RANGE_EPSILON = 1e-9;

/**
 * Materialize a MATLAB-style range `start:step:stop` (default `step = 1`,
 * per the SIR22 spec's `Range { step: Option<...> }` field) as a `1×n` row
 * vector — MATLAB's `:` always produces a row, never a column, which is why
 * this returns shape `[1, n]` rather than the "bare vector" shape `[n]`
 * `fromVec` uses.
 *
 * Bounded by `MAX_ELEMENTS` (shared with `ndarray`'s own cap, and matching
 * `matlab-runtime`'s `MAX_RANGE`) so a compiled program's `1:1e18`-style
 * range can't exhaust memory before this function ever gets to materialize
 * anything.
 */
export function range(start: number, stop: number, step = 1): NDArray {
  if (step === 0) {
    throw new Error("range: step cannot be zero");
  }
  // SECURITY: the loop condition below is false on its very first check
  // whenever start/stop/step is NaN (every relational comparison with
  // NaN is false), so an unguarded NaN bound would silently produce an
  // empty range instead of erroring — the same "NaN defeats a
  // comparison-based check" class `indexGet`/`indexSet`'s fix closes.
  // Reject non-finite bounds up front instead of letting them fall
  // through to a quietly-wrong empty result.
  if (!Number.isFinite(start) || !Number.isFinite(stop) || !Number.isFinite(step)) {
    throw new Error(`range: start/stop/step must be finite numbers, got (${start}, ${stop}, ${step})`);
  }
  const values: number[] = [];
  let x = start;
  while ((step > 0 && x <= stop + RANGE_EPSILON) || (step < 0 && x >= stop - RANGE_EPSILON)) {
    if (values.length >= MAX_ELEMENTS) {
      throw new Error(`range: produces more than ${MAX_ELEMENTS} elements`);
    }
    values.push(x);
    x += step;
  }
  return ndarray(
    values.length === 0 ? [1, 0] : [1, values.length],
    Float64Array.from(values),
  );
}
