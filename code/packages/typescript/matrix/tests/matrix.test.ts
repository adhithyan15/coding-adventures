import { Matrix } from "../src/matrix";

// ─── Original Base Tests ─────────────────────────────────────────────

describe("Matrix Base Operations", () => {
  it("should create zeros", () => {
    const z = Matrix.zeros(2, 3);
    expect(z.rows).toBe(2);
    expect(z.cols).toBe(3);
    expect(z.data[1][2]).toBe(0.0);
  });

  it("should add and subtract", () => {
    const A = new Matrix([[1.0, 2.0], [3.0, 4.0]]);
    const B = new Matrix([[5.0, 6.0], [7.0, 8.0]]);
    const C = A.add(B);
    expect(C.data).toEqual([[6.0, 8.0], [10.0, 12.0]]);

    const D = B.subtract(A);
    expect(D.data).toEqual([[4.0, 4.0], [4.0, 4.0]]);

    // Scalar operations
    const E = A.add(2.0);
    expect(E.data).toEqual([[3.0, 4.0], [5.0, 6.0]]);
    const F = A.subtract(1.0);
    expect(F.data).toEqual([[0.0, 1.0], [2.0, 3.0]]);

    expect(() => A.add(new Matrix([[1]]))).toThrow("Add dimension mismatch.");
  });

  it("should scale", () => {
    const A = new Matrix([[1.0, 2.0], [3.0, 4.0]]);
    const C = A.scale(2.0);
    expect(C.data).toEqual([[2.0, 4.0], [6.0, 8.0]]);
  });

  it("should transpose", () => {
    const A = new Matrix([[1.0, 2.0, 3.0], [4.0, 5.0, 6.0]]);
    const C = A.transpose();
    expect(C.data).toEqual([[1.0, 4.0], [2.0, 5.0], [3.0, 6.0]]);
  });

  it("should execute dot products mathematically", () => {
    const A = new Matrix([[1.0, 2.0], [3.0, 4.0]]);
    const B = new Matrix([[5.0, 6.0], [7.0, 8.0]]);
    const C = A.dot(B);
    expect(C.data).toEqual([[19.0, 22.0], [43.0, 50.0]]);

    const D = new Matrix([1.0, 2.0, 3.0]); // 1D array natively!
    const E = new Matrix([[4.0], [5.0], [6.0]]);
    const F = D.dot(E);
    expect(F.data).toEqual([[32.0]]);

    expect(() => A.dot(E)).toThrow("Dot product inner dimensions strictly mismatch.");
  });
});

// ─── Factory Methods ─────────────────────────────────────────────────

describe("Matrix Factory Methods", () => {
  it("should create identity matrix", () => {
    const I3 = Matrix.identity(3);
    expect(I3.rows).toBe(3);
    expect(I3.cols).toBe(3);
    expect(I3.data).toEqual([
      [1, 0, 0],
      [0, 1, 0],
      [0, 0, 1],
    ]);
  });

  it("identity(n).dot(M) == M for any n x m matrix", () => {
    const M = new Matrix([[1, 2, 3], [4, 5, 6], [7, 8, 9]]);
    const I3 = Matrix.identity(3);
    const result = I3.dot(M);
    expect(result.equals(M)).toBe(true);
  });

  it("should create diagonal matrix", () => {
    const D = Matrix.fromDiagonal([2, 3]);
    expect(D.data).toEqual([[2, 0], [0, 3]]);
  });

  it("identity(1) is [[1]]", () => {
    const I1 = Matrix.identity(1);
    expect(I1.data).toEqual([[1]]);
  });

  it("fromDiagonal with single value", () => {
    const D = Matrix.fromDiagonal([5]);
    expect(D.data).toEqual([[5]]);
  });
});

// ─── Element Access ──────────────────────────────────────────────────

