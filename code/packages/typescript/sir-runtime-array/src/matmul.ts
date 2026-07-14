import { ndarray, checkedShapeSize, nrows, ncols, type NDArray } from "./ndarray.js";

/**
 * Matrix product `[m, k] · [k, n] → [m, n]` (column-major throughout) —
 * mirrors `array_runtime::ops::matmul` exactly, including its indexing
 * arithmetic. `m` and `n` come from two *independent* operands (each
 * individually under `MAX_ELEMENTS`, but their product isn't bounded by
 * that alone — an outer-product-shaped call, e.g. `[2²⁶, 1] · [1, 2²⁶]`,
 * could still ask for a `2⁵²`-element output), so `checkedShapeSize`
 * validates `[m, n]` *before* allocating `out`, not after — the same
 * allocate-after-validate ordering `zeros`/`fromRows` use, and the same
 * class of guard the Rust reference closes with a checked multiply.
 */
export function matmul(a: NDArray, b: NDArray): NDArray {
  const m = nrows(a);
  const ka = ncols(a);
  const kb = nrows(b);
  const n = ncols(b);
  if (ka !== kb) {
    throw new Error(`matmul: inner dimensions disagree (${m}x${ka} · ${kb}x${n})`);
  }
  const outLen = checkedShapeSize([m, n]);
  const ad = a.data;
  const bd = b.data;
  const out = new Float64Array(outLen);
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < m; i++) {
      let acc = 0;
      for (let p = 0; p < ka; p++) {
        acc += ad[p * m + i] * bd[j * kb + p]; // column-major indexing
      }
      out[j * m + i] = acc;
    }
  }
  return ndarray([m, n], out);
}
