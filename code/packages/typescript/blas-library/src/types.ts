/**
 * BLAS Data Types -- Matrix, Vector, and enumeration types.
 *
 * === What Lives Here ===
 *
 * This module defines the core data types used throughout the BLAS library:
 *
 *     1. StorageOrder  -- how matrix elements are laid out in memory
 *     2. Transpose     -- whether to logically transpose a matrix
 *     3. Side          -- which side the special matrix is on (for SYMM)
 *     4. Vector        -- a 1-D array of floats
 *     5. Matrix        -- a 2-D array of floats stored as a flat array
 *
 * === Why Flat Storage? ===
 *
 * GPUs need contiguous memory. A TypeScript `number[][]` (nested 2D array)
 * has each row allocated separately in memory. A flat `number[]` is one
 * contiguous block -- when we upload it to GPU memory, it's a single memcpy.
 *
 *     Nested (existing matrix package):
 *         data = [[1, 2, 3],
 *                 [4, 5, 6]]
 *         // Each inner array is a separate object
 *
 *     Flat (BLAS library):
 *         data = [1, 2, 3, 4, 5, 6]
 *         // One contiguous array. A[i][j] = data[i * cols + j]
 *
 * === Conversion Utilities ===
 *
 * The `fromMatrixPkg()` and `toMatrixPkg()` functions convert between the
 * two representations, so existing ML code (loss functions, gradient descent)
 * can work with BLAS results.
 */

// =========================================================================
// Enumerations -- small types that control BLAS operation behavior
// =========================================================================

/**
 * How matrix elements are laid out in memory.
 *
 * ================================================================
 * HOW MATRICES ARE STORED IN MEMORY
 * ================================================================
 *
 * A 2x3 matrix:
 *     [ 1  2  3 ]
 *     [ 4  5  6 ]
 *
 * Row-major (C convention):    [1, 2, 3, 4, 5, 6]
 *     A[i][j] = data[i * cols + j]
 *
 * Column-major (Fortran/BLAS): [1, 4, 2, 5, 3, 6]
 *     A[i][j] = data[j * rows + i]
 *
 * We default to row-major because TypeScript, C, and most ML frameworks
 * use row-major. Traditional BLAS uses column-major (Fortran heritage).
 * ================================================================
 */
export enum StorageOrder {
  ROW_MAJOR = "row_major",
  COLUMN_MAJOR = "column_major",
}

/**
 * Transpose flags for GEMM and GEMV.
 *
 * ================================================================
 * TRANSPOSE FLAGS FOR GEMM AND GEMV
 * ================================================================
 *
 * When computing C = alpha * A * B + beta * C, you often want to use A^T
 * or B^T without physically transposing the matrix. The Transpose flag
 * tells the backend to "pretend" the matrix is transposed.
 *
 * This is a classic BLAS optimization: instead of allocating a new matrix
 * and copying transposed data, you just change the access pattern. For a
 * row-major matrix with shape (M, N):
 *   - NO_TRANS: access as (M, N), stride = N
 *   - TRANS:    access as (N, M), stride = M
 * ================================================================
 */
export enum Transpose {
  NO_TRANS = "no_trans",
  TRANS = "trans",
}

/**
 * Which side the special matrix is on (for SYMM, TRMM).
 *
 * ================================================================
 * WHICH SIDE THE SPECIAL MATRIX IS ON (FOR SYMM, TRMM)
 * ================================================================
 *
 * SYMM computes C = alpha * A * B + beta * C where A is symmetric.
 * If Side.LEFT:  A is on the left  -> C = alpha * (A) * B + beta * C
 * If Side.RIGHT: A is on the right -> C = alpha * B * (A) + beta * C
 * ================================================================
 */
export enum Side {
  LEFT = "left",
  RIGHT = "right",
}

// =========================================================================
// Vector -- a 1-D array of single-precision floats
// =========================================================================

/**
 * A 1-D array of single-precision floats.
 *
 * ================================================================
 * A 1-D ARRAY OF SINGLE-PRECISION FLOATS
 * ================================================================
 *
 * This is the simplest possible vector type. It holds:
 * - data: a flat array of number values
 * - size: how many elements
 *
 * It is NOT a tensor. It is NOT a GPU buffer. It lives on the host (CPU).
 * Each backend copies it to the device when needed and copies results back.
 * This keeps the interface dead simple.
 *
 * Example:
 *     const v = new Vector([1.0, 2.0, 3.0], 3);
 *     v.data[0]  // 1.0
 *     v.size     // 3
 * ================================================================
 */
