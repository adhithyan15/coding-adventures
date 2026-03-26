/**
 * CpuBlas -- pure TypeScript reference implementation of BLAS.
 *
 * === Why a CPU Backend? ===
 *
 * The CPU backend serves two critical purposes:
 *
 * 1. **Universal fallback** -- it works everywhere, on any machine, with no
 *    GPU drivers or hardware requirements. If everything else fails, CPU works.
 *
 * 2. **Reference implementation** -- every other backend (CUDA, Metal, etc.) is
 *    tested against the CPU backend's results. If CudaBlas and CpuBlas disagree,
 *    the bug is in CudaBlas.
 *
 * === How It Works ===
 *
 * Every BLAS operation is implemented with explicit loops. No native extensions,
 * no tricks -- just `for` loops and arithmetic. This makes every operation
 * completely transparent:
 *
 *     SAXPY:  for (let i = 0; i < n; i++) result[i] = alpha * x[i] + y[i]
 *     GEMM:   for i, for j, for k: C[i][j] += A[i][k] * B[k][j]
 *     DOT:    sum(x[i] * y[i])
 *
 * === Performance ===
 *
 * The CPU backend is SLOW. O(n^3) for GEMM with JS loop overhead on every
 * element. A 1000x1000 matrix multiply takes seconds. But that's fine -- the
 * CPU backend optimizes for **clarity**, not speed. The GPU backends optimize
 * for speed.
 *
 * === ML Extensions ===
 *
 * The CPU backend implements ALL ML extensions (activation functions, softmax,
 * layer normalization, batch normalization, conv2d, attention). These use
 * Math.exp, Math.sqrt, Math.tanh, etc.
 */

import { Matrix, Side, Transpose, Vector } from "../types.js";
import type { MlBlasBackend } from "../protocol.js";

// =========================================================================
// Helper: access a matrix element respecting transpose
// =========================================================================

/**
 * Access matrix element, respecting the transpose flag.
 *
 * ================================================================
 * VIRTUAL TRANSPOSE -- NO COPY NEEDED
 * ================================================================
 *
 * Instead of physically transposing a matrix (allocating new memory
 * and rearranging elements), we just swap the row/col indices:
 *
 *     NO_TRANS: A[row][col] = data[row * cols + col]
 *     TRANS:    A[row][col] = data[col * cols + row]
 *               (swap row and col, keep the original cols stride)
 *
 * This is how real BLAS libraries handle transpose -- the data stays
 * in place, only the access pattern changes.
 * ================================================================
 */
function getElement(m: Matrix, row: number, col: number, trans: Transpose): number {
  if (trans === Transpose.TRANS) {
    // Transposed: logical (row, col) maps to physical (col, row)
    return m.data[col * m.cols + row];
  }
  // Not transposed: direct access
  return m.data[row * m.cols + col];
}

/**
 * Get the effective (rows, cols) after applying the transpose flag.
 *
 * A 2x3 matrix transposed becomes 3x2:
 *     NO_TRANS: (2, 3) -> (2, 3)
 *     TRANS:    (2, 3) -> (3, 2)
 */
function effectiveShape(m: Matrix, trans: Transpose): [number, number] {
  if (trans === Transpose.TRANS) {
    return [m.cols, m.rows];
  }
  return [m.rows, m.cols];
}

/**
 * Pure TypeScript BLAS implementation -- the reference backend.
 *
 * ================================================================
 * CPU BLAS -- THE REFERENCE IMPLEMENTATION
 * ================================================================
 *
 * This class implements both BlasBackend and MlBlasBackend interfaces
 * using nothing but TypeScript loops and the `Math` standard library.
 *
 * Every other backend's correctness is measured against this one.
 * If CudaBlas.sgemm() and CpuBlas.sgemm() disagree on the result,
 * the bug is in CudaBlas, not CpuBlas.
 *
 * Usage:
 *     const blas = new CpuBlas();
 *     const result = blas.saxpy(2.0, x, y);
 *     const result = blas.sgemm(NO_TRANS, NO_TRANS, 1.0, A, B, 0.0, C);
 * ================================================================
 */
export class CpuBlas implements MlBlasBackend {
  // =================================================================
  // Identity properties
  // =================================================================

