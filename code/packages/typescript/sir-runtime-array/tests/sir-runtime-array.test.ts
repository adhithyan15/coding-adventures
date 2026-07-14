/** Tests for @coding-adventures/sir-runtime-array. */

import { describe, it, expect } from "vitest";
import * as arr from "../src/index.js";

describe("ndarray construction and accessors", () => {
  it("scalar is rank-0, length-1", () => {
    const s = arr.scalar(42);
    expect(arr.isScalar(s)).toBe(true);
    expect(arr.ndims(s)).toBe(0);
    expect(s.shape).toEqual([]);
    expect(arr.nrows(s)).toBe(1);
    expect(arr.ncols(s)).toBe(1);
    expect(arr.get(s, 0, 0)).toBe(42);
  });

  it("fromVec is a column vector (n x 1)", () => {
    const v = arr.fromVec([1, 2, 3]);
    expect(v.shape).toEqual([3]);
    expect(arr.ndims(v)).toBe(1);
    expect(arr.nrows(v)).toBe(3);
    expect(arr.ncols(v)).toBe(1);
    expect(arr.isScalar(v)).toBe(false);
  });

  it("fromVec validates length before allocating, even for a bare array-like with no real elements", () => {
    // `Float64Array.from` accepts any `{ length: N }` array-like, not just a
    // real `number[]` — a caller could pass one with no backing elements at
    // all, costing them nothing while requesting an N-sized allocation.
    const arrayLike = { length: 1e10 } as unknown as readonly number[];
    expect(() => arr.fromVec(arrayLike)).toThrow(/exceeds/);
  });

  it("fromRows stores column-major", () => {
    // Row-major input [[1,2,3],[4,5,6]] becomes column-major [1,4,2,5,3,6].
    const a = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    expect(a.shape).toEqual([2, 3]);
    expect(Array.from(a.data)).toEqual([1, 4, 2, 5, 3, 6]);
    expect(arr.get(a, 0, 0)).toBe(1);
    expect(arr.get(a, 1, 2)).toBe(6);
    expect(arr.get(a, 0, 2)).toBe(3);
  });

  it("fromRows rejects ragged rows", () => {
    expect(() => arr.fromRows([[1, 2], [3]])).toThrow(/ragged/);
  });

  it("fromRows of zero rows is a 0x0 array", () => {
    const empty = arr.fromRows([]);
    expect(empty.shape).toEqual([0, 0]);
    expect(empty.data.length).toBe(0);
  });

  it("zeros builds an all-zero matrix of the given shape", () => {
    const z = arr.zeros(2, 3);
    expect(z.shape).toEqual([2, 3]);
    expect(Array.from(z.data)).toEqual([0, 0, 0, 0, 0, 0]);
  });

  it("get is out-of-bounds-safe", () => {
    const a = arr.fromRows([[1, 2]]); // 1x2
    expect(arr.get(a, 0, 1)).toBe(2);
    expect(arr.get(a, 1, 0)).toBeUndefined(); // row OOB
    expect(arr.get(a, 0, 2)).toBeUndefined(); // col OOB
  });

  it("set mutates in place", () => {
    const a = arr.zeros(2, 2);
    arr.set(a, 1, 0, 9);
    expect(arr.get(a, 1, 0)).toBe(9);
    expect(arr.get(a, 0, 0)).toBe(0);
  });

  it("set rejects an out-of-bounds index", () => {
    const a = arr.zeros(1, 1);
    expect(() => arr.set(a, 1, 0, 1)).toThrow(/out of bounds/);
  });

  it("ndarray rejects a shape/data-length mismatch", () => {
    expect(() => arr.ndarray([2, 2], Float64Array.of(1, 2, 3))).toThrow(/implies/);
  });

  it("ndarray rejects a shape exceeding MAX_ELEMENTS", () => {
    expect(() => arr.ndarray([arr.MAX_ELEMENTS + 1], new Float64Array(arr.MAX_ELEMENTS + 1))).toThrow(
      /exceeds/,
    );
  });

  it("checkedShapeSize rejects negative and non-integer dimensions before any allocation", () => {
    expect(() => arr.checkedShapeSize([-2, 5])).toThrow(/negative or non-integer/);
    expect(() => arr.checkedShapeSize([2.5, 5])).toThrow(/negative or non-integer/);
  });

  it("zeros/fromRows validate the shape before allocating, not after", () => {
    // Two negative dims multiply to a small positive product — a naive
    // `rows * cols` check alone would miss this; `checkedShapeSize`
    // rejects the negative dimension directly instead.
    expect(() => arr.zeros(-100, -100)).toThrow(/negative or non-integer/);
  });
});

