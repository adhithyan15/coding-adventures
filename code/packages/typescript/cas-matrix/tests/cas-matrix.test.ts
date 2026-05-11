import { describe, expect, it } from "vitest";
import {
  MATRIX,
  MatrixError,
  addMatrices,
  determinant,
  dimensions,
  dot,
  getEntry,
  identityMatrix,
  inverse,
  isMatrix,
  matrix,
  numCols,
  numRows,
  rowsOf,
  scalarMultiply,
  subMatrices,
  trace,
  transpose,
  zeroMatrix,
} from "../src/index";
import { ADD, LIST, MUL, SUB, app, equals, int, sym, type IRNode } from "@coding-adventures/symbolic-ir";

function irow(values: readonly number[]): IRNode[] {
  return values.map((value) => int(value));
}

describe("construction and shape", () => {
  it("builds Matrix(List(...), ...) nodes", () => {
    const m = matrix([irow([1, 2]), irow([3, 4])]);
    expect(isMatrix(m)).toBe(true);
    expect(m.kind).toBe("apply");
    if (m.kind === "apply") {
      expect(equals(m.head, sym(MATRIX))).toBe(true);
      expect(m.args.length).toBe(2);
    }
  });

  it("rejects jagged and empty matrices", () => {
    expect(() => matrix([irow([1, 2]), irow([3])])).toThrow(MatrixError);
    expect(() => matrix([])).toThrow(MatrixError);
  });

  it("reports dimensions and entries", () => {
    const m = matrix([irow([1, 2, 3]), irow([4, 5, 6])]);
    expect(equals(dimensions(m), app(LIST, [int(2), int(3)]))).toBe(true);
    expect(numRows(m)).toBe(2);
    expect(numCols(m)).toBe(3);
    expect(equals(getEntry(m, 2, 1), int(4))).toBe(true);
    expect(() => getEntry(m, 3, 1)).toThrow(MatrixError);
    expect(() => getEntry(m, 1, 4)).toThrow(MatrixError);
  });

  it("extracts cloned rows and rejects non-matrices", () => {
    const m = matrix([[sym("a"), sym("b")], [sym("c"), sym("d")]]);
    const rows = rowsOf(m);
    expect(equals(rows[1][1], sym("d"))).toBe(true);
    expect(isMatrix(int(5))).toBe(false);
    expect(isMatrix(app(ADD, [int(1)]))).toBe(false);
    expect(() => rowsOf(sym("x"))).toThrow(MatrixError);
  });
});

describe("constructors and transpose", () => {
  it("builds identity and zero matrices", () => {
    const eye = identityMatrix(3);
    expect(equals(eye, matrix([irow([1, 0, 0]), irow([0, 1, 0]), irow([0, 0, 1])]))).toBe(true);
    expect(equals(getEntry(identityMatrix(1), 1, 1), int(1))).toBe(true);
    const zero = zeroMatrix(2, 4);
    expect(numRows(zero)).toBe(2);
    expect(numCols(zero)).toBe(4);
    expect(equals(getEntry(zero, 2, 2), int(0))).toBe(true);
    expect(() => identityMatrix(0)).toThrow(MatrixError);
    expect(() => zeroMatrix(0, 2)).toThrow(MatrixError);
  });

  it("transposes square and rectangular matrices", () => {
    expect(equals(
      transpose(matrix([irow([1, 2]), irow([3, 4])])),
      matrix([irow([1, 3]), irow([2, 4])]),
    )).toBe(true);
    const rect = matrix([irow([1, 2, 3]), irow([4, 5, 6])]);
    expect(equals(transpose(rect), matrix([irow([1, 4]), irow([2, 5]), irow([3, 6])]))).toBe(true);
    expect(equals(transpose(transpose(rect)), rect)).toBe(true);
  });
});

describe("arithmetic", () => {
  it("adds and subtracts elementwise", () => {
    const a = matrix([irow([1, 2])]);
    const b = matrix([irow([3, 4])]);
    expect(equals(getEntry(addMatrices(a, b), 1, 1), app(ADD, [int(1), int(3)]))).toBe(true);
    expect(equals(getEntry(subMatrices(a, b), 1, 1), app(SUB, [int(1), int(3)]))).toBe(true);
    expect(() => addMatrices(a, matrix([irow([1])]))).toThrow(MatrixError);
  });

  it("scalar-multiplies entries", () => {
    const out = scalarMultiply(int(3), matrix([irow([1, 2])]));
    expect(numRows(out)).toBe(1);
    expect(numCols(out)).toBe(2);
    expect(equals(getEntry(out, 1, 2), app(MUL, [int(3), int(2)]))).toBe(true);
  });

  it("performs symbolic dot products", () => {
    const a = matrix([irow([1, 2])]);
    const b = matrix([irow([3]), irow([4])]);
    const c = dot(a, b);
    expect(numRows(c)).toBe(1);
    expect(numCols(c)).toBe(1);
    expect(equals(
      getEntry(c, 1, 1),
      app(ADD, [app(MUL, [int(1), int(3)]), app(MUL, [int(2), int(4)])]),
    )).toBe(true);
    expect(() => dot(a, matrix([irow([3, 4])]))).toThrow(MatrixError);
  });

  it("computes symbolic trace", () => {
    expect(equals(trace(matrix([irow([1, 2]), irow([3, 4])])), app(ADD, [int(1), int(4)]))).toBe(true);
    expect(equals(trace(matrix([[sym("a")]])), sym("a"))).toBe(true);
    expect(() => trace(matrix([irow([1, 2, 3])]))).toThrow(MatrixError);
  });
});

describe("determinant and inverse", () => {
  it("computes determinant shapes", () => {
    expect(equals(determinant(matrix([[sym("a")]])), sym("a"))).toBe(true);
    const d2 = determinant(matrix([[sym("a"), sym("b")], [sym("c"), sym("d")]]));
    expect(equals(d2, app(SUB, [app(MUL, [sym("a"), sym("d")]), app(MUL, [sym("b"), sym("c")])]))).toBe(true);
    const d3 = determinant(matrix([irow([1, 2, 3]), irow([4, 5, 6]), irow([7, 8, 9])]));
    expect(d3.kind).toBe("apply");
    if (d3.kind === "apply") {
      expect(equals(d3.head, ADD)).toBe(true);
      expect(d3.args.length).toBe(3);
    }
    expect(() => determinant(matrix([irow([1, 2, 3])]))).toThrow(MatrixError);
  });

  it("computes symbolic inverse matrix shape and entries", () => {
    const inv = inverse(matrix([[sym("a"), sym("b")], [sym("c"), sym("d")]]));
    expect(numRows(inv)).toBe(2);
    expect(numCols(inv)).toBe(2);
    const entry = getEntry(inv, 1, 1);
    expect(entry.kind).toBe("apply");
    if (entry.kind === "apply") {
      expect(equals(entry.head, sym("Div"))).toBe(true);
    }

    const inv1 = inverse(matrix([[sym("a")]]));
    expect(numRows(inv1)).toBe(1);
    expect(numCols(inv1)).toBe(1);
    expect(() => inverse(matrix([irow([1, 2, 3])]))).toThrow(MatrixError);
  });
});