describe("Matrix Element Access", () => {
  const M = new Matrix([[1, 2], [3, 4]]);

  it("get(0,0) returns top-left element", () => {
    expect(M.get(0, 0)).toBe(1);
  });

  it("get(1,1) returns bottom-right element", () => {
    expect(M.get(1, 1)).toBe(4);
  });

  it("get out of bounds throws", () => {
    expect(() => M.get(2, 0)).toThrow();
    expect(() => M.get(0, 2)).toThrow();
    expect(() => M.get(-1, 0)).toThrow();
  });

  it("set returns new matrix without mutating original", () => {
    const M2 = M.set(0, 0, 99);
    expect(M2.get(0, 0)).toBe(99);
    expect(M.get(0, 0)).toBe(1); // original unchanged
  });

  it("set out of bounds throws", () => {
    expect(() => M.set(5, 0, 1)).toThrow();
  });
});

// ─── Reductions ──────────────────────────────────────────────────────

describe("Matrix Reductions", () => {
  const M = new Matrix([[1, 2], [3, 4]]);

  it("sum of [[1,2],[3,4]] is 10", () => {
    expect(M.sum()).toBe(10.0);
  });

  it("mean of [[1,2],[3,4]] is 2.5", () => {
    expect(M.mean()).toBe(2.5);
  });

  it("sum_rows of [[1,2],[3,4]] is [[3],[7]]", () => {
    const sr = M.sumRows();
    expect(sr.data).toEqual([[3], [7]]);
    expect(sr.rows).toBe(2);
    expect(sr.cols).toBe(1);
  });

  it("sum_cols of [[1,2],[3,4]] is [[4,6]]", () => {
    const sc = M.sumCols();
    expect(sc.data).toEqual([[4, 6]]);
    expect(sc.rows).toBe(1);
    expect(sc.cols).toBe(2);
  });

  it("min of [[1,2],[3,4]] is 1", () => {
    expect(M.min()).toBe(1.0);
  });

  it("max of [[1,2],[3,4]] is 4", () => {
    expect(M.max()).toBe(4.0);
  });

  it("argmin of [[1,2],[3,4]] is [0,0]", () => {
    expect(M.argmin()).toEqual([0, 0]);
  });

  it("argmax of [[1,2],[3,4]] is [1,1]", () => {
    expect(M.argmax()).toEqual([1, 1]);
  });

  it("argmin/argmax return first occurrence for ties", () => {
    const T = new Matrix([[5, 5], [5, 5]]);
    expect(T.argmin()).toEqual([0, 0]);
    expect(T.argmax()).toEqual([0, 0]);
  });

  it("reductions on larger matrix", () => {
    const L = new Matrix([[1, 2, 3], [4, 5, 6]]);
    expect(L.sum()).toBe(21);
    expect(L.mean()).toBe(3.5);
    expect(L.sumRows().data).toEqual([[6], [15]]);
    expect(L.sumCols().data).toEqual([[5, 7, 9]]);
  });
});

// ─── Element-wise Math ───────────────────────────────────────────────

describe("Matrix Element-wise Math", () => {
  it("map doubles every element", () => {
    const M = new Matrix([[1, 2], [3, 4]]);
    const doubled = M.map(x => x * 2);
    expect(doubled.data).toEqual([[2, 4], [6, 8]]);
  });

  it("sqrt of perfect squares", () => {
    const M = new Matrix([[1, 4], [9, 16]]);
    const s = M.sqrt();
    expect(s.data).toEqual([[1, 2], [3, 4]]);
  });

  it("abs of negative values", () => {
    const M = new Matrix([[-1, 2], [-3, 4]]);
    const a = M.abs();
    expect(a.data).toEqual([[1, 2], [3, 4]]);
  });

  it("pow squares elements", () => {
    const M = new Matrix([[1, 2], [3, 4]]);
    const p = M.pow(2);
    expect(p.data).toEqual([[1, 4], [9, 16]]);
  });

  it("M.close(M.sqrt().pow(2), 1e-9) is true", () => {
    const M = new Matrix([[1, 2], [3, 4]]);
    expect(M.close(M.sqrt().pow(2.0), 1e-9)).toBe(true);
  });
});

