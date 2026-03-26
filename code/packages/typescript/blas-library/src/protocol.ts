/**
 * BLAS Backend Protocols -- the contracts every backend must fulfill.
 *
 * === What is an Interface? ===
 *
 * A TypeScript interface defines a set of methods that a class must implement
 * to be considered "compatible." If your class has the right methods with the
 * right signatures, it satisfies the interface.
 *
 *     interface BlasBackend {
 *         saxpy(alpha: number, x: Vector, y: Vector): Vector;
 *     }
 *
 *     class MyCoolBackend implements BlasBackend {
 *         saxpy(alpha: number, x: Vector, y: Vector): Vector {
 *             return ...;  // just implement the method
 *         }
 *     }
 *
 * === Two Interfaces ===
 *
 * 1. `BlasBackend` -- the core BLAS operations (Levels 1, 2, 3).
 *    Every backend MUST implement this.
 *
 * 2. `MlBlasBackend` -- extends BlasBackend with ML operations
 *    (activations, softmax, normalization, conv2d, attention).
 *    This is OPTIONAL. The CPU backend implements it as a reference.
 */

import type { Matrix, Side, Transpose, Vector } from "./types.js";

// =========================================================================
// BlasBackend -- the core interface
// =========================================================================

/**
 * The BLAS backend interface -- the contract every backend must fulfill.
 *
 * ================================================================
 * THE BLAS BACKEND INTERFACE
 * ================================================================
 *
 * This is the contract every backend must fulfill. Whether you're
 * running on an NVIDIA GPU, an Apple M4, or a Raspberry Pi CPU,
 * if you implement this interface, you're a valid BLAS backend.
 *
 * All operations return NEW Matrix/Vector objects. They do not
 * mutate inputs. This is cleaner for testing and avoids aliasing
 * bugs. Real BLAS mutates in-place for performance, but we
 * optimize for clarity.
 * ================================================================
 */
export interface BlasBackend {
  /** Backend identifier: 'cpu', 'cuda', 'metal', etc. */
  readonly name: string;

  /** Human-readable device name: 'NVIDIA H100', 'Apple M4', 'CPU', etc. */
  readonly deviceName: string;

  // ==========================================================
  // LEVEL 1: VECTOR-VECTOR OPERATIONS -- O(n)
  // ==========================================================

  /**
   * SAXPY: y = alpha * x + y
   *
   * The most famous BLAS operation. Each element:
   *     result[i] = alpha * x[i] + y[i]
   *
   * Requires: x.size == y.size
   * Returns: new Vector of same size
   */
  saxpy(alpha: number, x: Vector, y: Vector): Vector;

  /**
   * DOT product: result = x . y = sum(x_i * y_i)
   *
   * Requires: x.size == y.size
   * Returns: scalar number
   */
  sdot(x: Vector, y: Vector): number;

  /**
   * Euclidean norm: result = ||x||_2 = sqrt(sum(x_i^2))
   *
   * Returns: scalar number >= 0
   */
  snrm2(x: Vector): number;

  /**
   * Scale: result = alpha * x
   *
   * Returns: new Vector of same size
   */
  sscal(alpha: number, x: Vector): Vector;

  /**
   * Absolute sum: result = sum(|x_i|)
   *
   * Returns: scalar number >= 0
   */
  sasum(x: Vector): number;

  /**
   * Index of max absolute value: result = argmax(|x_i|)
   *
   * Returns: integer index (0-based)
   */
  isamax(x: Vector): number;

  /**
   * Copy: result = x (deep copy)
   *
   * Returns: new Vector with same data
   */
  scopy(x: Vector): Vector;

  /**
   * Swap: x <-> y
   *
   * Returns: [new_x with y's data, new_y with x's data]
   * Requires: x.size == y.size
   */
  sswap(x: Vector, y: Vector): [Vector, Vector];

  // ==========================================================
  // LEVEL 2: MATRIX-VECTOR OPERATIONS -- O(n^2)
  // ==========================================================

  /**
   * General Matrix-Vector multiply: y = alpha * op(A) * x + beta * y
   *
   * If trans == TRANS, uses A^T instead of A.
   * Returns: new Vector
   */
  sgemv(
    trans: Transpose,
    alpha: number,
    a: Matrix,
    x: Vector,
    beta: number,
    y: Vector,
  ): Vector;

