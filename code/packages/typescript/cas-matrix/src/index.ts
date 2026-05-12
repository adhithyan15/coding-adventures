import {
  ADD,
  DIV,
  LIST,
  MUL,
  NEG,
  SQRT,
  SUB,
  app,
  headName,
  int,
  rational,
  sym,
  type IRInteger,
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

export function rowReduce(node: IRNode): IRNode {
  const frows = matrixToRationals(node);
  const nrows = frows.length;
  const ncols = frows[0]?.length ?? 0;

  let pivotRow = 0;
  for (let col = 0; col < ncols && pivotRow < nrows; col += 1) {
    let pivotPos = -1;
    for (let row = pivotRow; row < nrows; row += 1) {
      if (!frows[row][col].isZero()) {
        pivotPos = row;
        break;
      }
    }
    if (pivotPos === -1) continue;

    if (pivotPos !== pivotRow) {
      [frows[pivotRow], frows[pivotPos]] = [frows[pivotPos], frows[pivotRow]];
    }

    const pivot = frows[pivotRow][col];
    frows[pivotRow] = frows[pivotRow].map((entry) => entry.div(pivot));

    for (let row = 0; row < nrows; row += 1) {
      if (row === pivotRow) continue;
      const factor = frows[row][col];
      if (factor.isZero()) continue;
      frows[row] = frows[row].map((entry, entryCol) => entry.sub(factor.mul(frows[pivotRow][entryCol])));
    }

    pivotRow += 1;
  }

  return matrix(frows.map((row) => row.map(rationalToIr)));
}

export function rank(node: IRNode): IRInteger {
  const frows = matrixToRationals(node);
  const nrows = frows.length;
  const ncols = frows[0]?.length ?? 0;

  let pivotRow = 0;
  for (let col = 0; col < ncols && pivotRow < nrows; col += 1) {
    let pivotPos = -1;
    for (let row = pivotRow; row < nrows; row += 1) {
      if (!frows[row][col].isZero()) {
        pivotPos = row;
        break;
      }
    }
    if (pivotPos === -1) continue;

    [frows[pivotRow], frows[pivotPos]] = [frows[pivotPos], frows[pivotRow]];
    const pivot = frows[pivotRow][col];
    frows[pivotRow] = frows[pivotRow].map((entry) => entry.div(pivot));

    for (let row = pivotRow + 1; row < nrows; row += 1) {
      const factor = frows[row][col];
      if (factor.isZero()) continue;
      frows[row] = frows[row].map((entry, entryCol) => entry.sub(factor.mul(frows[pivotRow][entryCol])));
    }

    pivotRow += 1;
  }

  const nonZeroRows = frows.filter((row) => row.some((entry) => !entry.isZero())).length;
  return int(nonZeroRows);
}

export function norm(node: IRNode, kind?: string): IRNode {
  const rows = rowsOf(node);
  const nrows = rows.length;
  const ncols = rows[0]?.length ?? 0;
  if (kind !== undefined && kind !== "frobenius") {
    throw new MatrixError(`norm: unknown norm kind ${JSON.stringify(kind)}; use 'frobenius' or undefined`);
  }
  if (kind === undefined && ncols !== 1 && nrows !== 1) {
    throw new MatrixError(
      `norm: Euclidean norm requires a column or row vector (got ${nrows}x${ncols}); use norm(M, 'frobenius') for matrices`,
    );
  }

  const total = matrixToRationals(node)
    .flat()
    .reduce((acc, entry) => acc.add(entry.mul(entry)), RationalValue.zero());
  return sqrtRationalToIr(total);
}

export function frobeniusNorm(node: IRNode): IRNode {
  return norm(node, "frobenius");
}

export function luDecompose(node: IRNode): IRNode {
  const n = numRows(node);
  const ncols = numCols(node);
  if (n !== ncols) {
    throw new MatrixError(`luDecompose: matrix must be square, got ${n}x${ncols}`);
  }

  const u = matrixToRationals(node).map((row) => [...row]);
  const l = identityRationals(n);
  const p = identityRationals(n);

  for (let k = 0; k < n; k += 1) {
    let bestRow = k;
    for (let row = k + 1; row < n; row += 1) {
      if (compareAbsRational(u[row][k], u[bestRow][k]) > 0) {
        bestRow = row;
      }
    }

    if (bestRow !== k) {
      [u[k], u[bestRow]] = [u[bestRow], u[k]];
      [p[k], p[bestRow]] = [p[bestRow], p[k]];
      for (let col = 0; col < k; col += 1) {
        [l[k][col], l[bestRow][col]] = [l[bestRow][col], l[k][col]];
      }
    }

    const pivot = u[k][k];
    if (pivot.isZero()) {
      throw new MatrixError(`luDecompose: singular matrix (zero pivot at column ${k})`);
    }

    for (let row = k + 1; row < n; row += 1) {
      const factor = u[row][k].div(pivot);
      l[row][k] = factor;
      for (let col = k; col < n; col += 1) {
        u[row][col] = u[row][col].sub(factor.mul(u[k][col]));
      }
    }
  }

  return app(LIST, [rationalsToMatrix(l), rationalsToMatrix(u), rationalsToMatrix(p)]);
}

export function nullspace(node: IRNode): IRNode {
  const frows = matrixToRationals(node);
  const ncols = frows[0]?.length ?? 0;
  const { pivotCols, rref } = rrefPivotInfo(frows);
  const pivotSet = new Set(pivotCols);
  const basis: IRNode[] = [];

  for (let freeCol = 0; freeCol < ncols; freeCol += 1) {
    if (pivotSet.has(freeCol)) continue;

    const vector = Array.from({ length: ncols }, () => RationalValue.zero());
    vector[freeCol] = RationalValue.one();
    pivotCols.forEach((pivotCol, pivotRow) => {
      vector[pivotCol] = rref[pivotRow][freeCol].neg();
    });
    basis.push(rationalsToMatrix(vector.map((entry) => [entry])));
  }

  return app(LIST, basis);
}

export function columnspace(node: IRNode): IRNode {
  const { pivotCols } = rrefPivotInfo(matrixToRationals(node));
  const originalRows = rowsOf(node);
  const basis = pivotCols.map((col) => matrix(originalRows.map((row) => [row[col]])));
  return app(LIST, basis);
}

export function rowspace(node: IRNode): IRNode {
  const { rref } = rrefPivotInfo(matrixToRationals(node));
  const basis = rref
    .filter((row) => row.some((entry) => !entry.isZero()))
    .map((row) => rationalsToMatrix([row]));
  return app(LIST, basis);
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

function matrixToRationals(node: IRNode): RationalValue[][] {
  return rowsOf(node).map((row) => row.map(entryToRational));
}

function entryToRational(node: IRNode): RationalValue {
  if (node.kind === "integer") return new RationalValue(node.value, 1n);
  if (node.kind === "rational") return new RationalValue(node.numer, node.denom);
  throw new MatrixError(`rowReduce/rank: symbolic entry not supported: ${node.kind}`);
}

function rationalToIr(value: RationalValue): IRNode {
  return value.denom === 1n ? int(value.numer) : rational(value.numer, value.denom);
}

function rationalsToMatrix(rows: readonly (readonly RationalValue[])[]): IRNode {
  return matrix(rows.map((row) => row.map(rationalToIr)));
}

function identityRationals(n: number): RationalValue[][] {
  return Array.from({ length: n }, (_, row) =>
    Array.from({ length: n }, (_, col) => (row === col ? RationalValue.one() : RationalValue.zero())));
}

function rrefPivotInfo(rows: readonly (readonly RationalValue[])[]): { pivotCols: number[]; rref: RationalValue[][] } {
  const rref = rows.map((row) => [...row]);
  const nrows = rref.length;
  const ncols = rref[0]?.length ?? 0;
  const pivotCols: number[] = [];
  let pivotRow = 0;

  for (let col = 0; col < ncols && pivotRow < nrows; col += 1) {
    let pivotPos = -1;
    for (let row = pivotRow; row < nrows; row += 1) {
      if (!rref[row][col].isZero()) {
        pivotPos = row;
        break;
      }
    }
    if (pivotPos === -1) continue;

    if (pivotPos !== pivotRow) {
      [rref[pivotRow], rref[pivotPos]] = [rref[pivotPos], rref[pivotRow]];
    }

    const pivot = rref[pivotRow][col];
    rref[pivotRow] = rref[pivotRow].map((entry) => entry.div(pivot));

    for (let row = 0; row < nrows; row += 1) {
      if (row === pivotRow) continue;
      const factor = rref[row][col];
      if (factor.isZero()) continue;
      rref[row] = rref[row].map((entry, entryCol) => entry.sub(factor.mul(rref[pivotRow][entryCol])));
    }

    pivotCols.push(col);
    pivotRow += 1;
  }

  return { pivotCols, rref };
}

function sqrtRationalToIr(value: RationalValue): IRNode {
  if (value.numer < 0n) return app(SQRT, [rationalToIr(value)]);
  const numerRoot = exactIntegerSqrt(value.numer);
  const denomRoot = exactIntegerSqrt(value.denom);
  if (numerRoot !== null && denomRoot !== null) {
    return rationalToIr(new RationalValue(numerRoot, denomRoot));
  }
  return app(SQRT, [rationalToIr(value)]);
}

function exactIntegerSqrt(value: bigint): bigint | null {
  const root = integerSqrt(value);
  return root * root === value ? root : null;
}

function integerSqrt(value: bigint): bigint {
  if (value < 0n) throw new RangeError("integerSqrt requires a non-negative input");
  if (value < 2n) return value;
  let low = 1n;
  let high = value;
  while (low <= high) {
    const mid = (low + high) / 2n;
    const square = mid * mid;
    if (square === value) return mid;
    if (square < value) {
      low = mid + 1n;
    } else {
      high = mid - 1n;
    }
  }
  return high;
}

function compareAbsRational(a: RationalValue, b: RationalValue): number {
  const left = abs(a.numer) * b.denom;
  const right = abs(b.numer) * a.denom;
  if (left === right) return 0;
  return left > right ? 1 : -1;
}

class RationalValue {
  readonly numer: bigint;
  readonly denom: bigint;

  constructor(numer: bigint, denom: bigint) {
    if (denom === 0n) throw new RangeError("Rational denominator cannot be zero");
    let n = numer;
    let d = denom;
    if (d < 0n) {
      n = -n;
      d = -d;
    }
    const g = gcd(abs(n), d);
    this.numer = n / g;
    this.denom = d / g;
  }

  static zero(): RationalValue {
    return new RationalValue(0n, 1n);
  }

  static one(): RationalValue {
    return new RationalValue(1n, 1n);
  }

  isZero(): boolean {
    return this.numer === 0n;
  }

  add(other: RationalValue): RationalValue {
    return new RationalValue(this.numer * other.denom + other.numer * this.denom, this.denom * other.denom);
  }

  sub(other: RationalValue): RationalValue {
    return new RationalValue(this.numer * other.denom - other.numer * this.denom, this.denom * other.denom);
  }

  neg(): RationalValue {
    return new RationalValue(-this.numer, this.denom);
  }

  mul(other: RationalValue): RationalValue {
    return new RationalValue(this.numer * other.numer, this.denom * other.denom);
  }

  div(other: RationalValue): RationalValue {
    if (other.isZero()) throw new RangeError("Rational division by zero");
    return new RationalValue(this.numer * other.denom, this.denom * other.numer);
  }
}

function gcd(a: bigint, b: bigint): bigint {
  let x = a;
  let y = b;
  while (y !== 0n) {
    const t = y;
    y = x % y;
    x = t;
  }
  return x === 0n ? 1n : x;
}

function abs(value: bigint): bigint {
  return value < 0n ? -value : value;
}
