/**
 * The N-D array value model — dense, rectangular, **column-major** `f64`
 * (stored here as a `Float64Array`), mirroring `array_runtime::value::Array`
 * (`code/packages/rust/array-runtime/src/value.rs`) exactly. Column-major
 * (Fortran/MATLAB) storage is deliberate, not incidental: MATLAB's `reshape`,
 * linear indexing, and `[a; b]` literal semantics all assume it, and the
 * SIR22 spec's `Feature::ArrayColumnMajor` manifest flag exists precisely so
 * a JS/TS backend states this convention explicitly (see that spec's
 * "Storage convention" section) rather than leaving it implicit the way the
 * Rust struct's memory layout does.
 *
 * `shape == []` is a scalar, `[n]` a vector (treated as `n×1`, a column, for
 * row/column purposes — matching the Rust convention exactly), `[r, c]` a
 * matrix. Higher ranks are representable but no operation in this package
 * defines them yet (same "rank ≤ 2 today" scope `array-runtime` itself
 * documents).
 */

/** A dense N-D `f64` array — column-major storage, shape-validated. */
export interface NDArray {
  readonly shape: readonly number[];
  readonly data: Float64Array;
}

/**
 * Upper bound on total elements for any array this package constructs
 * (`ndarray`, `range`, `matmul`'s output, …) — a compiled program's array
 * construction is driven by potentially attacker-influenced input (sizes
 * computed at runtime, not fixed at compile time), so an unbounded shape or
 * range must fail cleanly rather than exhaust memory. Matches
 * `matlab-runtime`'s own `MAX_RANGE` bound
 * (`code/packages/rust/matlab-runtime/src/eval.rs`) for consistency across
 * the MATLAB-family stack.
 */
export const MAX_ELEMENTS = 1 << 26; // 67,108,864

/**
 * Validate a shape *before* any caller allocates a buffer sized from it —
 * every function that computes an output size from caller-supplied numbers
 * (`zeros`, `fromRows`, `matmul`'s `m * n`, …) must call this first, not
 * after, so a negative, non-integer, or absurdly large shape is rejected
 * with a clean `Error` before `new Float64Array(...)` ever runs. Calling
 * the cap check *after* allocating (as `ndarray`'s own validation alone
 * would, if factories didn't also call this) is too late: the allocation
 * attempt itself can throw an uncaught `RangeError` or stall on a huge
 * request before the cap gets a chance to reject anything cleanly.
 * Returns the validated element count.
 */
export function checkedShapeSize(shape: readonly number[]): number {
  if (!shape.every((d) => Number.isInteger(d) && d >= 0)) {
    throw new Error(`checkedShapeSize: shape ${JSON.stringify(shape)} has a negative or non-integer dimension`);
  }
  const n = shape.reduce((acc, d) => acc * d, 1);
  if (!Number.isFinite(n) || n > MAX_ELEMENTS) {
    throw new Error(`checkedShapeSize: shape ${JSON.stringify(shape)} (${n} elements) exceeds the ${MAX_ELEMENTS}-element cap`);
  }
  return n;
}

/**
 * Build an `NDArray` from an explicit column-major `data` buffer and
 * `shape` — the shared validating constructor every factory below funnels
 * through (mirrors `Array::from_shape`). Rejects a shape/data-length
 * mismatch and a shape whose element count exceeds `MAX_ELEMENTS`.
 *
 * `NDArray` is a plain structural interface, not a class, so nothing stops
 * a compiled-JS caller from handing back an object shaped like one whose
 * `data` isn't really a `Float64Array` — every other function in this
 * package sizes its own allocations from an existing `NDArray`'s
 * `data.length`, trusting it was already validated here, so this check is
 * where that trust actually has to be earned.
 */
export function ndarray(shape: readonly number[], data: Float64Array): NDArray {
  if (!(data instanceof Float64Array)) {
    throw new Error("ndarray: data must be a Float64Array");
  }
  const n = checkedShapeSize(shape);
  if (n !== data.length) {
    throw new Error(
      `ndarray: shape ${JSON.stringify(shape)} implies ${n} elements, got ${data.length}`,
    );
  }
  return { shape, data };
}

/** A length-1 (scalar) array. */
export function scalar(value: number): NDArray {
  return ndarray([], Float64Array.of(value));
}