describe("elementwise", () => {
  it("elementwise arithmetic on equal shapes", () => {
    const a = arr.fromVec([1, 2, 3]);
    const b = arr.fromVec([10, 20, 30]);
    expect(Array.from(arr.elementwise("Add", a, b).data)).toEqual([11, 22, 33]);
    expect(Array.from(arr.elementwise("Sub", b, a).data)).toEqual([9, 18, 27]);
    expect(Array.from(arr.elementwise("Mul", a, b).data)).toEqual([10, 40, 90]);
    expect(Array.from(arr.elementwise("Div", b, a).data)).toEqual([10, 10, 10]);
  });

  it("Pow raises elementwise (the one op array-runtime's BinOp lacks)", () => {
    const a = arr.fromVec([2, 3, 4]);
    const b = arr.fromVec([3, 2, 1]);
    expect(Array.from(arr.elementwise("Pow", a, b).data)).toEqual([8, 9, 4]);
  });

  it("a malformed op (only reachable from untyped emitted JS) is a clean error, not silent NaN corruption", () => {
    const a = arr.fromVec([1, 2]);
    const b = arr.fromVec([3, 4]);
    const bogus = "Bogus" as unknown as arr.ElementwiseOpKind;
    expect(() => arr.elementwise(bogus, a, b)).toThrow(/unrecognised ElementwiseOpKind/);
  });

  it("scalar broadcasts on either side, result keeps the array operand's shape", () => {
    const v = arr.fromVec([1, 2, 3]);
    const s = arr.scalar(2);
    expect(Array.from(arr.elementwise("Mul", s, v).data)).toEqual([2, 4, 6]);
    const r = arr.elementwise("Add", v, s);
    expect(Array.from(r.data)).toEqual([3, 4, 5]);
    expect(r.shape).toEqual([3]);
  });

  it("two scalars stay a scalar", () => {
    const r = arr.elementwise("Add", arr.scalar(2), arr.scalar(40));
    expect(arr.isScalar(r)).toBe(true);
    expect(Array.from(r.data)).toEqual([42]);
  });

  it("non-conformable shapes are an error", () => {
    const a = arr.fromVec([1, 2]);
    const b = arr.fromVec([1, 2, 3]);
    expect(() => arr.elementwise("Add", a, b)).toThrow(/non-conformable/);
  });

  it("comparisons produce APL-style 1/0, never a native boolean", () => {
    const a = arr.fromVec([1, 2, 3]);
    const b = arr.fromVec([3, 2, 1]);
    expect(Array.from(arr.elementwise("Eq", a, b).data)).toEqual([0, 1, 0]);
    expect(Array.from(arr.elementwise("Ne", a, b).data)).toEqual([1, 0, 1]);
    expect(Array.from(arr.elementwise("Lt", a, b).data)).toEqual([1, 0, 0]);
    expect(Array.from(arr.elementwise("Le", a, b).data)).toEqual([1, 1, 0]);
    expect(Array.from(arr.elementwise("Ge", a, b).data)).toEqual([0, 1, 1]);
    expect(Array.from(arr.elementwise("Gt", a, b).data)).toEqual([0, 0, 1]);
  });

  it("Max/Min are elementwise", () => {
    const a = arr.fromVec([1, 5, 3]);
    const b = arr.fromVec([4, 2, 3]);
    expect(Array.from(arr.elementwise("Max", a, b).data)).toEqual([4, 5, 3]);
    expect(Array.from(arr.elementwise("Min", a, b).data)).toEqual([1, 2, 3]);
  });

  it("NaN/Infinity propagate through Div", () => {
    const a = arr.fromVec([1, 0]);
    const b = arr.fromVec([0, 0]);
    const q = arr.elementwise("Div", a, b);
    expect(q.data[0]).toBe(Infinity);
    expect(Number.isNaN(q.data[1])).toBe(true);
  });
});