// ─── Shape Operations ────────────────────────────────────────────────

describe("Matrix Shape Operations", () => {
  const M = new Matrix([[1, 2], [3, 4]]);

  it("flatten produces row vector", () => {
    const f = M.flatten();
    expect(f.rows).toBe(1);
    expect(f.cols).toBe(4);
    expect(f.data).toEqual([[1, 2, 3, 4]]);
  });

  it("flatten then reshape roundtrip", () => {
    const roundtrip = M.flatten().reshape(M.rows, M.cols);
    expect(roundtrip.equals(M)).toBe(true);
  });

  it("reshape changes dimensions", () => {
    const flat = new Matrix([[1, 2, 3, 4, 5, 6]]);
    const reshaped = flat.reshape(2, 3);
    expect(reshaped.data).toEqual([[1, 2, 3], [4, 5, 6]]);
    expect(reshaped.rows).toBe(2);
    expect(reshaped.cols).toBe(3);
  });

  it("reshape with invalid dimensions throws", () => {
    expect(() => M.reshape(3, 3)).toThrow();
  });

  it("row extracts single row", () => {
    expect(M.row(0).data).toEqual([[1, 2]]);
    expect(M.row(1).data).toEqual([[3, 4]]);
  });

  it("row out of bounds throws", () => {
    expect(() => M.row(2)).toThrow();
    expect(() => M.row(-1)).toThrow();
  });

  it("col extracts single column", () => {
    expect(M.col(0).data).toEqual([[1], [3]]);
    expect(M.col(1).data).toEqual([[2], [4]]);
  });

  it("col out of bounds throws", () => {
    expect(() => M.col(2)).toThrow();
  });

  it("slice extracts sub-matrix", () => {
    const S = M.slice(0, 2, 0, 1);
    expect(S.data).toEqual([[1], [3]]);
  });

  it("slice on larger matrix", () => {
    const L = new Matrix([[1, 2, 3], [4, 5, 6], [7, 8, 9]]);
    const S = L.slice(0, 2, 1, 3);
    expect(S.data).toEqual([[2, 3], [5, 6]]);
  });

  it("slice with invalid bounds throws", () => {
    expect(() => M.slice(0, 3, 0, 1)).toThrow();
    expect(() => M.slice(1, 0, 0, 1)).toThrow();
  });
});

// ─── Equality and Comparison ─────────────────────────────────────────

describe("Matrix Equality", () => {
  it("equals returns true for identical matrices", () => {
    const A = new Matrix([[1, 2], [3, 4]]);
    const B = new Matrix([[1, 2], [3, 4]]);
    expect(A.equals(B)).toBe(true);
  });

  it("equals returns false for different values", () => {
    const A = new Matrix([[1, 2], [3, 4]]);
    const B = new Matrix([[1, 2], [3, 5]]);
    expect(A.equals(B)).toBe(false);
  });

  it("equals returns false for different shapes", () => {
    const A = new Matrix([[1, 2], [3, 4]]);
    const B = new Matrix([[1, 2, 3]]);
    expect(A.equals(B)).toBe(false);
  });

  it("close with tolerance handles floating point", () => {
    const A = new Matrix([[1.0000000001]]);
    const B = new Matrix([[1.0]]);
    expect(A.close(B, 1e-9)).toBe(true);
  });

  it("close returns false when outside tolerance", () => {
    const A = new Matrix([[1.1]]);
    const B = new Matrix([[1.0]]);
    expect(A.close(B, 0.01)).toBe(false);
  });

  it("close returns false for different shapes", () => {
    const A = new Matrix([[1]]);
    const B = new Matrix([[1, 2]]);
    expect(A.close(B)).toBe(false);
  });
});
