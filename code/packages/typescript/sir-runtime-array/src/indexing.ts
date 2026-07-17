import { get, set, ndarray, checkedShapeSize, nrows, ncols, type NDArray } from "./ndarray.js";

/**
 * One MATLAB-style index-position argument — mirrors the SIR22 spec's
 * `IndexArg` exactly:
 *
 * ```text
 * IndexArg = Scalar(Box<Expr>) | Whole | Range(Box<Expr>)
 * ```
 *
 * `end`-relative indices (`A(end)`, `A(end-1)`) are never seen here — per
 * SIR10 discipline, the frontend resolves `end` to a concrete 0-based
 * `Scalar` index before emitting `IndexGet`/`IndexSet`, so this module only
 * ever deals in already-resolved, already-0-based positions.
 */
export type IndexArg =
  | { kind: "scalar"; value: number }
  | { kind: "whole" }
  | { kind: "range"; indices: NDArray };

/**
 * Validate one resolved position is a real, finite integer.
 *
 * SECURITY: `indexGet`/`indexSet`'s own bounds checks further down this
 * file compare a position against `0`/`dimSize` with `<`/`>=`. Under
 * IEEE-754, every relational comparison with `NaN` is `false` — so a
 * comparison-based check alone would let `i = NaN` sail through as
 * neither "too small" nor "too large", silently reading/writing a stray
 * non-index property instead of throwing. A position reaching this
 * function comes from the *compiled program's own runtime arithmetic*
 * (e.g. `0/0`), not just a hand-built edge case, so this validates once,
 * here — the single choke point every `resolvePositions` caller routes
 * through — rather than re-deriving a NaN-safe check at each call site.
 */
function assertValidPosition(i: number): number {
  if (!Number.isInteger(i)) {
    throw new Error(`resolvePositions: index ${i} is not a finite integer`);
  }
  return i;
}

/** Resolve one `IndexArg` against a dimension of size `dimSize` into a flat list of 0-based positions along that dimension. */
function resolvePositions(arg: IndexArg, dimSize: number): number[] {
  switch (arg.kind) {
    case "scalar":
      return [assertValidPosition(arg.value)];
    case "whole":
      return Array.from({ length: dimSize }, (_, i) => i);
    case "range":
      return Array.from(arg.indices.data, (x) => assertValidPosition(Math.trunc(x)));
    default:
      // Emitted code crosses a JS runtime boundary that TypeScript can't
      // enforce at the actual call site — a malformed `kind` must fail
      // cleanly here, not fall through to `undefined` and surface as a
      // confusing `TypeError` several calls further down.
      throw new Error(`resolvePositions: unrecognised IndexArg ${JSON.stringify(arg)}`);
  }
}

/**
 * `A(i)` / `A(i, j)` — read one element or a sub-array. Scoped to 1 or 2
 * index arguments (rank ≤ 2, matching this whole package's scope): a single
 * argument indexes `a`'s underlying column-major data linearly (MATLAB's own
 * single-subscript convention, which is column-major too — the *same*
 * order this package already stores data in, so no reordering is needed);
 * two arguments index `(row, col)`. Returns a bare `number` when every
 * argument is `scalar` (a single element), otherwise an `NDArray`.
 */
export function indexGet(a: NDArray, indices: readonly IndexArg[]): NDArray | number {
  if (indices.length === 1) {
    const [arg] = indices;
    const positions = resolvePositions(arg, a.data.length);
    const read = (i: number): number => {
      if (i < 0 || i >= a.data.length) {
        throw new Error(`indexGet: linear index ${i} out of bounds`);
      }
      return a.data[i];
    };
    if (arg.kind === "scalar") {
      return read(positions[0]);
    }
    return ndarray([1, positions.length], Float64Array.from(positions, read));
  }
  if (indices.length === 2) {
    const [rowArg, colArg] = indices;
    const rows = resolvePositions(rowArg, nrows(a));
    const cols = resolvePositions(colArg, ncols(a));
    const read = (r: number, c: number): number => {
      const v = get(a, r, c);
      if (v === undefined) {
        throw new Error(`indexGet: (${r}, ${c}) out of bounds for shape ${JSON.stringify(a.shape)}`);
      }
      return v;
    };
    if (rowArg.kind === "scalar" && colArg.kind === "scalar") {
      return read(rows[0], cols[0]);
    }
    // `rows.length`/`cols.length` are each individually bounded by
    // `a`'s own dimensions (`whole`) or by a `range` NDArray's own
    // `MAX_ELEMENTS` cap — but nothing bounds their *product* on its own
    // (a `range`-selected row list and a `range`-selected column list can
    // each independently approach `MAX_ELEMENTS`), so this is the exact
    // outer-product-shaped allocation `matmul` guards against, one level
    // up. Validate before allocating, not after.
    const outLen = checkedShapeSize([rows.length, cols.length]);
    const data = new Float64Array(outLen);
    for (let c = 0; c < cols.length; c++) {
      for (let r = 0; r < rows.length; r++) {
        data[c * rows.length + r] = read(rows[r], cols[c]);
      }
    }
    return ndarray([rows.length, cols.length], data);
  }
  throw new Error(`indexGet: only 1 or 2 index arguments are supported (rank ≤ 2 scope), got ${indices.length}`);
}

/** Broadcast a scalar-or-`NDArray` right-hand side to exactly `count` values (mirrors [`elementwise`](./elementwise.js)'s scalar-broadcast rule). */
function broadcastValues(value: number | NDArray, count: number): Float64Array {
  if (typeof value === "number") {
    return new Float64Array(count).fill(value);
  }
  if (value.data.length === 1) {
    return new Float64Array(count).fill(value.data[0]);
  }
  if (value.data.length !== count) {
    throw new Error(`indexSet: value has ${value.data.length} elements, expected ${count}`);
  }
  return value.data;
}

/**
 * `A(i) = v` / `A(i, j) = v` — write one element or a sub-array, **in
 * place** (see [`set`](./ndarray.js)'s doc comment for why this mutates
 * rather than returns a new array — the SIR22 spec makes `IndexSet` a
 * statement, not a pure expression, for the same reason). `value` may be a
 * scalar (broadcast to every selected position) or an `NDArray` with
 * exactly as many elements as positions are selected.
 */
export function indexSet(a: NDArray, indices: readonly IndexArg[], value: number | NDArray): void {
  if (indices.length === 1) {
    const [arg] = indices;
    const positions = resolvePositions(arg, a.data.length);
    const values = broadcastValues(value, positions.length);
    positions.forEach((i, k) => {
      if (i < 0 || i >= a.data.length) {
        throw new Error(`indexSet: linear index ${i} out of bounds`);
      }
      a.data[i] = values[k];
    });
    return;
  }
  if (indices.length === 2) {
    const [rowArg, colArg] = indices;
    const rows = resolvePositions(rowArg, nrows(a));
    const cols = resolvePositions(colArg, ncols(a));
    // Same product-of-two-independent-selections gap `indexGet` closes
    // above — validate before `broadcastValues` allocates.
    const count = checkedShapeSize([rows.length, cols.length]);
    const values = broadcastValues(value, count);
    let k = 0;
    for (let c = 0; c < cols.length; c++) {
      for (let r = 0; r < rows.length; r++) {
        set(a, rows[r], cols[c], values[k]);
        k++;
      }
    }
    return;
  }
  throw new Error(`indexSet: only 1 or 2 index arguments are supported (rank ≤ 2 scope), got ${indices.length}`);
}