describe("matmul", () => {
  it("identity leaves a matrix unchanged", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]);
    const identity = arr.fromRows([
      [1, 0],
      [0, 1],
    ]);
    expect(Array.from(arr.matmul(a, identity).data)).toEqual(Array.from(a.data));
  });

  it("2x2 · 2x2 product", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]);
    const b = arr.fromRows([
      [5, 6],
      [7, 8],
    ]);
    const c = arr.matmul(a, b);
    expect(c.shape).toEqual([2, 2]);
    expect(arr.get(c, 0, 0)).toBe(19);
    expect(arr.get(c, 0, 1)).toBe(22);
    expect(arr.get(c, 1, 0)).toBe(43);
    expect(arr.get(c, 1, 1)).toBe(50);
  });

  it("non-square (2x3 · 3x1 -> 2x1)", () => {
    const a = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    const x = arr.fromRows([[1], [0], [-1]]);
    const y = arr.matmul(a, x);
    expect(y.shape).toEqual([2, 1]);
    expect(Array.from(y.data)).toEqual([-2, -2]);
  });

  it("dimension mismatch is an error", () => {
    const a = arr.fromRows([[1, 2]]); // 1x2
    const b = arr.fromRows([[1, 2]]); // 1x2
    expect(() => arr.matmul(a, b)).toThrow(/inner dimensions disagree/);
  });

  it("an outer-product-shaped call whose output would exceed MAX_ELEMENTS is a clean error, not an OOM", () => {
    // Each operand is individually tiny; only their PRODUCT (m * n) is
    // absurd. This proves the output shape is validated before `matmul`
    // ever allocates `out`, not after — an allocate-then-validate ordering
    // would attempt the huge allocation first.
    const col: arr.NDArray = { shape: [100000, 1], data: new Float64Array(1) };
    const row: arr.NDArray = { shape: [1, 100000], data: new Float64Array(1) };
    expect(() => arr.matmul(col, row)).toThrow(/exceeds/);
  });
});

describe("transpose", () => {
  it("swaps rows and columns", () => {
    const a = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    const t = arr.transpose(a);
    expect(t.shape).toEqual([3, 2]);
    expect(arr.get(t, 0, 0)).toBe(1);
    expect(arr.get(t, 2, 1)).toBe(6);
  });

  it("is an involution", () => {
    const a = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    expect(Array.from(arr.transpose(arr.transpose(a)).data)).toEqual(Array.from(a.data));
  });

  it("conjugate flag is a no-op on real data (no Complex type yet)", () => {
    const a = arr.fromRows([[1, 2]]);
    expect(Array.from(arr.transpose(a, true).data)).toEqual(Array.from(arr.transpose(a, false).data));
  });
});