  /** Backend identifier. */
  get name(): string {
    return "cpu";
  }

  /** Human-readable device name. */
  get deviceName(): string {
    return "CPU (pure TypeScript)";
  }

  // =================================================================
  // LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
  // =================================================================

  /**
   * SAXPY: result = alpha * x + y
   *
   * ================================================================
   * SAXPY -- THE HELLO WORLD OF BLAS
   * ================================================================
   *
   * S = Single precision, A = Alpha, X = vector X, P = Plus, Y = vector Y
   *
   * This is the simplest BLAS operation and our running example since
   * Layer 11 (logic gates). Each element:
   *
   *     result[i] = alpha * x[i] + y[i]
   *
   * Time complexity: O(n) -- one pass through both vectors.
   * ================================================================
   */
  saxpy(alpha: number, x: Vector, y: Vector): Vector {
    if (x.size !== y.size) {
      throw new Error(
        `SAXPY dimension mismatch: x.size=${x.size} != y.size=${y.size}`
      );
    }
    const result: number[] = new Array(x.size);
    for (let i = 0; i < x.size; i++) {
      result[i] = alpha * x.data[i] + y.data[i];
    }
    return new Vector(result, x.size);
  }

  /**
   * DOT product: result = sum(x[i] * y[i])
   *
   * ================================================================
   * DOT PRODUCT -- FOUNDATION OF SIMILARITY
   * ================================================================
   *
   * The dot product measures how "aligned" two vectors are:
   * - Parallel vectors: large positive dot product
   * - Perpendicular vectors: dot product = 0
   * - Anti-parallel: large negative dot product
   *
   * It's also the building block of matrix multiply (GEMM is
   * just a grid of dot products).
   *
   * Time complexity: O(n)
   * ================================================================
   */
  sdot(x: Vector, y: Vector): number {
    if (x.size !== y.size) {
      throw new Error(
        `DOT dimension mismatch: x.size=${x.size} != y.size=${y.size}`
      );
    }
    let sum = 0;
    for (let i = 0; i < x.size; i++) {
      sum += x.data[i] * y.data[i];
    }
    return sum;
  }

  /**
   * Euclidean norm: result = sqrt(sum(x[i]^2))
   *
   * ================================================================
   * EUCLIDEAN NORM (L2 NORM)
   * ================================================================
   *
   * The "length" of a vector in Euclidean space. Used for:
   * - Normalizing vectors (dividing by the norm to get unit vectors)
   * - Convergence checks (is the gradient small enough?)
   * - Regularization (keeping weights small)
   *
   * Numerically: sqrt(x[0]^2 + x[1]^2 + ... + x[n-1]^2)
   *
   * Time complexity: O(n)
   * ================================================================
   */
  snrm2(x: Vector): number {
    let sum = 0;
    for (let i = 0; i < x.size; i++) {
      sum += x.data[i] * x.data[i];
    }
    return Math.sqrt(sum);
  }

  /**
   * Scale: result = alpha * x
   *
   * Multiply every element by the scalar alpha.
   * Time complexity: O(n)
   */
  sscal(alpha: number, x: Vector): Vector {
    const result: number[] = new Array(x.size);
    for (let i = 0; i < x.size; i++) {
      result[i] = alpha * x.data[i];
    }
    return new Vector(result, x.size);
  }

  /**
   * Absolute sum (L1 norm): result = sum(|x[i]|)
   *
   * Also called the Manhattan distance or taxicab norm. Used in
   * L1 regularization (LASSO) which encourages sparsity.
   *
   * Time complexity: O(n)
   */
  sasum(x: Vector): number {
    let sum = 0;
    for (let i = 0; i < x.size; i++) {
      sum += Math.abs(x.data[i]);
    }
    return sum;
  }

  /**
   * Index of maximum absolute value: argmax(|x[i]|)
   *
   * Returns the 0-based index of the element with the largest
   * absolute value. Used in partial pivoting for LU decomposition
   * to improve numerical stability.
   *
   * Time complexity: O(n)
   */
  isamax(x: Vector): number {
    if (x.size === 0) {
      return 0;
    }
    let maxIdx = 0;
    let maxVal = Math.abs(x.data[0]);
    for (let i = 1; i < x.size; i++) {
      const val = Math.abs(x.data[i]);
      if (val > maxVal) {
        maxVal = val;
        maxIdx = i;
      }
    }
    return maxIdx;
  }

