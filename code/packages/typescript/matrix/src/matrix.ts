/**
 * # Matrix — A Pure TypeScript Matrix Library
 *
 * This module provides a 2D matrix type with arithmetic, reductions,
 * element-wise math, shape manipulation, and comparison operations.
 *
 * ## Design Principles
 *
 * 1. **Immutable by default.** Every method returns a *new* Matrix instance;
 *    the original is never mutated. This makes reasoning about data flow
 *    straightforward and avoids hidden side-effects.
 *
 * 2. **No external dependencies.** Only `Math.*` from the JavaScript standard
 *    library. The package can run anywhere TypeScript runs.
 *
 * 3. **Row-major storage.** `data[i][j]` is the element at row i, column j.
 *    This matches the mathematical convention M_{i,j} and the way most
 *    people write matrices on paper.
 *
 * ## Internal Representation
 *
 * ```
 *   data: number[][]   -- 2D array, outer = rows, inner = columns
 *   rows: number       -- cached row count (data.length)
 *   cols: number       -- cached column count (data[0].length)
 * ```
 */
function assertMatrixIndex(index: number, limit: number, axis: "row" | "col"): void {
  if (!Number.isSafeInteger(index) || index < 0 || index >= limit) {
    throw new Error(`${axis} index ${String(index)} out of bounds for size ${limit}.`);
  }
}


export class Matrix {
  data: number[][];
  rows: number;
  cols: number;

  /**
   * Construct a Matrix from a scalar, a 1D array (treated as a single row),
   * or a 2D array (rows x cols).
   *
   * Examples:
   *   new Matrix(5)              -> 1x1 matrix [[5]]
   *   new Matrix([1, 2, 3])      -> 1x3 matrix [[1, 2, 3]]
   *   new Matrix([[1,2],[3,4]])   -> 2x2 matrix
   */
  constructor(data: number | number[] | number[][]) {
    if (typeof data === "number") {
      this.data = [[data]];
    } else if (Array.isArray(data) && data.length > 0 && typeof data[0] === "number") {
      this.data = [(data as number[])];
    } else if (Array.isArray(data)) {
      this.data = data as number[][];
    } else {
      this.data = [];
    }
    this.rows = this.data.length;
    this.cols = this.rows > 0 ? this.data[0].length : 0;
  }

  // ─── Factory Methods ───────────────────────────────────────────────

  /**
   * Create an rows x cols matrix filled with zeros.
   *
   * This is the workhorse factory — used internally by `dot`, `transpose`,
   * and many other methods that need a blank canvas to write into.
   */
  static zeros(rows: number, cols: number): Matrix {
    return new Matrix(Array.from({ length: rows }, () => Array(cols).fill(0.0)));
  }

  /**
   * Create an n x n identity matrix.
   *
   * The identity matrix has 1.0 on the main diagonal and 0.0 everywhere
   * else. It is the multiplicative identity for matrix dot products:
   *
   *   identity(n).dot(M) == M   (for any n x m matrix M)
   *
   * This is analogous to multiplying a number by 1 — it changes nothing.
   *
   * ```
   *   identity(3) -> [[1,0,0],
   *                    [0,1,0],
   *                    [0,0,1]]
   * ```
   */
  static identity(n: number): Matrix {
    const data = Array.from({ length: n }, (_, i) =>
      Array.from({ length: n }, (_, j) => (i === j ? 1.0 : 0.0))
    );
    return new Matrix(data);
  }

  /**
   * Create a diagonal matrix from an array of values.
   *
   * The resulting matrix is n x n where n = values.length.
   * Only the main diagonal is populated; off-diagonal entries are 0.
   *
   *   from_diagonal([2, 3]) -> [[2, 0],
   *                             [0, 3]]
   */
  static fromDiagonal(values: number[]): Matrix {
    const n = values.length;
    const data = Array.from({ length: n }, (_, i) =>
      Array.from({ length: n }, (_, j) => (i === j ? values[i] : 0.0))
    );
    return new Matrix(data);
  }

  // ─── Basic Arithmetic ──────────────────────────────────────────────

