import { ndarray, nrows, ncols, type NDArray } from "./ndarray.js";

/**
 * Matrix transpose — mirrors `array_runtime::ops::transpose`. `conjugate`
 * distinguishes MATLAB `'` (`true`) from `.'` (`false`), per the SIR22
 * spec's `Transpose { conjugate }` field. This runtime has no `Complex`
 * value type yet (matching `array-runtime`'s own real-only scope today), so
 * a conjugate transpose of real data is identical to a plain transpose —
 * `conjugate` is accepted for API-shape parity with the SIR spec and
 * documented here so a future `Complex` extension knows exactly where the
 * actual conjugation step belongs.
 */
export function transpose(a: NDArray, conjugate = false): NDArray {
  void conjugate;
  const m = nrows(a);
  const n = ncols(a);
  const ad = a.data;
  const out = new Float64Array(ad.length);
  for (let j = 0; j < n; j++) {
    for (let i = 0; i < m; i++) {
      out[i * n + j] = ad[j * m + i];
    }
  }
  return ndarray([n, m], out);
}