  /**
   * Copy: result = x (deep copy)
   *
   * Creates a completely independent copy. Modifying the result
   * does not affect the original.
   *
   * Time complexity: O(n)
   */
  scopy(x: Vector): Vector {
    return new Vector([...x.data], x.size);
  }

  /**
   * Swap: exchange the contents of x and y.
   *
   * Returns [new_x, new_y] where new_x has y's data and new_y
   * has x's data. The originals are not modified.
   *
   * Time complexity: O(n)
   */
  sswap(x: Vector, y: Vector): [Vector, Vector] {
    if (x.size !== y.size) {
      throw new Error(
        `SWAP dimension mismatch: x.size=${x.size} != y.size=${y.size}`
      );
    }
    return [
      new Vector([...y.data], y.size),
      new Vector([...x.data], x.size),
    ];
  }

  // =================================================================
  // LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
  // =================================================================

  /**
   * General Matrix-Vector multiply: y = alpha * op(A) * x + beta * y
   *
   * ================================================================
   * GEMV -- MATRIX TIMES VECTOR
   * ================================================================
   *
   * op(A) is the matrix A, optionally transposed:
   *     NO_TRANS: op(A) = A        (M x N)
   *     TRANS:    op(A) = A^T      (N x M)
   *
   * After applying the transpose:
   *     op(A) has shape (m x n)
   *     x must have size n
   *     y must have size m
   *     result has size m
   *
   * Each element of the result:
   *     result[i] = alpha * sum(op(A)[i][k] * x[k], k=0..n-1) + beta * y[i]
   *
   * Time complexity: O(M * N)
   * ================================================================
   */
  sgemv(
    trans: Transpose,
    alpha: number,
    a: Matrix,
    x: Vector,
    beta: number,
    y: Vector,
  ): Vector {
    const [m, n] = effectiveShape(a, trans);

    if (x.size !== n) {
      throw new Error(
        `GEMV dimension mismatch: op(A) is ${m}x${n} but x.size=${x.size}`
      );
    }
    if (y.size !== m) {
      throw new Error(
        `GEMV dimension mismatch: op(A) is ${m}x${n} but y.size=${y.size}`
      );
    }

    const result: number[] = new Array(m);
    for (let i = 0; i < m; i++) {
      let s = 0;
      for (let k = 0; k < n; k++) {
        s += getElement(a, i, k, trans) * x.data[k];
      }
      result[i] = alpha * s + beta * y.data[i];
    }

    return new Vector(result, m);
  }

  /**
   * Outer product (rank-1 update): A = alpha * x * y^T + A
   *
   * ================================================================
   * GER -- OUTER PRODUCT
   * ================================================================
   *
   * The outer product of two vectors creates a matrix:
   *
   *     x = [a, b]     y = [c, d, e]
   *
   *     x * y^T = [ a*c  a*d  a*e ]
   *               [ b*c  b*d  b*e ]
   *
   * Then we scale by alpha and add to the existing matrix A.
   * Each element: result[i][j] = alpha * x[i] * y[j] + A[i][j]
   *
   * Time complexity: O(M * N)
   * ================================================================
   */
  sger(alpha: number, x: Vector, y: Vector, a: Matrix): Matrix {
    if (a.rows !== x.size) {
      throw new Error(
        `GER dimension mismatch: A.rows=${a.rows} != x.size=${x.size}`
      );
    }
    if (a.cols !== y.size) {
      throw new Error(
        `GER dimension mismatch: A.cols=${a.cols} != y.size=${y.size}`
      );
    }

    const result = [...a.data]; // copy
    for (let i = 0; i < a.rows; i++) {
      for (let j = 0; j < a.cols; j++) {
        result[i * a.cols + j] += alpha * x.data[i] * y.data[j];
      }
    }

    return new Matrix(result, a.rows, a.cols, a.order);
  }

  // =================================================================
  // LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
  // =================================================================