  /**
   * Element-wise addition. Accepts either a Matrix (must have same shape)
   * or a scalar (broadcast to every element).
   */
  add(other: Matrix | number): Matrix {
    if (typeof other === "number") {
      return new Matrix(this.data.map(row => row.map(val => val + other)));
    }
    if (this.rows !== other.rows || this.cols !== other.cols) throw new Error("Add dimension mismatch.");
    return new Matrix(this.data.map((row, i) => row.map((val, j) => val + other.data[i][j])));
  }

  /**
   * Element-wise subtraction. Same broadcasting rules as `add`.
   */
  subtract(other: Matrix | number): Matrix {
    if (typeof other === "number") {
      return new Matrix(this.data.map(row => row.map(val => val - other)));
    }
    if (this.rows !== other.rows || this.cols !== other.cols) throw new Error("Subtract dimension mismatch.");
    return new Matrix(this.data.map((row, i) => row.map((val, j) => val - other.data[i][j])));
  }

  /**
   * Multiply every element by a scalar. This is *not* matrix multiplication;
   * for that, see `dot`.
   */
  scale(scalar: number): Matrix {
    return new Matrix(this.data.map(row => row.map(val => val * scalar)));
  }

  /**
   * Transpose: swap rows and columns.
   *
   * If M is m x n, then M.transpose() is n x m, where
   * M^T[j][i] = M[i][j] for all i, j.
   */
  transpose(): Matrix {
    if (this.rows === 0) return new Matrix([]);
    return new Matrix(this.data[0].map((_, colIndex) => this.data.map(row => row[colIndex])));
  }

