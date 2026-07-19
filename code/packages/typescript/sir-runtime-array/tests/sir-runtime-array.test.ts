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

  it("ndarray rejects a data buffer that isn't really a Float64Array", () => {
    // NDArray is a plain structural interface, not a class — a compiled-JS
    // caller could hand back an object shaped like one whose `data` is a
    // plain array or array-like instead of a real Float64Array.
    const notReallyFloat64 = [1, 2, 3, 4] as unknown as Float64Array;
    expect(() => arr.ndarray([2, 2], notReallyFloat64)).toThrow(/must be a Float64Array/);
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

  it("a bare number operand is coerced to a scalar, not read as `.data`/`.shape` directly", () => {
    // Mirrors matlab-to-semantic-ir's lowering: `A .* 2` emits `2` as a bare
    // number, not wrapped in an ArrayLit/scalar-array constructor first.
    const v = arr.fromVec([1, 2, 3]);
    const r = arr.elementwise("Mul", v, 2);
    expect(Array.from(r.data)).toEqual([2, 4, 6]);
    const r2 = arr.elementwise("Add", 10, v);
    expect(Array.from(r2.data)).toEqual([11, 12, 13]);
  });

  it("two bare number operands both coerce and stay a scalar", () => {
    const r = arr.elementwise("Add", 2, 40);
    expect(arr.isScalar(r)).toBe(true);
    expect(Array.from(r.data)).toEqual([42]);
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

  it(
    "a range that would produce more than MAX_ELEMENTS is a clean error, not an OOM",
    () => {
      // A compiled program's range bounds can be attacker-influenced (runtime
      // values, not fixed at compile time) — this proves the cap actually
      // trips rather than trusting the code that it would. Pushing ~2^26
      // elements before the cap trips is inherently CPU-bound work, not a
      // hang — the default 5000ms timeout is too tight on a loaded CI
      // runner (observed timing out on macOS), so this test gets a longer
      // budget rather than a smaller repro that wouldn't actually exercise
      // the cap.
      expect(() => arr.range(1, Number.MAX_SAFE_INTEGER)).toThrow(/produces more than/);
    },
    20000,
  );

  it("a NaN start/stop/step is a clean error, not a silently empty range", () => {
    // Without the Number.isFinite guard, the while loop's condition is
    // false on its very first check for a NaN bound (every relational
    // comparison with NaN is false), so this would silently return a
    // valid-looking `[1, 0]` empty array instead of erroring.
    expect(() => arr.range(NaN, 5)).toThrow(/must be finite/);
    expect(() => arr.range(1, NaN)).toThrow(/must be finite/);
    expect(() => arr.range(1, 5, NaN)).toThrow(/must be finite/);
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

  it("a NaN scalar index is a clean error for indexGet, not a silent wrong read", () => {
    // `NDArray` index values come from the compiled program's own runtime
    // arithmetic (e.g. `0/0`), not just a hand-built edge case. Without
    // `assertValidPosition`, the linear path's bounds check
    // (`i < 0 || i >= length`) is an OR-form that is NOT the negation of a
    // comparison under IEEE-754 for i=NaN (every relational comparison with
    // NaN is false), so the throw would be skipped and `a.data[NaN]` would
    // silently return `undefined` instead.
    const a = arr.fromVec([1, 2, 3]);
    expect(() => arr.indexGet(a, [{ kind: "scalar", value: NaN }])).toThrow(/not a finite integer/);
  });

  it("a NaN scalar index is a clean error for indexSet, not a silently dropped write", () => {
    const a = arr.zeros(1, 3);
    expect(() => arr.indexSet(a, [{ kind: "scalar", value: NaN }], 9)).toThrow(/not a finite integer/);
  });

  it("a non-integer scalar index is a clean error, not truncated silently", () => {
    const a = arr.fromVec([1, 2, 3]);
    expect(() => arr.indexGet(a, [{ kind: "scalar", value: 1.5 }])).toThrow(/not a finite integer/);
  });

  it("set() itself rejects a NaN row/col even though not reachable via indexGet/indexSet today", () => {
    // `set` is part of this module's exported public surface; every
    // current caller resolves positions through `assertValidPosition`
    // first, but `set` stays NaN-safe on its own rather than relying on
    // that invariant holding forever.
    const a = arr.zeros(2, 2);
    expect(() => arr.set(a, NaN, 0, 9)).toThrow(/out of bounds/);
    expect(() => arr.set(a, 0, NaN, 9)).toThrow(/out of bounds/);
  });
});

// ── SIR22 addendum: APL primitive operators ─────────────────────────────
//
// `reduce`/`scan`/`outer`/`shape`/`reshape`/`indexGenerator`/`indexOf`/
// `ravel`/`catenate` — ported from `array_runtime::ops::{reduce,scan,outer}`
// and `apl_runtime::builtins::{shape,reshape,index_generator,index_of,
// ravel,catenate}`, by way of `semantic-ir-to-javascript`'s own already-
// merged, already-reviewed inlined port of the same nine functions (see
// that crate's "SIR22 APL-addendum codegen" PR). Every rank case each
// function documents supporting gets its own test, plus the two named
// correctness traps (column-major reduce/scan row-folding, and reshape's
// row-major-fill-transposed-into-column-major-storage requirement) and the
// two bounded-allocation checks (indexOf's O(len*len) product cap,
// catenate's combined-length cap).

describe("reduce / scan", () => {
  it("reduce/scan of a scalar is the scalar itself (nothing to fold)", () => {
    const s = arr.scalar(7);
    expect(Array.from(arr.reduce("Add", s).data)).toEqual([7]);
    expect(Array.from(arr.scan("Add", s).data)).toEqual([7]);
  });

  it("reduce folds a vector left-to-right", () => {
    const v = arr.fromVec([1, 2, 3, 4]);
    // +/v = ((1+2)+3)+4 = 10
    const r = arr.reduce("Add", v);
    expect(arr.isScalar(r)).toBe(true);
    expect(Array.from(r.data)).toEqual([10]);
    // ×/v = ((1×2)×3)×4 = 24
    expect(Array.from(arr.reduce("Mul", v).data)).toEqual([24]);
  });

  it("scan keeps every running fold, same shape as the input", () => {
    const v = arr.fromVec([1, 2, 3, 4]);
    // +\v = [1, 1+2, 1+2+3, 1+2+3+4] = [1, 3, 6, 10]
    const s = arr.scan("Add", v);
    expect(s.shape).toEqual([4]);
    expect(Array.from(s.data)).toEqual([1, 3, 6, 10]);
  });

  it("reduce folds each row of a matrix across its columns (the column-major indexing trap)", () => {
    // [[1,2,3],[4,5,6]] (2x3): row0 -> 1+2+3=6, row1 -> 4+5+6=15. The
    // backing store is column-major ([1,4,2,5,3,6], per `fromRows`'s own
    // test above) — a row/col-swapped indexing bug in the fold (reading
    // `d[row * c + col]` instead of the correct `d[col * r + row]`) would
    // silently produce [7, 14] instead: a plausible-looking but WRONG
        // answer, not a crash, which is exactly why this needs its own test
    // rather than trusting the code reads correctly.
    const m = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    const r = arr.reduce("Add", m);
    expect(r.shape).toEqual([2]);
    expect(Array.from(r.data)).toEqual([6, 15]);
  });

  it("scan scans each row of a matrix independently across its columns", () => {
    // [[1,2,3],[4,5,6]]: row0 running sums [1,3,6], row1 [4,9,15].
    const m = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    const s = arr.scan("Add", m);
    expect(s.shape).toEqual([2, 3]);
    expect(arr.get(s, 0, 0)).toBe(1);
    expect(arr.get(s, 0, 1)).toBe(3);
    expect(arr.get(s, 0, 2)).toBe(6);
    expect(arr.get(s, 1, 0)).toBe(4);
    expect(arr.get(s, 1, 1)).toBe(9);
    expect(arr.get(s, 1, 2)).toBe(15);
  });

  it("reduce/scan work with Max, not just Add/Mul (generic over any ElementwiseOpKind)", () => {
    const v = arr.fromVec([3, 7, 2, 9, 4]);
    expect(Array.from(arr.reduce("Max", v).data)).toEqual([9]);
    expect(Array.from(arr.scan("Max", v).data)).toEqual([3, 7, 7, 9, 9]);
  });

  it("reduce rejects an empty vector or an empty matrix row (no identity element for an arbitrary op)", () => {
    const emptyVec = arr.fromVec([]);
    expect(() => arr.reduce("Add", emptyVec)).toThrow(/cannot fold an empty vector/);
    expect(() => arr.reduce("Mul", emptyVec)).toThrow(/cannot fold an empty vector/);

    const emptyRowMatrix = arr.ndarray([2, 0], new Float64Array(0));
    expect(() => arr.reduce("Add", emptyRowMatrix)).toThrow(/cannot fold an empty row/);
  });

  it("scan of an empty vector is an empty vector, not an error", () => {
    const empty = arr.fromVec([]);
    const s = arr.scan("Add", empty);
    expect(s.shape).toEqual([0]);
    expect(s.data.length).toBe(0);
  });

  it("reduce/scan reject rank > 2", () => {
    const cube = arr.ndarray([2, 2, 2], new Float64Array(8));
    expect(() => arr.reduce("Add", cube)).toThrow(/rank > 2 not yet supported/);
    expect(() => arr.scan("Add", cube)).toThrow(/rank > 2 not yet supported/);
  });
});

describe("outer", () => {
  it("scalar outer scalar is a scalar", () => {
    const r = arr.outer("Mul", arr.scalar(6), arr.scalar(7));
    expect(arr.isScalar(r)).toBe(true);
    expect(Array.from(r.data)).toEqual([42]);
  });

  it("scalar outer vector broadcasts, either side", () => {
    const v = arr.fromVec([1, 2, 3]);
    const r1 = arr.outer("Mul", arr.scalar(10), v);
    expect(r1.shape).toEqual([3]);
    expect(Array.from(r1.data)).toEqual([10, 20, 30]);

    const r2 = arr.outer("Mul", v, arr.scalar(10));
    expect(Array.from(r2.data)).toEqual([10, 20, 30]);
  });

  it("vector outer vector is a rank-sum matrix", () => {
    // [1,2,3] outer-x [10,100] = [[10,100],[20,200],[30,300]] (3x2).
    const a = arr.fromVec([1, 2, 3]);
    const b = arr.fromVec([10, 100]);
    const r = arr.outer("Mul", a, b);
    expect(r.shape).toEqual([3, 2]);
    expect(arr.get(r, 0, 0)).toBe(10);
    expect(arr.get(r, 0, 1)).toBe(100);
    expect(arr.get(r, 1, 0)).toBe(20);
    expect(arr.get(r, 1, 1)).toBe(200);
    expect(arr.get(r, 2, 0)).toBe(30);
    expect(arr.get(r, 2, 1)).toBe(300);
  });

  it("outer Add matches manual pairwise sums", () => {
    const a = arr.fromVec([1, 2]);
    const b = arr.fromVec([100, 200, 300]);
    const r = arr.outer("Add", a, b);
    expect(r.shape).toEqual([2, 3]);
    for (let i = 0; i < 2; i++) {
      for (let j = 0; j < 3; j++) {
        expect(arr.get(r, i, j)).toBe(a.data[i] + b.data[j]);
      }
    }
  });

  it("outer rejects operands of rank > 1", () => {
    const m = arr.fromRows([
      [1, 2],
      [3, 4],
    ]);
    const v = arr.fromVec([1, 2]);
    expect(() => arr.outer("Add", m, v)).toThrow(/rank > 1 not yet supported/);
    expect(() => arr.outer("Add", v, m)).toThrow(/rank > 1 not yet supported/);
  });

  it("outer of an empty vector operand is an empty result, not an error", () => {
    const empty = arr.fromVec([]);
    const v = arr.fromVec([1, 2, 3]);
    const r = arr.outer("Mul", empty, v);
    expect(r.shape).toEqual([0, 3]);
    expect(r.data.length).toBe(0);
  });

  it("an outer-product call whose output would exceed MAX_ELEMENTS is a clean error, not an OOM", () => {
    // Each operand is individually tiny (its actual `data` is length 1);
    // only the claimed shape's PRODUCT (m * n) is absurd. Proves the output
    // shape is validated (`checkedShapeSize`) before `outer` ever allocates
    // `out`, not after — the same allocate-after-validate ordering
    // `matmul`'s own equivalent test proves.
    const a: arr.NDArray = { shape: [100000], data: new Float64Array(1) };
    const b: arr.NDArray = { shape: [100000], data: new Float64Array(1) };
    expect(() => arr.outer("Mul", a, b)).toThrow(/exceeds/);
  });
});

describe("shape / reshape", () => {
  it("shape of a scalar is the empty vector, not a scalar", () => {
    // ⍴5 is ⍳0-shaped: a length-0 vector, not a length-1 one.
    const s = arr.shape(arr.scalar(7));
    expect(s.shape).toEqual([0]);
    expect(s.data.length).toBe(0);
  });

  it("shape of a vector and a matrix", () => {
    expect(Array.from(arr.shape(arr.fromVec([1, 2, 3])).data)).toEqual([3]);
    const m = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    expect(Array.from(arr.shape(m).data)).toEqual([2, 3]);
  });

  it("reshape cycles a shorter source, filling in ROW-major order (the correctness trap)", () => {
    // 2 3⍴1 2 -- cycles [1, 2] to fill 6 elements, row-major fill order:
    // row0 = [1, 2, 1], row1 = [2, 1, 2]. Getting the row-major-fill-
    // transposed-into-column-major-storage step backwards here would
    // silently produce row0 = [1, 1, 1], row1 = [2, 2, 2] instead — the
    // right multiset of values, in the WRONG positions, which is exactly
    // the plausible-but-wrong failure mode this test exists to catch.
    const shapeArg = arr.fromVec([2, 3]);
    const source = arr.fromVec([1, 2]);
    const r = arr.reshape(shapeArg, source);
    expect(r.shape).toEqual([2, 3]);
    expect(arr.get(r, 0, 0)).toBe(1);
    expect(arr.get(r, 0, 1)).toBe(2);
    expect(arr.get(r, 0, 2)).toBe(1);
    expect(arr.get(r, 1, 0)).toBe(2);
    expect(arr.get(r, 1, 1)).toBe(1);
    expect(arr.get(r, 1, 2)).toBe(2);
  });

  it("reshape truncates a longer source", () => {
    // 2 2⍴1 2 3 4 5 6 -- only the first 4 elements are used.
    const shapeArg = arr.fromVec([2, 2]);
    const source = arr.fromVec([1, 2, 3, 4, 5, 6]);
    const r = arr.reshape(shapeArg, source);
    expect(r.shape).toEqual([2, 2]);
    expect(arr.get(r, 0, 0)).toBe(1);
    expect(arr.get(r, 0, 1)).toBe(2);
    expect(arr.get(r, 1, 0)).toBe(3);
    expect(arr.get(r, 1, 1)).toBe(4);
  });

  it("reshape into a rank <= 1 target needs no row-major/column-major transpose", () => {
    // Rank <= 1 is the branch where row-major and column-major coincide --
    // `filled` is handed straight to `ndarray` with no transpose step.
    const shapeArg = arr.fromVec([4]);
    const source = arr.fromVec([1, 2]);
    const r = arr.reshape(shapeArg, source);
    expect(r.shape).toEqual([4]);
    expect(Array.from(r.data)).toEqual([1, 2, 1, 2]);
  });

  it("reshape rejects a rank > 1 shape argument", () => {
    const shapeArg = arr.fromRows([[2, 2]]);
    const source = arr.fromVec([1]);
    expect(() => arr.reshape(shapeArg, source)).toThrow(/must be a scalar or vector/);
  });

  it("reshape rejects negative or non-integer shape elements", () => {
    const source = arr.fromVec([1]);
    expect(() => arr.reshape(arr.fromVec([-1]), source)).toThrow(/non-negative integers/);
    expect(() => arr.reshape(arr.fromVec([2.5]), source)).toThrow(/non-negative integers/);
  });

  it("reshape rejects a target shape of rank > 2", () => {
    const shapeArg = arr.fromVec([2, 2, 2]);
    const source = arr.fromVec([1]);
    expect(() => arr.reshape(shapeArg, source)).toThrow(/rank > 2 is not yet supported/);
  });

  it("reshape of an empty source into a non-empty target is an error", () => {
    const shapeArg = arr.fromVec([3]);
    const empty = arr.fromVec([]);
    expect(() => arr.reshape(shapeArg, empty)).toThrow(/cannot reshape an empty source/);
  });

  it("reshape caps the target element count before allocating", () => {
    const shapeArg = arr.fromVec([arr.MAX_ELEMENTS + 1]);
    const source = arr.fromVec([1]);
    expect(() => arr.reshape(shapeArg, source)).toThrow(/exceeds/);
  });
});

describe("indexGenerator / indexOf", () => {
  it("indexGenerator produces a 1-based run, unlike this package's 0-based indexGet/indexSet", () => {
    const r = arr.indexGenerator(arr.scalar(5));
    expect(Array.from(r.data)).toEqual([1, 2, 3, 4, 5]);
  });

  it("indexGenerator of zero is empty", () => {
    const r = arr.indexGenerator(arr.scalar(0));
    expect(r.data.length).toBe(0);
  });

  it("indexGenerator rejects negative and non-integer arguments", () => {
    expect(() => arr.indexGenerator(arr.scalar(-1))).toThrow(/non-negative integer/);
    expect(() => arr.indexGenerator(arr.scalar(2.5))).toThrow(/non-negative integer/);
  });

  it("indexGenerator rejects a non-scalar argument", () => {
    const v = arr.fromVec([1, 2]);
    expect(() => arr.indexGenerator(v)).toThrow(/must be a scalar/);
  });

  it("indexGenerator caps n before allocating", () => {
    const huge = arr.scalar(arr.MAX_ELEMENTS + 1);
    expect(() => arr.indexGenerator(huge)).toThrow(/exceeds/);
  });

  it("indexOf finds each needle's 1-based position, or haystack.length + 1 if not found", () => {
    const haystack = arr.fromVec([10, 20, 30]);
    const needles = arr.fromVec([20, 99, 10]);
    const r = arr.indexOf(haystack, needles);
    // 20 is at 1-based index 2, 99 is not found (len+1 = 4), 10 is at index 1.
    expect(Array.from(r.data)).toEqual([2, 4, 1]);
    expect(r.shape).toEqual(needles.shape);
  });

  it("indexOf rejects a haystack of rank > 1", () => {
    const m = arr.fromRows([
      [1, 2],
      [3, 4],
    ]);
    const needles = arr.fromVec([1]);
    expect(() => arr.indexOf(m, needles)).toThrow(/must be a scalar or vector/);
  });

  it("indexOf caps the O(len(haystack) * len(needle)) work product before scanning", () => {
    // Neither operand alone exceeds MAX_ELEMENTS, but their PRODUCT (the
    // work this does) does: 8200 * 8200 ≈ 67.24M > MAX_ELEMENTS (2^26 ≈
    // 67.11M) -- this is the shape a security review flags as unbounded if
    // the cap were missing, or checked each operand's own length instead of
    // their product. Both buffers are individually tiny to allocate (8200
    // zero-filled elements), so this stays fast despite exercising the cap.
    const n = 8200;
    const haystack = arr.ndarray([n], new Float64Array(n));
    const needle = arr.ndarray([n], new Float64Array(n));
    expect(() => arr.indexOf(haystack, needle)).toThrow(/exceeds/);
  });
});

describe("ravel / catenate", () => {
  it("ravel flattens a matrix in row-major order", () => {
    // [[1,2,3],[4,5,6]] ravels to [1,2,3,4,5,6] (row-major), even though
    // the backing store is column-major [1,4,2,5,3,6].
    const m = arr.fromRows([
      [1, 2, 3],
      [4, 5, 6],
    ]);
    expect(Array.from(arr.ravel(m).data)).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it("ravel of a scalar and a vector is a no-op reshape", () => {
    expect(Array.from(arr.ravel(arr.scalar(9)).data)).toEqual([9]);
    const v = arr.fromVec([1, 2]);
    expect(Array.from(arr.ravel(v).data)).toEqual([1, 2]);
  });

  it("ravel is total (not throwing) even on a rank > 2 array, though such input is unreachable via any function in this package", () => {
    // No function in this package's own public surface *produces* a rank > 2
    // `NDArray` -- but `ndarray()` itself doesn't reject one either (only
    // `reduce`/`scan`/`outer` explicitly cap at rank <= 2), so a directly
    // constructed rank-3 array is a real, if unusual, input `flattenRowMajor`
    // must stay total over rather than crash on — mirroring the Rust
    // reference's own `_ => a.data().to_vec()` fallback.
    const cube = arr.ndarray([2, 2, 2], Float64Array.from([1, 2, 3, 4, 5, 6, 7, 8]));
    const r = arr.ravel(cube);
    expect(r.shape).toEqual([8]);
    expect(Array.from(r.data)).toEqual([1, 2, 3, 4, 5, 6, 7, 8]);
  });

  it("catenate scalar and scalar", () => {
    const r = arr.catenate(arr.scalar(1), arr.scalar(2));
    expect(Array.from(r.data)).toEqual([1, 2]);
  });

  it("catenate scalar and vector prepends or appends", () => {
    const v = arr.fromVec([2, 3]);
    expect(Array.from(arr.catenate(arr.scalar(1), v).data)).toEqual([1, 2, 3]);
    expect(Array.from(arr.catenate(v, arr.scalar(4)).data)).toEqual([2, 3, 4]);
  });

  it("catenate vector and vector", () => {
    const a = arr.fromVec([1, 2]);
    const b = arr.fromVec([3, 4, 5]);
    expect(Array.from(arr.catenate(a, b).data)).toEqual([1, 2, 3, 4, 5]);
  });

  it("catenate matrices with equal row counts concatenates columns", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]); // 2x2
    const b = arr.fromRows([[5], [6]]); // 2x1
    const r = arr.catenate(a, b);
    expect(r.shape).toEqual([2, 3]);
    expect(arr.get(r, 0, 0)).toBe(1);
    expect(arr.get(r, 0, 1)).toBe(2);
    expect(arr.get(r, 0, 2)).toBe(5);
    expect(arr.get(r, 1, 2)).toBe(6);
  });

  it("catenate rejects mismatched matrix row counts", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]); // 2x2
    const b = arr.fromRows([[5, 6]]); // 1x2
    expect(() => arr.catenate(a, b)).toThrow(/equal row counts/);
  });

  it("catenate rejects a matrix paired with a vector", () => {
    const a = arr.fromRows([
      [1, 2],
      [3, 4],
    ]);
    const v = arr.fromVec([1, 2]);
    expect(() => arr.catenate(a, v)).toThrow(/not yet supported/);
    expect(() => arr.catenate(v, a)).toThrow(/not yet supported/);
  });

  it("catenate caps the combined length before allocating, re-checked on every call", () => {
    // Neither operand alone exceeds MAX_ELEMENTS, but their sum does -- a
    // script that repeatedly did `A = catenate(A, A)` would otherwise
    // double the size every call with no ceiling at all. The check reads
    // the operands' actual `data.length` (not a claimed shape, unlike the
    // `outer`/`matmul` tests above), so this needs two real buffers -- cheap
    // to allocate since a `Float64Array` of a given length is zero-filled
    // directly, not built up incrementally.
    const half = Math.floor(arr.MAX_ELEMENTS / 2) + 1;
    const a = arr.ndarray([half], new Float64Array(half));
    const b = arr.ndarray([half], new Float64Array(half));
    expect(() => arr.catenate(a, b)).toThrow(/exceeds/);
  });
});
