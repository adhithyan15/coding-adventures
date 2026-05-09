import {
  ADD,
  DIV,
  LIST,
  MUL,
  NEG,
  SUB,
  app,
  headName,
  int,
  sym,
  type IRNode,
} from "@coding-adventures/symbolic-ir";

export const MATRIX = "Matrix";
export const MATRIX_HEAD = sym(MATRIX);

export type MatrixRows = IRNode[][];
export type MatrixResult<T> = T;

export class MatrixError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "MatrixError";
  }
}

export function matrix(rows: readonly (readonly IRNode[])[]): IRNode {
  if (rows.length === 0) {
    throw new MatrixError("matrix() requires at least one row");
  }
  const width = rows[0].length;
  rows.forEach((row, index) => {
    if (row.length !== width) {
      throw new MatrixError(`matrix row ${index} has ${row.length} entries, expected ${width}`);
    }
  });
  return app(MATRIX_HEAD, rows.map((row) => app(LIST, row)));
}

export function isMatrix(node: IRNode): boolean {
  return node.kind === "apply" && headName(node.head) === MATRIX;
}

export function rowsOf(node: IRNode): MatrixRows {
  if (!isMatrix(node) || node.kind !== "apply") {
    throw new MatrixError(`expected a Matrix, got ${node.kind}`);
  }
  return node.args.map(rowArgs);
}

export function dimensions(node: IRNode): IRNode {
  const rows = rowsOf(node);
  return app(LIST, [int(rows.length), int(rows[0]?.length ?? 0)]);
}

export function numRows(node: IRNode): number {
  return rowsOf(node).length;
}

export function numCols(node: IRNode): number {
  const rows = rowsOf(node);
  return rows[0]?.length ?? 0;
}

export function getEntry(node: IRNode, row: number, col: number): IRNode {
  const rows = rowsOf(node);
  const nrows = rows.length;
  const ncols = rows[0]?.length ?? 0;
  if (!Number.isInteger(row) || !Number.isInteger(col) || row < 1 || row > nrows || col < 1 || col > ncols) {
    throw new MatrixError(`index (${row}, ${col}) out of range for ${nrows}x${ncols} matrix`);
  }
  return rows[row - 1][col - 1];
}

export function identityMatrix(n: number): IRNode {
  if (!Number.isInteger(n) || n <= 0) {
    throw new MatrixError("identityMatrix: n must be positive");
  }
  return matrix(Array.from({ length: n }, (_, i) =>
    Array.from({ length: n }, (_, j) => int(i === j ? 1 : 0))));
}

export function zeroMatrix(nrows: number, ncols: number): IRNode {
  if (!Number.isInteger(nrows) || !Number.isInteger(ncols) || nrows <= 0 || ncols <= 0) {
    throw new MatrixError("zeroMatrix: dims must be positive");
  }
  return matrix(Array.from({ length: nrows }, () => Array.from({ length: ncols }, () => int(0))));
}

export function transpose(node: IRNode): IRNode {
  const rows = rowsOf(node);
  const nrows = rows.length;
  const ncols = rows[0]?.length ?? 0;
  return matrix(Array.from({ length: ncols }, (_, j) =>
    Array.from({ length: nrows }, (_, i) => rows[i][j])));
}

export function addMatrices(a: IRNode, b: IRNode): IRNode {
  const aRows = rowsOf(a);
  const bRows = rowsOf(b);
  checkSameShape(aRows, bRows, "add");
  return matrix(elementwise(aRows, bRows, (x, y) => app(ADD, [x, y])));
}

export function subMatrices(a: IRNode, b: IRNode): IRNode {
  const aRows = rowsOf(a);
  const bRows = rowsOf(b);
  checkSameShape(aRows, bRows, "sub");
  return matrix(elementwise(aRows, bRows, (x, y) => app(SUB, [x, y])));
}

export function scalarMultiply(scalar: IRNode, node: IRNode): IRNode {
  return matrix(rowsOf(node).map((row) => row.map((cell) => app(MUL, [scalar, cell]))));
}