export class Vector {
  readonly data: number[];
  readonly size: number;

  constructor(data: number[], size: number) {
    /**
     * Validate that data length matches declared size.
     *
     * This catches bugs early -- if you accidentally pass the wrong
     * size, you get a clear error instead of a silent mismatch that
     * causes wrong results deep in a BLAS operation.
     */
    if (data.length !== size) {
      throw new Error(
        `Vector data has ${data.length} elements but size=${size}`
      );
    }
    this.data = data;
    this.size = size;
  }
}

// =========================================================================
// Matrix -- a 2-D array of single-precision floats (flat storage)
// =========================================================================

/**
 * A 2-D array of single-precision floats stored as a flat array.
 *
 * ================================================================
 * A 2-D ARRAY OF SINGLE-PRECISION FLOATS
 * ================================================================
 *
 * Stored as a flat array in row-major order by default:
 *
 *     new Matrix([1,2,3,4,5,6], 2, 3)
 *
 *     represents:  [ 1  2  3 ]
 *                  [ 4  5  6 ]
 *
 *     data[i * cols + j] = element at row i, column j
 *
 * The Matrix type is deliberately simple -- it's a container for moving
 * data between the caller and the BLAS backend. The backend handles device
 * memory management internally.
 * ================================================================
 */
export class Matrix {
  readonly data: number[];
  readonly rows: number;
  readonly cols: number;
  readonly order: StorageOrder;

  constructor(
    data: number[],
    rows: number,
    cols: number,
    order: StorageOrder = StorageOrder.ROW_MAJOR,
  ) {
    /**
     * Validate that data length matches rows * cols.
     *
     * A 2x3 matrix must have exactly 6 elements. No more, no less.
     * This validation catches shape mismatches before they cause
     * cryptic errors in BLAS operations.
     */
    if (data.length !== rows * cols) {
      throw new Error(
        `Matrix data has ${data.length} elements ` +
        `but shape is ${rows}x${cols} = ${rows * cols}`
      );
    }
    this.data = data;
    this.rows = rows;
    this.cols = cols;
    this.order = order;
  }
}

// =========================================================================
// Conversion utilities -- bridge to the existing matrix package
// =========================================================================

/**
 * A duck-typed interface for the existing matrix package's Matrix type.
 * Any object with `data` (2D nested array), `rows`, and `cols` qualifies.
 */
export interface MatrixPkgLike {
  data: number[][];
  rows: number;
  cols: number;
}

/**
 * Convert an existing Matrix (2D nested array) to BLAS Matrix (flat).
 *
 * The existing `matrix` package stores data as `number[][]`.
 * This function flattens it into the BLAS library's `number[]` format,
 * row by row:
 *
 *     Existing:  [[1, 2, 3], [4, 5, 6]]
 *     BLAS flat: [1, 2, 3, 4, 5, 6]
 *
 * @param m - A matrix object with `.data` (array of arrays), `.rows`, `.cols`.
 * @returns A BLAS Matrix with the same data in flat row-major order.
 */
export function fromMatrixPkg(m: MatrixPkgLike): Matrix {
  const flat: number[] = [];
  for (let i = 0; i < m.rows; i++) {
    for (let j = 0; j < m.cols; j++) {
      flat.push(m.data[i][j]);
    }
  }
  return new Matrix(flat, m.rows, m.cols);
}

/**
 * Convert a BLAS Matrix (flat) to the existing Matrix format (2D nested array).
 *
 * The reverse of `fromMatrixPkg()`. Reshapes the flat data back into nested
 * arrays:
 *
 *     BLAS flat: [1, 2, 3, 4, 5, 6]  (rows=2, cols=3)
 *     Existing:  [[1, 2, 3], [4, 5, 6]]
 *
 * @param m - A BLAS Matrix.
 * @returns An object with a 2D nested `data` array, `rows`, and `cols`.
 */
export function toMatrixPkg(m: Matrix): MatrixPkgLike {
  const data2d: number[][] = [];
  for (let i = 0; i < m.rows; i++) {
    data2d.push(m.data.slice(i * m.cols, (i + 1) * m.cols));
  }
  return { data: data2d, rows: m.rows, cols: m.cols };
}