  /**
   * Outer product (rank-1 update): A = alpha * x * y^T + A
   *
   * Every element: result[i][j] = alpha * x[i] * y[j] + A[i][j]
   * Requires: A.rows == x.size, A.cols == y.size
   * Returns: new Matrix of same shape as A
   */
  sger(alpha: number, x: Vector, y: Vector, a: Matrix): Matrix;

  // ==========================================================
  // LEVEL 3: MATRIX-MATRIX OPERATIONS -- O(n^3)
  // ==========================================================

  /**
   * General Matrix Multiply: C = alpha * op(A) * op(B) + beta * C
   *
   * where op(X) = X      if trans == NO_TRANS
   *       op(X) = X^T    if trans == TRANS
   *
   * Returns: new Matrix of same shape as C
   */
  sgemm(
    transA: Transpose,
    transB: Transpose,
    alpha: number,
    a: Matrix,
    b: Matrix,
    beta: number,
    c: Matrix,
  ): Matrix;

  /**
   * Symmetric Matrix Multiply: C = alpha * A * B + beta * C (A symmetric)
   *
   * If side == LEFT:  C = alpha * A * B + beta * C
   * If side == RIGHT: C = alpha * B * A + beta * C
   *
   * A must be square and symmetric.
   * Returns: new Matrix of same shape as C
   */
  ssymm(
    side: Side,
    alpha: number,
    a: Matrix,
    b: Matrix,
    beta: number,
    c: Matrix,
  ): Matrix;

  /**
   * Batched GEMM: multiple independent GEMMs.
   *
   *     Cs[i] = alpha * op(As[i]) * op(Bs[i]) + beta * Cs[i]
   *
   * Requires: aList.length == bList.length == cList.length
   * Returns: array of new Matrices
   */
  sgemmBatched(
    transA: Transpose,
    transB: Transpose,
    alpha: number,
    aList: Matrix[],
    bList: Matrix[],
    beta: number,
    cList: Matrix[],
  ): Matrix[];
}

// =========================================================================
// MlBlasBackend -- optional ML extensions
// =========================================================================

/**
 * ML extensions beyond classic BLAS.
 *
 * ================================================================
 * ML EXTENSIONS BEYOND CLASSIC BLAS
 * ================================================================
 *
 * Classic BLAS handles linear algebra. ML needs additional operations:
 * activation functions, normalization, convolution, and attention.
 * These operations CAN be built from BLAS primitives (attention = two
 * GEMMs + softmax), but dedicated implementations are much faster.
 *
 * This interface is OPTIONAL. A backend that only implements BlasBackend
 * is still a valid BLAS backend.
 * ================================================================
 */
export interface MlBlasBackend extends BlasBackend {
  /** ReLU: result[i] = max(0, x[i]) */
  relu(x: Matrix): Matrix;

  /** GELU: result[i] = x[i] * Phi(x[i]) where Phi is CDF of N(0,1) */
  gelu(x: Matrix): Matrix;

  /** Sigmoid: result[i] = 1 / (1 + exp(-x[i])) */
  sigmoid(x: Matrix): Matrix;

  /** Tanh: result[i] = tanh(x[i]) */
  tanhActivation(x: Matrix): Matrix;

  /** Softmax along an axis (numerically stable). */
  softmax(x: Matrix, axis?: number): Matrix;

  /** Layer Normalization (Ba et al., 2016). */
  layerNorm(
    x: Matrix,
    gamma: Vector,
    beta: Vector,
    eps?: number,
  ): Matrix;

  /** Batch Normalization (Ioffe & Szegedy, 2015). */
  batchNorm(
    x: Matrix,
    gamma: Vector,
    beta: Vector,
    runningMean: Vector,
    runningVar: Vector,
    eps?: number,
    training?: boolean,
  ): Matrix;

  /** 2D Convolution via im2col + GEMM. */
  conv2d(
    inputMat: Matrix,
    weight: Matrix,
    bias?: Vector | null,
    stride?: number,
    padding?: number,
  ): Matrix;

  /** Scaled Dot-Product Attention (Vaswani et al., 2017). */
  attention(
    q: Matrix,
    k: Matrix,
    v: Matrix,
    mask?: Matrix | null,
    scale?: number | null,
  ): Matrix;
}