describe("range", () => {
  it("unit-step range is a 1xn row vector", () => {
    const r = arr.range(1, 5);
    expect(r.shape).toEqual([1, 5]);
    expect(Array.from(r.data)).toEqual([1, 2, 3, 4, 5]);
  });

  it("stepped range", () => {
    const r = arr.range(1, 10, 2);
    expect(Array.from(r.data)).toEqual([1, 3, 5, 7, 9]);
  });

  it("negative step counts down", () => {
    const r = arr.range(5, 1, -1);
    expect(Array.from(r.data)).toEqual([5, 4, 3, 2, 1]);
  });

  it("floating step is inclusive of stop within tolerance", () => {
    const r = arr.range(0, 1, 0.25);
    expect(Array.from(r.data)).toEqual([0, 0.25, 0.5, 0.75, 1]);
  });

  it("an empty range (start already past stop) is a 1x0 array", () => {
    const r = arr.range(5, 1);
    expect(r.shape).toEqual([1, 0]);
    expect(r.data.length).toBe(0);
  });

  it("zero step is an error", () => {
    expect(() => arr.range(1, 5, 0)).toThrow(/step cannot be zero/);
  });

  it("a range that would produce more than MAX_ELEMENTS is a clean error, not an OOM", () => {
    // A compiled program's range bounds can be attacker-influenced (runtime
    // values, not fixed at compile time) — this proves the cap actually
    // trips rather than trusting the code that it would.
    expect(() => arr.range(1, Number.MAX_SAFE_INTEGER)).toThrow(/produces more than/);
  });
});