  /**
   * Matrix multiplication (dot product).
   *
   * For an m x k matrix A and a k x n matrix B, the result is m x n
   * where C[i][j] = sum over k of A[i][k] * B[k][j].
   *
   * The inner dimensions must match: A.cols === B.rows.
   */
  dot(other: Matrix): Matrix {
    if (this.cols !== other.rows) throw new Error("Dot product inner dimensions strictly mismatch.");
    const c = Matrix.zeros(this.rows, other.cols);
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < other.cols; j++) {
        for (let k = 0; k < this.cols; k++) {
          c.data[i][j] += this.data[i][k] * other.data[k][j];
        }
      }
    }
    return c;
  }

  // ─── Element Access ────────────────────────────────────────────────

  /**
   * Get the element at (row, col).
   *
   * Indices are zero-based, so get(0, 0) returns the top-left element.
   * Throws if the index is out of bounds.
   */
  get(row: number, col: number): number {
    assertMatrixIndex(row, this.rows, "row");
    assertMatrixIndex(col, this.cols, "col");
    return this.data[row][col];
  }

  /**
   * Return a *new* matrix with the element at (row, col) replaced by `value`.
   *
   * The original matrix is not modified — immutability is key.
   *
   *   M.set(0, 0, 99).get(0, 0)  -> 99
   *   M.get(0, 0)                 -> unchanged original value
   */
  set(row: number, col: number, value: number): Matrix {
    assertMatrixIndex(row, this.rows, "row");
    assertMatrixIndex(col, this.cols, "col");
    const updatedRow = [...this.data[row]];
    updatedRow.splice(col, 1, value);
    const newData = this.data.map((r, index) =>
      index === row ? updatedRow : [...r],
    );
    return new Matrix(newData);
  }

  // ─── Reductions ────────────────────────────────────────────────────

  /**
   * Sum of all elements.
   *
   * For [[1,2],[3,4]]: 1 + 2 + 3 + 4 = 10.0
   *
   * This is a "full reduction" — it collapses the entire matrix down
   * to a single scalar. Compare with sum_rows and sum_cols which
   * reduce along one axis only.
   */
  sum(): number {
    let total = 0;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        total += this.data[i][j];
      }
    }
    return total;
  }

  /**
   * Sum each row, returning an n x 1 column vector.
   *
   * For [[1,2],[3,4]]:
   *   Row 0: 1 + 2 = 3
   *   Row 1: 3 + 4 = 7
   *   Result: [[3],[7]]
   *
   * This "reduces along axis 1" (columns collapse, rows survive).
   */
  sumRows(): Matrix {
    const data = this.data.map(row => [row.reduce((a, b) => a + b, 0)]);
    return new Matrix(data);
  }

  /**
   * Sum each column, returning a 1 x m row vector.
   *
   * For [[1,2],[3,4]]:
   *   Col 0: 1 + 3 = 4
   *   Col 1: 2 + 4 = 6
   *   Result: [[4,6]]
   *
   * This "reduces along axis 0" (rows collapse, columns survive).
   */
  sumCols(): Matrix {
    const sums = Array(this.cols).fill(0);
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        sums[j] += this.data[i][j];
      }
    }
    return new Matrix([sums]);
  }

  /**
   * Arithmetic mean of all elements: sum / count.
   *
   * For [[1,2],[3,4]]: 10 / 4 = 2.5
   */
  mean(): number {
    return this.sum() / (this.rows * this.cols);
  }

  /**
   * Minimum element value (scanning row-major order).
   */
  min(): number {
    let minVal = Infinity;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        if (this.data[i][j] < minVal) minVal = this.data[i][j];
      }
    }
    return minVal;
  }

  /**
   * Maximum element value (scanning row-major order).
   */
  max(): number {
    let maxVal = -Infinity;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        if (this.data[i][j] > maxVal) maxVal = this.data[i][j];
      }
    }
    return maxVal;
  }

  /**
   * (row, col) of the minimum element.
   *
   * Returns the *first* occurrence when scanning in row-major order
   * (left to right, top to bottom). This convention is consistent
   * across all 9 language implementations.
   */
  argmin(): [number, number] {
    let minVal = Infinity;
    let minRow = 0, minCol = 0;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        if (this.data[i][j] < minVal) {
          minVal = this.data[i][j];
          minRow = i;
          minCol = j;
        }
      }
    }
    return [minRow, minCol];
  }

  /**
   * (row, col) of the maximum element.
   *
   * Same first-occurrence convention as argmin.
   *
   *   [[1,2],[3,4]].argmax() -> [1, 1]  (element 4 at row 1, col 1)
   */
  argmax(): [number, number] {
    let maxVal = -Infinity;
    let maxRow = 0, maxCol = 0;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        if (this.data[i][j] > maxVal) {
          maxVal = this.data[i][j];
          maxRow = i;
          maxCol = j;
        }
      }
    }
    return [maxRow, maxCol];
  }

  // ─── Element-wise Math ─────────────────────────────────────────────

  /**
   * Apply a function to every element, returning a new matrix.
   *
   * This is the most general element-wise operation. `sqrt`, `abs`,
   * and `pow` are all special cases of `map`:
   *
   *   M.sqrt()  is equivalent to  M.map(Math.sqrt)
   *   M.abs()   is equivalent to  M.map(Math.abs)
   *   M.pow(2)  is equivalent to  M.map(x => Math.pow(x, 2))
   */
  map(fn: (x: number) => number): Matrix {
    return new Matrix(this.data.map(row => row.map(fn)));
  }

  /**
   * Element-wise square root. Every element must be >= 0 for real results.
   */
  sqrt(): Matrix {
    return this.map(Math.sqrt);
  }

  /**
   * Element-wise absolute value.
   */
  abs(): Matrix {
    return this.map(Math.abs);
  }

  /**
   * Element-wise exponentiation: each element raised to `exp`.
   *
   *   M.pow(2)   -> squares every element
   *   M.pow(0.5) -> equivalent to M.sqrt()
   */
  pow(exp: number): Matrix {
    return this.map(x => Math.pow(x, exp));
  }

  // ─── Shape Operations ──────────────────────────────────────────────

  /**
   * Flatten into a 1 x n row vector (n = rows * cols).
   *
   * Elements are read in row-major order:
   *   [[1,2],[3,4]].flatten() -> [[1,2,3,4]]
   *
   * This is the inverse of reshape: M.flatten().reshape(M.rows, M.cols)
   * gives back the original matrix.
   */
  flatten(): Matrix {
    const flat: number[] = [];
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        flat.push(this.data[i][j]);
      }
    }
    return new Matrix([flat]);
  }

  /**
   * Reshape into a matrix with the given dimensions.
   *
   * The total number of elements must stay the same:
   *   rows * cols === this.rows * this.cols
   *
   * Elements are filled in row-major order (same order as flatten).
   *
   *   [[1,2,3,4,5,6]].reshape(2, 3) -> [[1,2,3],[4,5,6]]
   *   [[1,2,3,4,5,6]].reshape(3, 2) -> [[1,2],[3,4],[5,6]]
   */
  reshape(rows: number, cols: number): Matrix {
    if (rows * cols !== this.rows * this.cols) {
      throw new Error(`Cannot reshape ${this.rows}x${this.cols} to ${rows}x${cols}.`);
    }
    const flat = this.flatten().data[0];
    const data: number[][] = [];
    for (let i = 0; i < rows; i++) {
      data.push(flat.slice(i * cols, (i + 1) * cols));
    }
    return new Matrix(data);
  }

  /**
   * Extract row i as a 1 x cols matrix.
   *
   *   [[1,2],[3,4]].row(0) -> [[1,2]]
   *   [[1,2],[3,4]].row(1) -> [[3,4]]
   */
  row(i: number): Matrix {
    if (i < 0 || i >= this.rows) {
      throw new Error(`Row index ${i} out of bounds for ${this.rows} rows.`);
    }
    return new Matrix([[ ...this.data[i] ]]);
  }

  /**
   * Extract column j as a rows x 1 matrix.
   *
   *   [[1,2],[3,4]].col(0) -> [[1],[3]]
   *   [[1,2],[3,4]].col(1) -> [[2],[4]]
   */
  col(j: number): Matrix {
    if (j < 0 || j >= this.cols) {
      throw new Error(`Column index ${j} out of bounds for ${this.cols} cols.`);
    }
    return new Matrix(this.data.map(row => [row[j]]));
  }

  /**
   * Extract a sub-matrix from rows [r0..r1) and columns [c0..c1).
   *
   * The range is half-open (like array.slice): r1 and c1 are exclusive.
   *
   *   [[1,2,3],[4,5,6],[7,8,9]].slice(0, 2, 1, 3) -> [[2,3],[5,6]]
   */
  slice(r0: number, r1: number, c0: number, c1: number): Matrix {
    if (r0 < 0 || r1 > this.rows || c0 < 0 || c1 > this.cols || r0 >= r1 || c0 >= c1) {
      throw new Error(`Invalid slice [${r0}:${r1}, ${c0}:${c1}] for ${this.rows}x${this.cols} matrix.`);
    }
    const data: number[][] = [];
    for (let i = r0; i < r1; i++) {
      data.push(this.data[i].slice(c0, c1));
    }
    return new Matrix(data);
  }

  // ─── Equality and Comparison ───────────────────────────────────────

  /**
   * Exact element-wise equality.
   *
   * Two matrices are equal iff they have the same shape and every
   * corresponding element is exactly the same floating-point value.
   *
   * For approximate comparison (to handle floating-point rounding),
   * use `close` instead.
   */
  equals(other: Matrix): boolean {
    if (this.rows !== other.rows || this.cols !== other.cols) return false;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        if (this.data[i][j] !== other.data[i][j]) return false;
      }
    }
    return true;
  }

  /**
   * Approximate equality within a tolerance.
   *
   * Returns true iff every pair of corresponding elements satisfies:
   *   |a - b| <= tolerance
   *
   * Default tolerance is 1e-9, which handles typical floating-point
   * rounding from operations like sqrt followed by pow(2).
   *
   *   M.close(M.sqrt().pow(2), 1e-9) -> true
   */
  close(other: Matrix, tolerance: number = 1e-9): boolean {
    if (this.rows !== other.rows || this.cols !== other.cols) return false;
    for (let i = 0; i < this.rows; i++) {
      for (let j = 0; j < this.cols; j++) {
        if (Math.abs(this.data[i][j] - other.data[i][j]) > tolerance) return false;
      }
    }
    return true;
  }
}