/**
 * A 1-D array (a column vector's worth of values, shape `[n]`). Validates
 * `values.length` with `checkedShapeSize` *before* `Float64Array.from`
 * allocates — a genuine TypeScript-typed `number[]` costs its caller
 * memory proportional to its own length already, but `values` crosses the
 * same unenforced JS-runtime boundary every other factory in this module
 * does, and `Float64Array.from` also accepts a bare `{ length: N }`
 * array-like with no real backing elements, which would otherwise drive an
 * `N`-sized allocation from a caller who paid for none of it.
 */
export function fromVec(values: readonly number[]): NDArray {
  checkedShapeSize([values.length]);
  return ndarray([values.length], Float64Array.from(values));
}

/**
 * Build a matrix from rows (mirrors `Array::from_rows`). All rows must be
 * the same length; the data is transposed into column-major order on the
 * way in.
 */
export function fromRows(rows: readonly (readonly number[])[]): NDArray {
  const nrows = rows.length;
  if (nrows === 0) {
    return ndarray([0, 0], new Float64Array(0));
  }
  const ncols = rows[0].length;
  if (rows.some((r) => r.length !== ncols)) {
    throw new Error("fromRows: ragged rows");
  }
  const n = checkedShapeSize([nrows, ncols]);
  const data = new Float64Array(n);
  for (let r = 0; r < nrows; r++) {
    for (let c = 0; c < ncols; c++) {
      data[c * nrows + r] = rows[r][c]; // column-major store
    }
  }
  return ndarray([nrows, ncols], data);
}

/** An `[rows, cols]` array of zeros. */
export function zeros(rows: number, cols: number): NDArray {
  const n = checkedShapeSize([rows, cols]);
  return ndarray([rows, cols], new Float64Array(n));
}

export function ndims(a: NDArray): number {
  return a.shape.length;
}

export function isScalar(a: NDArray): boolean {
  return a.data.length === 1;
}

/** Rows, treating a scalar as `1×1` and a vector `[n]` as `n×1`. */
export function nrows(a: NDArray): number {
  switch (a.shape.length) {
    case 0:
      return 1;
    default:
      return a.shape[0];
  }
}

/** Columns, treating a scalar as `1×1` and a vector `[n]` as `n×1`. */
export function ncols(a: NDArray): number {
  switch (a.shape.length) {
    case 0:
    case 1:
      return 1;
    default:
      return a.shape[1];
  }
}

/** Element `(r, c)` (column-major), or `undefined` if out of bounds. */
export function get(a: NDArray, r: number, c: number): number | undefined {
  if (r >= 0 && c >= 0 && r < nrows(a) && c < ncols(a)) {
    return a.data[c * nrows(a) + r];
  }
  return undefined;
}

/**
 * Set element `(r, c)` in place (column-major). Mutates `a.data` directly —
 * matches MATLAB assignment semantics (`A(i,j) = v` rebinds one element of
 * the existing array, it does not produce a new one), and the SIR22 spec's
 * `IndexSet` is a *statement*, not a pure expression, for exactly this
 * reason.
 */
export function set(a: NDArray, r: number, c: number, value: number): void {
  // SECURITY: written as the negation of `get`'s AND-form
  // (`!(r >= 0 && ...)`), not as an OR-form (`r < 0 || ...`) — under
  // IEEE-754 those are NOT equivalent for NaN: every relational
  // comparison with NaN is false, so an OR-form check would have every
  // branch evaluate false for r=NaN, silently skipping the throw.
  // `a.data[c * nrows(a) + NaN] = value` would then set a stray,
  // non-index property on the `Float64Array` rather than writing the
  // buffer — the exact same silent-write-drop bug `indexSet`'s own fix
  // (via `resolvePositions`/`assertValidPosition` in `indexing.ts`)
  // closes for its call path into this function. `set` itself is not
  // reachable with an unvalidated NaN today (every caller resolves
  // positions through `assertValidPosition` first), but it is part of
  // this module's exported public surface, so it stays NaN-safe on its
  // own rather than relying on every future caller to re-derive that
  // invariant.
  if (!(r >= 0 && c >= 0 && r < nrows(a) && c < ncols(a))) {
    throw new Error(`set: index (${r}, ${c}) out of bounds for shape ${JSON.stringify(a.shape)}`);
  }
  a.data[c * nrows(a) + r] = value;
}