  /**
   * General Matrix Multiply: C = alpha * op(A) * op(B) + beta * C
   *
   * ================================================================
   * GEMM -- THE MOST IMPORTANT FUNCTION IN ALL OF COMPUTING
   * ================================================================
   *
   * This is the function that NVIDIA employs entire teams to optimize.
   * 70-90% of ML training time is spent here.
   *
   * C = alpha * op(A) * op(B) + beta * C
   *
   * where:
   *     op(A) has shape (M x K)
   *     op(B) has shape (K x N)
   *     C     has shape (M x N)
   *
   * The triple nested loop:
   *     for i in range(M):          // row of C
   *         for j in range(N):      // col of C
   *             sum = 0.0
   *             for k in range(K):  // shared dimension
   *                 sum += op(A)[i][k] * op(B)[k][j]
   *             C[i][j] = alpha * sum + beta * C[i][j]
   *
   * Common special cases:
   *     C = A * B        -> alpha=1, beta=0
   *     C = A^T * B      -> transA=TRANS, alpha=1, beta=0
   *     C += A * B       -> alpha=1, beta=1
   *     C = 2*A*B + 3*C  -> alpha=2, beta=3
   *
   * Time complexity: O(M * N * K)
   * ================================================================
   */
  sgemm(
    transA: Transpose,
    transB: Transpose,
    alpha: number,
    a: Matrix,
    b: Matrix,
    beta: number,
    c: Matrix,
  ): Matrix {
    // Determine effective shapes after transpose
    const [m, kA] = effectiveShape(a, transA);
    const [kB, n] = effectiveShape(b, transB);

    // The inner dimensions must match
    if (kA !== kB) {
      throw new Error(
        `GEMM dimension mismatch: op(A) is ${m}x${kA}, ` +
        `op(B) is ${kB}x${n}. Inner dimensions ${kA} != ${kB}`
      );
    }
    const k = kA;

    // C must have shape (M x N)
    if (c.rows !== m || c.cols !== n) {
      throw new Error(
        `GEMM dimension mismatch: result should be ${m}x${n} ` +
        `but C is ${c.rows}x${c.cols}`
      );
    }

    // The triple nested loop -- the heart of linear algebra
    const result: number[] = new Array(m * n);
    for (let i = 0; i < m; i++) {
      for (let j = 0; j < n; j++) {
        let s = 0;
        for (let kk = 0; kk < k; kk++) {
          s += getElement(a, i, kk, transA) * getElement(b, kk, j, transB);
        }
        result[i * n + j] = alpha * s + beta * c.data[i * c.cols + j];
      }
    }

    return new Matrix(result, m, n, c.order);
  }

