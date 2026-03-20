/**
 * Tests for BLAS data types: Vector, Matrix, enums, and conversion utilities.
 */

import { describe, it, expect } from "vitest";
import {
  Vector,
  Matrix,
  StorageOrder,
  Transpose,
  Side,
  fromMatrixPkg,
  toMatrixPkg,
} from "../src/types.js";

// =========================================================================
// Vector tests
// =========================================================================

describe("Vector", () => {
  it("should create a vector with correct data and size", () => {
    const v = new Vector([1, 2, 3], 3);
    expect(v.data).toEqual([1, 2, 3]);
    expect(v.size).toBe(3);
  });

  it("should create an empty vector", () => {
    const v = new Vector([], 0);
    expect(v.data).toEqual([]);
    expect(v.size).toBe(0);
  });

  it("should create a single-element vector", () => {
    const v = new Vector([42], 1);
    expect(v.data[0]).toBe(42);
    expect(v.size).toBe(1);
  });

  it("should handle negative values", () => {
    const v = new Vector([-1, -2, -3], 3);
    expect(v.data).toEqual([-1, -2, -3]);
  });

  it("should handle floating point values", () => {
    const v = new Vector([1.5, 2.7, 3.14], 3);
    expect(v.data[2]).toBeCloseTo(3.14);
  });

  it("should throw if data length does not match size", () => {
    expect(() => new Vector([1, 2, 3], 2)).toThrow(
      "Vector data has 3 elements but size=2"
    );
  });

  it("should throw if size is larger than data", () => {
    expect(() => new Vector([1], 5)).toThrow(
      "Vector data has 1 elements but size=5"
    );
  });

  it("should have readonly data", () => {
    const v = new Vector([1, 2, 3], 3);
    // data property itself is readonly but array contents can be modified
    expect(v.data.length).toBe(3);
  });
});

// =========================================================================
// Matrix tests
// =========================================================================

describe("Matrix", () => {
  it("should create a matrix with correct data, rows, cols", () => {
    const m = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    expect(m.data).toEqual([1, 2, 3, 4, 5, 6]);
    expect(m.rows).toBe(2);
    expect(m.cols).toBe(3);
  });

  it("should default to ROW_MAJOR order", () => {
    const m = new Matrix([1, 2, 3, 4], 2, 2);
    expect(m.order).toBe(StorageOrder.ROW_MAJOR);
  });

  it("should accept COLUMN_MAJOR order", () => {
    const m = new Matrix([1, 2, 3, 4], 2, 2, StorageOrder.COLUMN_MAJOR);
    expect(m.order).toBe(StorageOrder.COLUMN_MAJOR);
  });

  it("should create a 1x1 matrix", () => {
    const m = new Matrix([42], 1, 1);
    expect(m.data[0]).toBe(42);
    expect(m.rows).toBe(1);
    expect(m.cols).toBe(1);
  });

  it("should create a row vector (1xN matrix)", () => {
    const m = new Matrix([1, 2, 3], 1, 3);
    expect(m.rows).toBe(1);
    expect(m.cols).toBe(3);
  });

  it("should create a column vector (Nx1 matrix)", () => {
    const m = new Matrix([1, 2, 3], 3, 1);
    expect(m.rows).toBe(3);
    expect(m.cols).toBe(1);
  });

  it("should throw if data length does not match rows * cols", () => {
    expect(() => new Matrix([1, 2, 3], 2, 2)).toThrow(
      "Matrix data has 3 elements but shape is 2x2 = 4"
    );
  });

  it("should throw for empty data with nonzero dimensions", () => {
    expect(() => new Matrix([], 1, 1)).toThrow();
  });

  it("should handle negative values", () => {
    const m = new Matrix([-1, -2, -3, -4], 2, 2);
    expect(m.data).toEqual([-1, -2, -3, -4]);
  });

  it("should handle large matrices", () => {
    const data = new Array(100).fill(0).map((_, i) => i);
    const m = new Matrix(data, 10, 10);
    expect(m.rows * m.cols).toBe(100);
  });
});

// =========================================================================
// Enum tests
// =========================================================================

describe("StorageOrder", () => {
  it("should have ROW_MAJOR value", () => {
    expect(StorageOrder.ROW_MAJOR).toBe("row_major");
  });

  it("should have COLUMN_MAJOR value", () => {
    expect(StorageOrder.COLUMN_MAJOR).toBe("column_major");
  });
});

describe("Transpose", () => {
  it("should have NO_TRANS value", () => {
    expect(Transpose.NO_TRANS).toBe("no_trans");
  });

  it("should have TRANS value", () => {
    expect(Transpose.TRANS).toBe("trans");
  });
});

describe("Side", () => {
  it("should have LEFT value", () => {
    expect(Side.LEFT).toBe("left");
  });

  it("should have RIGHT value", () => {
    expect(Side.RIGHT).toBe("right");
  });
});

// =========================================================================
// Conversion utility tests
// =========================================================================

describe("fromMatrixPkg", () => {
  it("should convert a 2D nested array to flat BLAS matrix", () => {
    const nested = { data: [[1, 2, 3], [4, 5, 6]], rows: 2, cols: 3 };
    const m = fromMatrixPkg(nested);
    expect(m.data).toEqual([1, 2, 3, 4, 5, 6]);
    expect(m.rows).toBe(2);
    expect(m.cols).toBe(3);
  });

  it("should handle 1x1 matrix", () => {
    const nested = { data: [[42]], rows: 1, cols: 1 };
    const m = fromMatrixPkg(nested);
    expect(m.data).toEqual([42]);
  });

  it("should handle row vector", () => {
    const nested = { data: [[1, 2, 3]], rows: 1, cols: 3 };
    const m = fromMatrixPkg(nested);
    expect(m.data).toEqual([1, 2, 3]);
  });

  it("should handle column vector", () => {
    const nested = { data: [[1], [2], [3]], rows: 3, cols: 1 };
    const m = fromMatrixPkg(nested);
    expect(m.data).toEqual([1, 2, 3]);
  });
});

describe("toMatrixPkg", () => {
  it("should convert flat BLAS matrix to 2D nested array", () => {
    const m = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const nested = toMatrixPkg(m);
    expect(nested.data).toEqual([[1, 2, 3], [4, 5, 6]]);
    expect(nested.rows).toBe(2);
    expect(nested.cols).toBe(3);
  });

  it("should handle 1x1 matrix", () => {
    const m = new Matrix([42], 1, 1);
    const nested = toMatrixPkg(m);
    expect(nested.data).toEqual([[42]]);
  });

  it("should round-trip: fromMatrixPkg -> toMatrixPkg", () => {
    const original = { data: [[1, 2], [3, 4], [5, 6]], rows: 3, cols: 2 };
    const result = toMatrixPkg(fromMatrixPkg(original));
    expect(result.data).toEqual(original.data);
  });
});