export function trace(node: IRNode): IRNode {
  const rows = rowsOf(node);
  const nrows = rows.length;
  const ncols = rows[0]?.length ?? 0;
  if (nrows !== ncols) {
    throw new MatrixError(`trace: matrix must be square, got ${nrows}x${ncols}`);
  }
  const diag = rows.map((row, i) => row[i]);
  if (diag.length === 0) return int(0);
  if (diag.length === 1) return diag[0];
  return app(ADD, diag);
}

export function dot(a: IRNode, b: IRNode): IRNode {
  const aRows = rowsOf(a);
  const bRows = rowsOf(b);
  const aCols = aRows[0]?.length ?? 0;
  const bRowCount = bRows.length;
  if (aCols !== bRowCount) {
    throw new MatrixError(`dot: cols(A)=${aCols} != rows(B)=${bRowCount}`);
  }
  const bCols = bRows[0]?.length ?? 0;
  return matrix(aRows.map((row) =>
    Array.from({ length: bCols }, (_, j) => {
      const terms = row.map((cell, k) => app(MUL, [cell, bRows[k][j]]));
      return terms.length === 1 ? terms[0] : app(ADD, terms);
    })));
}

export function determinant(node: IRNode): IRNode {
  const rows = rowsOf(node);
  const n = rows.length;
  const ncols = rows[0]?.length ?? 0;
  if (n !== ncols) {
    throw new MatrixError(`determinant: matrix must be square, got ${n}x${ncols}`);
  }
  return det(rows);
}

export function inverse(node: IRNode): IRNode {
  const rows = rowsOf(node);
  const n = rows.length;
  const ncols = rows[0]?.length ?? 0;
  if (n !== ncols) {
    throw new MatrixError(`inverse: matrix must be square, got ${n}x${ncols}`);
  }
  const determinantNode = det(rows);
  const cofactors = rows.map((_, i) =>
    rows.map((__, j) => {
      const subDet = det(minor(rows, i, j));
      return (i + j) % 2 === 0 ? subDet : app(NEG, [subDet]);
    }));
  const adjugate = cofactors.map((row, i) => row.map((_, j) => cofactors[j][i]));
  return matrix(adjugate.map((row) => row.map((cell) => app(DIV, [cell, determinantNode]))));
}

function rowArgs(row: IRNode): IRNode[] {
  if (row.kind === "apply" && headName(row.head) === LIST.name) {
    return [...row.args];
  }
  throw new MatrixError(`matrix row must be a List, got ${row.kind}`);
}

function checkSameShape(a: MatrixRows, b: MatrixRows, op: string): void {
  const ar = a.length;
  const ac = a[0]?.length ?? 0;
  const br = b.length;
  const bc = b[0]?.length ?? 0;
  if (ar !== br || ac !== bc) {
    throw new MatrixError(`${op}: shape mismatch (${ar}x${ac} vs ${br}x${bc})`);
  }
}

function elementwise(a: MatrixRows, b: MatrixRows, f: (x: IRNode, y: IRNode) => IRNode): MatrixRows {
  return a.map((row, i) => row.map((cell, j) => f(cell, b[i][j])));
}

function det(rows: MatrixRows): IRNode {
  const n = rows.length;
  if (n === 0) return int(1);
  if (n === 1) return rows[0][0];
  if (n === 2) {
    const [[a, b], [c, d]] = rows;
    return app(SUB, [app(MUL, [a, d]), app(MUL, [b, c])]);
  }
  const terms = rows[0].map((entry, j) => {
    const product = app(MUL, [entry, det(minor(rows, 0, j))]);
    return j % 2 === 0 ? product : app(NEG, [product]);
  });
  return terms.length === 1 ? terms[0] : app(ADD, terms);
}

function minor(rows: MatrixRows, skipRow: number, skipCol: number): MatrixRows {
  return rows
    .filter((_, rowIndex) => rowIndex !== skipRow)
    .map((row) => row.filter((_, colIndex) => colIndex !== skipCol));
}
