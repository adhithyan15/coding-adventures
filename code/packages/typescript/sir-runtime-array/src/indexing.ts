import { get, set, ndarray, nrows, ncols, type NDArray } from "./ndarray.js";

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

/** Resolve one `IndexArg` against a dimension of size `dimSize` into a flat list of 0-based positions along that dimension. */
function resolvePositions(arg: IndexArg, dimSize: number): number[] {
  switch (arg.kind) {
    case "scalar":
      return [arg.value];
    case "whole":
      return Array.from({ length: dimSize }, (_, i) => i);
    case "range":
      return Array.from(arg.indices.data, (x) => Math.trunc(x));
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
    const data = new Float64Array(rows.length * cols.length);
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
    const values = broadcastValues(value, rows.length * cols.length);
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