  /**
   * Symmetric Matrix Multiply.
   *
   * ================================================================
   * SYMM -- SYMMETRIC MATRIX MULTIPLY
   * ================================================================
   *
   * Like GEMM, but exploits the fact that A is symmetric (A = A^T).
   * The backend only needs to read half of A.
   *
   * LEFT:  C = alpha * A * B + beta * C
   * RIGHT: C = alpha * B * A + beta * C
   *
   * A must be square (rows == cols).
   * ================================================================
   */
  ssymm(
    side: Side,
    alpha: number,
    a: Matrix,
    b: Matrix,
    beta: number,
    c: Matrix,
  ): Matrix {
    if (a.rows !== a.cols) {
      throw new Error(`SSYMM: A must be square but is ${a.rows}x${a.cols}`);
    }

    let m: number;
    let n: number;

    if (side === Side.LEFT) {
      // C = alpha * A * B + beta * C
      // A is (M x M), B is (M x N), C is (M x N)
      m = a.rows;
      n = b.cols;
      if (b.rows !== m) {
        throw new Error(`SSYMM LEFT: A is ${m}x${m} but B.rows=${b.rows}`);
      }
    } else {
      // C = alpha * B * A + beta * C
      // B is (M x N), A is (N x N), C is (M x N)
      m = b.rows;
      n = a.rows;
      if (b.cols !== n) {
        throw new Error(`SSYMM RIGHT: A is ${n}x${n} but B.cols=${b.cols}`);
      }
    }

    if (c.rows !== m || c.cols !== n) {
      throw new Error(`SSYMM: C should be ${m}x${n} but is ${c.rows}x${c.cols}`);
    }

    // Use sgemm with NO_TRANS for both -- A is symmetric so A = A^T
    if (side === Side.LEFT) {
      return this.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, alpha, a, b, beta, c
      );
    } else {
      return this.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, alpha, b, a, beta, c
      );
    }
  }

  /**
   * Batched GEMM: multiple independent GEMMs.
   *
   * ================================================================
   * BATCHED GEMM -- MANY MATRIX MULTIPLIES AT ONCE
   * ================================================================
   *
   * Used for multi-head attention (each head is a separate GEMM),
   * batched inference (each sample is a separate GEMM), and more.
   *
   * On a GPU, all GEMMs can run in parallel. On CPU, we just loop.
   * ================================================================
   */
  sgemmBatched(
    transA: Transpose,
    transB: Transpose,
    alpha: number,
    aList: Matrix[],
    bList: Matrix[],
    beta: number,
    cList: Matrix[],
  ): Matrix[] {
    if (aList.length !== bList.length || bList.length !== cList.length) {
      throw new Error(
        `Batched GEMM: batch sizes don't match: ` +
        `A=${aList.length}, B=${bList.length}, C=${cList.length}`
      );
    }
    return aList.map((a, i) =>
      this.sgemm(transA, transB, alpha, a, bList[i], beta, cList[i])
    );
  }

  // =================================================================
  // ML EXTENSIONS: Activation Functions
  // =================================================================

  /**
   * ReLU activation: max(0, x)
   *
   * ================================================================
   * RELU -- RECTIFIED LINEAR UNIT
   * ================================================================
   *
   * The most common activation function in deep learning:
   *     relu(x) = max(0, x)
   *
   * Truth table for a single element:
   *     x < 0  -> 0.0    (negative inputs are zeroed)
   *     x >= 0 -> x      (positive inputs pass through)
   *
   * ReLU is popular because:
   * 1. It's extremely fast to compute (just a comparison)
   * 2. It doesn't saturate for positive values (no vanishing gradient)
   * 3. It produces sparse activations (many zeros)
   * ================================================================
   */
  relu(x: Matrix): Matrix {
    const result = x.data.map((v) => Math.max(0, v));
    return new Matrix(result, x.rows, x.cols, x.order);
  }

  /**
   * GELU activation: x * Phi(x) where Phi is the CDF of N(0,1).
   *
   * ================================================================
   * GELU -- GAUSSIAN ERROR LINEAR UNIT
   * ================================================================
   *
   * Used in GPT, BERT, and modern Transformers. Unlike ReLU which
   * has a hard cutoff at 0, GELU smoothly transitions:
   *
   *     gelu(x) = x * 0.5 * (1 + tanh(sqrt(2/pi) * (x + 0.044715 * x^3)))
   *
   * This approximation (from Hendrycks & Gimpel, 2016) is what
   * PyTorch and TensorFlow use.
   * ================================================================
   */
  gelu(x: Matrix): Matrix {
    const sqrt2OverPi = Math.sqrt(2.0 / Math.PI);
    const result = x.data.map((v) => {
      const inner = sqrt2OverPi * (v + 0.044715 * v * v * v);
      return 0.5 * v * (1.0 + Math.tanh(inner));
    });
    return new Matrix(result, x.rows, x.cols, x.order);
  }

  /**
   * Sigmoid activation: 1 / (1 + exp(-x))
   *
   * ================================================================
   * SIGMOID -- THE LOGISTIC FUNCTION
   * ================================================================
   *
   * Maps any real number to the range (0, 1):
   *     sigmoid(-inf) -> 0
   *     sigmoid(0)    -> 0.5
   *     sigmoid(+inf) -> 1
   *
   * Numerically stable implementation: for large negative x,
   * exp(-x) overflows. We use: if x >= 0, compute as 1/(1+exp(-x));
   * if x < 0, compute as exp(x)/(1+exp(x)).
   * ================================================================
   */
  sigmoid(x: Matrix): Matrix {
    const result = x.data.map((v) => {
      if (v >= 0) {
        return 1.0 / (1.0 + Math.exp(-v));
      } else {
        const ev = Math.exp(v);
        return ev / (1.0 + ev);
      }
    });
    return new Matrix(result, x.rows, x.cols, x.order);
  }

  /**
   * Tanh activation: tanh(x)
   *
   * Maps any real number to (-1, 1). Used in RNNs and as an
   * activation function. Related to sigmoid: tanh(x) = 2*sigmoid(2x) - 1.
   */
  tanhActivation(x: Matrix): Matrix {
    const result = x.data.map((v) => Math.tanh(v));
    return new Matrix(result, x.rows, x.cols, x.order);
  }

  // =================================================================
  // ML EXTENSIONS: Softmax
  // =================================================================

  /**
   * Numerically stable softmax along an axis.
   *
   * ================================================================
   * SOFTMAX -- PROBABILITY DISTRIBUTION OVER A VECTOR
   * ================================================================
   *
   * Converts a vector of real numbers into a probability distribution:
   *     softmax(x)[i] = exp(x[i]) / sum(exp(x[j]))
   *
   * The NAIVE implementation overflows for large x because exp(710) is
   * infinity in float64. The STABLE version subtracts the max first:
   *     softmax(x)[i] = exp(x[i] - max(x)) / sum(exp(x[j] - max(x)))
   *
   * This works because softmax is invariant to constant shifts:
   *     softmax(x + c) = softmax(x)  for any constant c
   *
   * axis=-1 means "along the last dimension" (columns for 2D).
   * For a 2D matrix, this means each ROW becomes a probability
   * distribution that sums to 1.0.
   * ================================================================
   */
  softmax(x: Matrix, axis: number = -1): Matrix {
    // Normalize axis
    const actualAxis = axis === -1 ? 1 : axis;

    if (actualAxis === 1) {
      // Softmax along each row
      const result: number[] = [];
      for (let i = 0; i < x.rows; i++) {
        const row = x.data.slice(i * x.cols, (i + 1) * x.cols);
        const maxVal = Math.max(...row);
        const exps = row.map((v) => Math.exp(v - maxVal));
        const total = exps.reduce((a, b) => a + b, 0);
        for (const e of exps) {
          result.push(e / total);
        }
      }
      return new Matrix(result, x.rows, x.cols, x.order);
    } else {
      // axis === 0: softmax along each column
      const result = [...x.data];
      for (let j = 0; j < x.cols; j++) {
        const col: number[] = [];
        for (let i = 0; i < x.rows; i++) {
          col.push(x.data[i * x.cols + j]);
        }
        const maxVal = Math.max(...col);
        const exps = col.map((v) => Math.exp(v - maxVal));
        const total = exps.reduce((a, b) => a + b, 0);
        for (let i = 0; i < x.rows; i++) {
          result[i * x.cols + j] = exps[i] / total;
        }
      }
      return new Matrix(result, x.rows, x.cols, x.order);
    }
  }

  // =================================================================
  // ML EXTENSIONS: Normalization
  // =================================================================

  /**
   * Layer Normalization (Ba et al., 2016).
   *
   * ================================================================
   * LAYER NORM -- NORMALIZE EACH SAMPLE INDEPENDENTLY
   * ================================================================
   *
   * For each row (sample) in the matrix:
   *     1. Compute mean: mu = sum(x) / n
   *     2. Compute variance: var = sum((x - mu)^2) / n
   *     3. Normalize: x_hat = (x - mu) / sqrt(var + eps)
   *     4. Scale and shift: result = gamma * x_hat + beta
   *
   * gamma and beta are learnable parameters (one per feature).
   *
   * Used in: Transformers, GPT, BERT (before every attention/FFN block)
   * ================================================================
   */
  layerNorm(
    x: Matrix,
    gamma: Vector,
    beta: Vector,
    eps: number = 1e-5,
  ): Matrix {
    if (gamma.size !== x.cols) {
      throw new Error(`LayerNorm: gamma.size=${gamma.size} != x.cols=${x.cols}`);
    }
    if (beta.size !== x.cols) {
      throw new Error(`LayerNorm: beta.size=${beta.size} != x.cols=${x.cols}`);
    }

    const result: number[] = new Array(x.rows * x.cols);
    const n = x.cols;

    for (let i = 0; i < x.rows; i++) {
      const row = x.data.slice(i * n, (i + 1) * n);

      // Step 1: mean
      let mean = 0;
      for (const v of row) mean += v;
      mean /= n;

      // Step 2: variance
      let variance = 0;
      for (const v of row) variance += (v - mean) ** 2;
      variance /= n;

      // Step 3 & 4: normalize, scale, shift
      const invStd = 1.0 / Math.sqrt(variance + eps);
      for (let j = 0; j < n; j++) {
        const xHat = (row[j] - mean) * invStd;
        result[i * n + j] = gamma.data[j] * xHat + beta.data[j];
      }
    }

    return new Matrix(result, x.rows, x.cols, x.order);
  }

  /**
   * Batch Normalization (Ioffe & Szegedy, 2015).
   *
   * ================================================================
   * BATCH NORM -- NORMALIZE EACH FEATURE ACROSS THE BATCH
   * ================================================================
   *
   * Unlike layer norm (which normalizes each sample), batch norm
   * normalizes each FEATURE across all samples in the batch:
   *
   * Training mode:
   *     mean_j = sum(x[i][j] for i in batch) / batch_size
   *     var_j  = sum((x[i][j] - mean_j)^2 for i in batch) / batch_size
   *     x_hat[i][j] = (x[i][j] - mean_j) / sqrt(var_j + eps)
   *     result[i][j] = gamma[j] * x_hat[i][j] + beta[j]
   *
   * Inference mode:
   *     Uses runningMean and runningVar instead of batch statistics.
   *
   * Used in: CNNs, ResNets, most non-Transformer architectures
   * ================================================================
   */
  batchNorm(
    x: Matrix,
    gamma: Vector,
    beta: Vector,
    runningMean: Vector,
    runningVar: Vector,
    eps: number = 1e-5,
    training: boolean = false,
  ): Matrix {
    if (gamma.size !== x.cols) {
      throw new Error(`BatchNorm: gamma.size=${gamma.size} != x.cols=${x.cols}`);
    }
    if (beta.size !== x.cols) {
      throw new Error(`BatchNorm: beta.size=${beta.size} != x.cols=${x.cols}`);
    }

    const result: number[] = new Array(x.rows * x.cols);
    const batchSize = x.rows;
    const nFeatures = x.cols;

    if (training) {
      // Compute batch statistics
      for (let j = 0; j < nFeatures; j++) {
        const col: number[] = [];
        for (let i = 0; i < batchSize; i++) {
          col.push(x.data[i * nFeatures + j]);
        }
        let mean = 0;
        for (const v of col) mean += v;
        mean /= batchSize;

        let variance = 0;
        for (const v of col) variance += (v - mean) ** 2;
        variance /= batchSize;

        const invStd = 1.0 / Math.sqrt(variance + eps);
        for (let i = 0; i < batchSize; i++) {
          const xHat = (col[i] - mean) * invStd;
          result[i * nFeatures + j] = gamma.data[j] * xHat + beta.data[j];
        }
      }
    } else {
      // Use running statistics
      for (let j = 0; j < nFeatures; j++) {
        const mean = runningMean.data[j];
        const variance = runningVar.data[j];
        const invStd = 1.0 / Math.sqrt(variance + eps);
        for (let i = 0; i < batchSize; i++) {
          const xHat = (x.data[i * nFeatures + j] - mean) * invStd;
          result[i * nFeatures + j] = gamma.data[j] * xHat + beta.data[j];
        }
      }
    }

    return new Matrix(result, x.rows, x.cols, x.order);
  }

  // =================================================================
  // ML EXTENSIONS: Convolution
  // =================================================================

  /**
   * 2D Convolution via im2col + GEMM.
   *
   * ================================================================
   * CONV2D -- SIMPLIFIED 2D CONVOLUTION
   * ================================================================
   *
   * We treat inputMat as a 2D spatial feature map (height x width)
   * and weight as a 2D filter (kH x kW). This is a simplified
   * single-channel convolution for demonstration.
   *
   * Steps:
   * 1. Apply padding if needed
   * 2. Extract all patches (im2col style) into columns
   * 3. Flatten weight into a row vector
   * 4. Compute dot product of weight with each patch
   *
   * The output has shape:
   *     outH = (height + 2*padding - kH) / stride + 1
   *     outW = (width + 2*padding - kW) / stride + 1
   * ================================================================
   */
  conv2d(
    inputMat: Matrix,
    weight: Matrix,
    bias: Vector | null = null,
    stride: number = 1,
    padding: number = 0,
  ): Matrix {
    const hIn = inputMat.rows;
    const wIn = inputMat.cols;
    const kH = weight.rows;
    const kW = weight.cols;

    // Output dimensions
    const outH = Math.floor((hIn + 2 * padding - kH) / stride) + 1;
    const outW = Math.floor((wIn + 2 * padding - kW) / stride) + 1;

    if (outH <= 0 || outW <= 0) {
      throw new Error(
        `Conv2d: output dimensions are non-positive: ${outH}x${outW}`
      );
    }

    // Create padded input if needed
    let padded: number[];
    let paddedW: number;
    if (padding > 0) {
      const paddedH = hIn + 2 * padding;
      paddedW = wIn + 2 * padding;
      padded = new Array(paddedH * paddedW).fill(0);
      for (let i = 0; i < hIn; i++) {
        for (let j = 0; j < wIn; j++) {
          padded[(i + padding) * paddedW + (j + padding)] =
            inputMat.data[i * wIn + j];
        }
      }
    } else {
      paddedW = wIn;
      padded = [...inputMat.data];
    }

    // Compute convolution
    const result: number[] = new Array(outH * outW);
    const weightFlat = weight.data;

    for (let oh = 0; oh < outH; oh++) {
      for (let ow = 0; ow < outW; ow++) {
        let s = 0;
        for (let kh = 0; kh < kH; kh++) {
          for (let kw = 0; kw < kW; kw++) {
            const ih = oh * stride + kh;
            const iw = ow * stride + kw;
            s += padded[ih * paddedW + iw] * weightFlat[kh * kW + kw];
          }
        }
        if (bias !== null) {
          // For simplified single-filter case, use bias[0]
          s += bias.size > 0 ? bias.data[0] : 0;
        }
        result[oh * outW + ow] = s;
      }
    }

    return new Matrix(result, outH, outW);
  }

  // =================================================================
  // ML EXTENSIONS: Attention
  // =================================================================

  /**
   * Scaled Dot-Product Attention (Vaswani et al., 2017).
   *
   * ================================================================
   * ATTENTION -- THE CORE OF TRANSFORMERS
   * ================================================================
   *
   * Attention(Q, K, V) = softmax(Q * K^T / sqrt(d_k)) * V
   *
   * Steps:
   * 1. scores = Q * K^T                     (SGEMM, Level 3)
   * 2. scores = scores / scale               (SSCAL-like)
   * 3. if mask: scores = scores + mask        (element-wise)
   * 4. weights = softmax(scores, axis=-1)    (ML extension)
   * 5. output = weights * V                  (SGEMM, Level 3)
   *
   * Q shape: (seq_len x d_k)
   * K shape: (seq_len x d_k)
   * V shape: (seq_len x d_v)
   * Returns: (seq_len x d_v)
   *
   * This is the function that enables GPT, BERT, and every
   * Transformer model to attend to different parts of the input.
   * ================================================================
   */
  attention(
    q: Matrix,
    k: Matrix,
    v: Matrix,
    mask: Matrix | null = null,
    scale: number | null = null,
  ): Matrix {
    const dK = q.cols;
    const actualScale = scale ?? Math.sqrt(dK);

    // Step 1: scores = Q * K^T using SGEMM
    const seqLen = q.rows;
    const scoresC = new Matrix(
      new Array(seqLen * k.rows).fill(0),
      seqLen,
      k.rows,
    );
    const scores = this.sgemm(
      Transpose.NO_TRANS, Transpose.TRANS, 1.0, q, k, 0.0, scoresC
    );

    // Step 2: scale
    const scaledData = scores.data.map((val) => val / actualScale);

    // Step 3: apply mask (additive, typically -inf for masked positions)
    if (mask !== null) {
      for (let i = 0; i < scaledData.length; i++) {
        scaledData[i] += mask.data[i];
      }
    }

    const scoresMatrix = new Matrix(scaledData, scores.rows, scores.cols);

    // Step 4: softmax along the last dimension (each row)
    const weights = this.softmax(scoresMatrix, -1);

    // Step 5: output = weights * V using SGEMM
    const outputC = new Matrix(
      new Array(weights.rows * v.cols).fill(0),
      weights.rows,
      v.cols,
    );
    return this.sgemm(
      Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, weights, v, 0.0, outputC
    );
  }
}
