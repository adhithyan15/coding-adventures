/**
 * `IndexGenerator`/`IndexOf` — the SIR22 addendum's monadic and dyadic `⍳`
 * (APL index generator / index-of). Named `iota.ts`, not `range.ts` (already
 * taken by this package's MATLAB-style `start:step:stop` materialization),
 * to avoid colliding with that unrelated concern despite both being
 * "produce a sequence of numbers" functions.
 *
 * Ported 1:1 from `apl_runtime::builtins::{index_generator,index_of}`
 * (`code/packages/rust/apl-runtime/src/builtins.rs`), previously ported
 * once already into `semantic-ir-to-javascript`'s inlined runtime (see that
 * crate's "SIR22 APL-addendum codegen" PR) — this is the same port made a
 * third time, mechanically, into this package's own module-per-concern
 * layout.
 */
import { ndarray, checkedShapeSize, isScalar, type NDArray } from "./ndarray.js";

/**
 * Monadic `⍳` (index generator / iota) — `⍳n` is the **1-based** vector
 * `[1, 2, …, n]`. Ported 1:1 from `apl_runtime::builtins::index_generator`.
 *
 * NOTE the 1-basedness: unlike every other index in this package
 * (`indexGet`/`indexSet` in `./indexing.js` are 0-based, matching this
 * package's own JS/TS-native convention), `⍳` is genuinely 1-based at the
 * SURFACE-SYNTAX level in real APL — `⍳5` is `1 2 3 4 5`, never `0 1 2 3 4`
 * — so this is a real, deliberate inconsistency with the rest of this
 * package's indexing, not an oversight. (The SIR22 spec's own
 * `Expr::IndexGenerator` doc-comment prose describes a 0-based
 * `0, 1, …, N-1` result, but that text predates `apl-runtime`'s actual
 * implementation and the already-merged `semantic-ir-to-javascript` port
 * both of which are genuinely 1-based — this function follows those two
 * real references, not the spec's stale prose.)
 *
 * SECURITY: `checkedShapeSize([n])` both validates `n` is a non-negative
 * integer AND caps it at `MAX_ELEMENTS` before allocating — `n` is a
 * runtime value a compiled program computes, not a fixed constant, so `⍳`
 * of an absurd size must fail cleanly.
 */
export function indexGenerator(a: NDArray): NDArray {
  if (!isScalar(a)) {
    throw new Error("indexGenerator: monadic argument must be a scalar");
  }
  const x = a.data[0];
  if (!(Number.isInteger(x) && x >= 0)) {
    throw new Error(`indexGenerator: monadic argument must be a non-negative integer, got ${x}`);
  }
  const n = checkedShapeSize([x]);
  const out = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    out[i] = i + 1;
  }
  return ndarray([n], out);
}

/**
 * Dyadic `⍳` (index-of / search) — for every element of `needle`, the
 * 1-based index of its first occurrence in the vector `haystack` (or
 * `haystack.length + 1` if not found — "not found" is a valid, always-in-
 * range position, not `-1`/`undefined`). Ported 1:1 from
 * `apl_runtime::builtins::index_of`: plain EXACT equality (no
 * floating-point tolerance — `Float64Array.prototype.indexOf` already uses
 * strict `===`, so `NaN` correctly never matches, same as Rust's `==`). The
 * result has `needle`'s shape.
 *
 * SECURITY: the work done is O(len(haystack) × len(needle)) — a full linear
 * scan of `haystack` per element of `needle` — so each operand
 * individually staying under `MAX_ELEMENTS` does NOT bound their *product*
 * (up to ~4.5 × 10¹⁵ comparisons otherwise, a compute-bound DoS even though
 * no single allocation is oversized). `checkedShapeSize` is reused here
 * purely for its "product ≤ MAX_ELEMENTS" check (both lengths are already
 * valid non-negative integers by construction, so its dimension-validity
 * half is a no-op) — it runs *before* any scanning, not after, exactly like
 * every other bounded-allocation check in this package.
 */
export function indexOf(haystack: NDArray, needle: NDArray): NDArray {
  if (haystack.shape.length > 1) {
    throw new Error(`indexOf: left argument must be a scalar or vector (got rank ${haystack.shape.length})`);
  }
  checkedShapeSize([haystack.data.length, needle.data.length]);
  const hay = haystack.data;
  const out = Float64Array.from(needle.data, (n) => {
    const idx = hay.indexOf(n);
    return idx === -1 ? hay.length + 1 : idx + 1;
  });
  return ndarray(needle.shape, out);
}
