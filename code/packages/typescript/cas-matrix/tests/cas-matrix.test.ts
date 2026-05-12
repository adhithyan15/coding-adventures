import { describe, expect, it } from "vitest";
import {
  MATRIX,
  MatrixError,
  addMatrices,
  columnspace,
  determinant,
  dimensions,
  dot,
  frobeniusNorm,
  getEntry,
  identityMatrix,
  inverse,
  isMatrix,
  luDecompose,
  matrix,
  norm,
  numCols,
  numRows,
  nullspace,
  rank,
  rowsOf,
  rowReduce,
  rowspace,
  scalarMultiply,
  subMatrices,
  trace,
  transpose,
  zeroMatrix,
} from "../src/index";
import { ADD, LIST, MUL, SQRT, SUB, app, equals, int, numberNode, rational, sym, type IRNode } from "@coding-adventures/symbolic-ir";

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
    expect(equals(getEntry(addMatrices(a, b), 1, 1), int(4))).toBe(true);
    expect(equals(getEntry(subMatrices(a, b), 1, 1), int(-2))).toBe(true);
    expect(() => addMatrices(a, matrix([irow([1])]))).toThrow(MatrixError);
  });

  it("falls back to symbolic add and subtract when entries are symbolic", () => {
    const a = matrix([[sym("x")]]);
    const b = matrix([[int(3)]]);
    expect(equals(getEntry(addMatrices(a, b), 1, 1), app(ADD, [sym("x"), int(3)]))).toBe(true);
    expect(equals(getEntry(subMatrices(a, b), 1, 1), app(SUB, [sym("x"), int(3)]))).toBe(true);
  });

  it("scalar-multiplies entries", () => {
    const out = scalarMultiply(int(3), matrix([irow([1, 2])]));
    expect(numRows(out)).toBe(1);
    expect(numCols(out)).toBe(2);
    expect(equals(getEntry(out, 1, 2), int(6))).toBe(true);
  });

  it("falls back to symbolic scalar multiplication when entries are symbolic", () => {
    const out = scalarMultiply(int(3), matrix([[sym("x")]]));
    expect(numRows(out)).toBe(1);
    expect(numCols(out)).toBe(1);
    expect(equals(getEntry(out, 1, 1), app(MUL, [int(3), sym("x")]))).toBe(true);
  });

  it("uses backend float arithmetic for float matrices", () => {
    const out = addMatrices(
      matrix([[numberNode(1.5), numberNode(2)]]),
      matrix([[numberNode(0.5), numberNode(4)]]),
    );
    expect(equals(getEntry(out, 1, 1), numberNode(2))).toBe(true);
    expect(equals(getEntry(out, 1, 2), numberNode(6))).toBe(true);
  });

  it("keeps exact rationals in symbolic fallback", () => {
    const out = scalarMultiply(rational(1, 2), matrix([irow([2])]));
    expect(equals(getEntry(out, 1, 1), app(MUL, [rational(1, 2), int(2)]))).toBe(true);
  });

  it("performs backend dot products for integer matrices", () => {
    const a = matrix([irow([1, 2])]);
    const b = matrix([irow([3]), irow([4])]);
    const c = dot(a, b);
    expect(numRows(c)).toBe(1);
    expect(numCols(c)).toBe(1);
    expect(equals(getEntry(c, 1, 1), int(11))).toBe(true);
    expect(() => dot(a, matrix([irow([3, 4])]))).toThrow(MatrixError);
  });

  it("falls back to symbolic dot products when entries are symbolic", () => {
    const a = matrix([[sym("x"), int(2)]]);
    const b = matrix([irow([3]), irow([4])]);
    const c = dot(a, b);
    expect(equals(
      getEntry(c, 1, 1),
      app(ADD, [app(MUL, [sym("x"), int(3)]), app(MUL, [int(2), int(4)])]),
    )).toBe(true);
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

describe("rowReduce and rank", () => {
  it("leaves identity matrices unchanged and reports full rank", () => {
    const eye = identityMatrix(3);
    expect(rowReduce(eye)).toEqual(eye);
    expect(rank(eye)).toEqual(int(3));
  });

  it("keeps zero matrices at rank zero", () => {
    const zero = zeroMatrix(2, 3);
    expect(rowReduce(zero)).toEqual(zero);
    expect(rank(zero)).toEqual(int(0));
  });

  it("reduces a full-rank 2x2 matrix to identity", () => {
    const m = matrix([irow([2, 4]), irow([1, 3])]);
    expect(rowReduce(m)).toEqual(identityMatrix(2));
    expect(rank(m)).toEqual(int(2));
  });

  it("reduces a singular 3x3 matrix and reports rank two", () => {
    const m = matrix([irow([1, 2, 3]), irow([4, 5, 6]), irow([7, 8, 9])]);
    expect(rowReduce(m)).toEqual(matrix([irow([1, 0, -1]), irow([0, 1, 2]), irow([0, 0, 0])]));
    expect(rank(m)).toEqual(int(2));
  });

  it("handles rational dependent rows exactly", () => {
    const m = matrix([[rational(1, 2), int(1)], [int(1), int(2)]]);
    expect(rowReduce(m)).toEqual(matrix([[int(1), int(2)], [int(0), int(0)]]));
    expect(rank(m)).toEqual(int(1));
  });

  it("reports rank for wide and tall matrices", () => {
    const wide = matrix([irow([1, 0, 2, 1]), irow([0, 1, 3, -1])]);
    const tall = matrix([irow([1, 0]), irow([0, 1]), irow([1, 1]), irow([2, 3])]);
    expect(rank(wide)).toEqual(int(2));
    expect(rank(tall)).toEqual(int(2));
  });

  it("rejects symbolic entries for exact row reduction", () => {
    const m = matrix([[sym("a"), int(1)], [int(0), int(1)]]);
    expect(() => rowReduce(m)).toThrow(MatrixError);
    expect(() => rank(m)).toThrow(MatrixError);
  });
});

describe("norms", () => {
  it("computes exact Euclidean vector norms", () => {
    const column = matrix([irow([3]), irow([4])]);
    const row = matrix([irow([3, 4])]);
    const rationalVector = matrix([[rational(3, 5)], [rational(4, 5)]]);
    expect(norm(column)).toEqual(int(5));
    expect(norm(row)).toEqual(int(5));
    expect(norm(rationalVector)).toEqual(int(1));
  });

  it("returns Sqrt for non-perfect-square norms", () => {
    expect(norm(matrix([irow([1]), irow([1])]))).toEqual(app(SQRT, [int(2)]));
  });

  it("computes Frobenius norms and rejects matrix Euclidean norms", () => {
    expect(frobeniusNorm(matrix([irow([1, 1]), irow([1, 1])]))).toEqual(int(2));
    expect(norm(identityMatrix(3), "frobenius")).toEqual(app(SQRT, [int(3)]));
    expect(() => norm(matrix([irow([1, 2]), irow([3, 4])]))).toThrow(MatrixError);
    expect(() => norm(matrix([irow([1, 2])]), "spectral")).toThrow(MatrixError);
  });
});

describe("LU decomposition", () => {
  it("returns List(L, U, P) for matrices that require pivoting", () => {
    const result = luDecompose(matrix([irow([0, 1]), irow([1, 0])]));
    expect(result).toEqual(app(LIST, [
      identityMatrix(2),
      identityMatrix(2),
      matrix([irow([0, 1]), irow([1, 0])]),
    ]));
  });

  it("keeps exact rational multipliers", () => {
    const result = luDecompose(matrix([irow([2, 1]), irow([1, 3])]));
    expect(result.kind).toBe("apply");
    if (result.kind !== "apply") return;
    const [l, u, p] = result.args;
    expect(l).toEqual(matrix([[int(1), int(0)], [rational(1, 2), int(1)]]));
    expect(u).toEqual(matrix([[int(2), int(1)], [int(0), rational(5, 2)]]));
    expect(p).toEqual(identityMatrix(2));
  });

  it("rejects singular and non-square matrices", () => {
    expect(() => luDecompose(zeroMatrix(2, 2))).toThrow(MatrixError);
    expect(() => luDecompose(matrix([irow([1, 2, 3]), irow([4, 5, 6])]))).toThrow(MatrixError);
  });
});

describe("subspaces", () => {
  it("computes nullspace bases as a List of column-vector matrices", () => {
    const basis = nullspace(matrix([irow([1, 2, 3]), irow([4, 5, 6])]));
    expect(basis).toEqual(app(LIST, [
      matrix([irow([1]), irow([-2]), irow([1])]),
    ]));
    expect(nullspace(identityMatrix(2))).toEqual(app(LIST, []));
  });

  it("computes columnspace from original pivot columns", () => {
    const basis = columnspace(matrix([irow([1, 2]), irow([2, 4])]));
    expect(basis).toEqual(app(LIST, [
      matrix([irow([1]), irow([2])]),
    ]));
  });

  it("computes rowspace from non-zero RREF rows", () => {
    const basis = rowspace(matrix([irow([1, 2, 3]), irow([2, 4, 6])]));
    expect(basis).toEqual(app(LIST, [
      matrix([irow([1, 2, 3])]),
    ]));
  });
});