describe("indexGet / indexSet", () => {
  it("2-index scalar read", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]);
    expect(arr.indexGet(a, [{ kind: "scalar", value: 1 }, { kind: "scalar", value: 0 }])).toBe(3);
  });

  it("whole-row / whole-column selection", () => {
    const a = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    const row0 = arr.indexGet(a, [{ kind: "scalar", value: 0 }, { kind: "whole" }]);
    expect(row0).not.toBeTypeOf("number");
    expect(Array.from((row0 as arr.NDArray).data)).toEqual([1, 2, 3]);

    const col1 = arr.indexGet(a, [{ kind: "whole" }, { kind: "scalar", value: 1 }]);
    expect(Array.from((col1 as arr.NDArray).data)).toEqual([2, 5]);
  });

  it("range-selected sub-array", () => {
    const a = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    const cols = arr.range(1, 2); // 0-based columns 1..2, already resolved by the frontend
    const sub = arr.indexGet(a, [{ kind: "whole" }, { kind: "range", indices: cols }]);
    expect((sub as arr.NDArray).shape).toEqual([2, 2]);
    expect(Array.from((sub as arr.NDArray).data)).toEqual([2, 5, 3, 6]);
  });

  it("a 2-index sub-array selection whose row x col product would exceed MAX_ELEMENTS is a clean error, not an OOM", () => {
    // Each of `rows`/`cols` is individually a perfectly legitimate
    // range-selection size (100,000 positions) — only their PRODUCT is
    // absurd, exactly the outer-product-shaped gap `matmul` guards
    // against, one level up in `indexGet`/`indexSet`.
    const a = arr.zeros(1, 1);
    const bigSelection: arr.NDArray = {
      shape: [1, 100000],
      data: Float64Array.from({ length: 100000 }, (_, i) => i),
    };
    const twoBigRanges = [
      { kind: "range" as const, indices: bigSelection },
      { kind: "range" as const, indices: bigSelection },
    ];
    expect(() => arr.indexGet(a, twoBigRanges)).toThrow(/exceeds/);
    expect(() => arr.indexSet(a, twoBigRanges, 0)).toThrow(/exceeds/);
  });

  it("single-argument linear indexing", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]); // column-major data: [1, 3, 2, 4]
    expect(arr.indexGet(a, [{ kind: "scalar", value: 2 }])).toBe(2);
  });

  it("single-argument whole-array linear read returns every element as a row vector", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]); // column-major data: [1, 3, 2, 4]
    const all = arr.indexGet(a, [{ kind: "whole" }]);
    expect((all as arr.NDArray).shape).toEqual([1, 4]);
    expect(Array.from((all as arr.NDArray).data)).toEqual([1, 3, 2, 4]);
  });

  it("single-argument linear read out of bounds is a clean error", () => {
    const a = arr.fromVec([1, 2]);
    expect(() => arr.indexGet(a, [{ kind: "scalar", value: 9 }])).toThrow(/out of bounds/);
  });

  it("indexGet out of bounds is a clean error", () => {
    const a = arr.fromRows([[1, 2]]);
    expect(() => arr.indexGet(a, [{ kind: "scalar", value: 5 }, { kind: "scalar", value: 0 }])).toThrow(
      /out of bounds/,
    );
  });

  it("indexSet writes a scalar in place", () => {
    const a = arr.zeros(2, 2);
    arr.indexSet(a, [{ kind: "scalar", value: 0 }, { kind: "scalar", value: 1 }], 9);
    expect(arr.get(a, 0, 1)).toBe(9);
  });

  it("indexSet with a single (linear) index writes into the underlying data", () => {
    const a = arr.zeros(2, 2); // column-major data [0, 0, 0, 0]
    arr.indexSet(a, [{ kind: "scalar", value: 2 }], 5);
    expect(arr.get(a, 0, 1)).toBe(5); // linear position 2 is (row 0, col 1)
  });

  it("indexSet with a single whole-array index broadcasts a scalar to every element", () => {
    const a = arr.zeros(1, 3);
    arr.indexSet(a, [{ kind: "whole" }], 4);
    expect(Array.from(a.data)).toEqual([4, 4, 4]);
  });

  it("indexSet with a single out-of-bounds linear index is an error", () => {
    const a = arr.zeros(1, 2);
    expect(() => arr.indexSet(a, [{ kind: "scalar", value: 5 }], 1)).toThrow(/out of bounds/);
  });

  it("indexSet value can be an NDArray matching the selection count exactly", () => {
    const a = arr.zeros(1, 3);
    arr.indexSet(a, [{ kind: "scalar", value: 0 }, { kind: "whole" }], arr.fromVec([1, 2, 3]));
    expect(Array.from(a.data)).toEqual([1, 2, 3]);
  });

  it("indexSet value can be a length-1 NDArray, broadcast like a scalar", () => {
    const a = arr.zeros(1, 3);
    arr.indexSet(a, [{ kind: "scalar", value: 0 }, { kind: "whole" }], arr.scalar(6));
    expect(Array.from(a.data)).toEqual([6, 6, 6]);
  });

  it("indexSet broadcasts a scalar across a whole-row selection", () => {
    const a = arr.zeros(2, 3);
    arr.indexSet(a, [{ kind: "scalar", value: 0 }, { kind: "whole" }], 7);
    expect(arr.get(a, 0, 0)).toBe(7);
    expect(arr.get(a, 0, 1)).toBe(7);
    expect(arr.get(a, 0, 2)).toBe(7);
    expect(arr.get(a, 1, 0)).toBe(0);
  });

  it("indexSet with a mismatched value length is an error", () => {
    const a = arr.zeros(1, 3);
    expect(() =>
      arr.indexSet(a, [{ kind: "scalar", value: 0 }, { kind: "whole" }], arr.fromVec([1, 2])),
    ).toThrow(/expected 3/);
  });

  it("indexGet/indexSet reject more than 2 index arguments", () => {
    const a = arr.zeros(1, 1);
    const tooMany = [
      { kind: "scalar" as const, value: 0 },
      { kind: "scalar" as const, value: 0 },
      { kind: "scalar" as const, value: 0 },
    ];
    expect(() => arr.indexGet(a, tooMany)).toThrow(/rank ≤ 2/);
    expect(() => arr.indexSet(a, tooMany, 1)).toThrow(/rank ≤ 2/);
  });

  it("a malformed IndexArg (only reachable from untyped emitted JS, not real TypeScript callers) is a clean error", () => {
    const a = arr.zeros(1, 1);
    // `as unknown as arr.IndexArg` simulates a compiled-JS call site that
    // TypeScript's own type-checking can't police at runtime.
    const malformed = { kind: "bogus" } as unknown as arr.IndexArg;
    expect(() => arr.indexGet(a, [malformed])).toThrow(/unrecognised IndexArg/);
  });
});
