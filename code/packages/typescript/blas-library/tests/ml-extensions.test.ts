/**
 * Tests for ML extensions: activation functions, normalization, conv2d, attention.
 *
 * These operations are only implemented by the CpuBlas backend (which implements
 * MlBlasBackend). The GPU backends only implement the core BlasBackend interface.
 */

import { describe, it, expect } from "vitest";
import { Vector, Matrix } from "../src/types.js";
import { CpuBlas } from "../src/backends/cpu.js";

const blas = new CpuBlas();

// =========================================================================
// Activation functions
// =========================================================================

describe("ML: ReLU", () => {
  it("should zero out negatives", () => {
    const x = new Matrix([-1, 2, -3, 4], 2, 2);
    const result = blas.relu(x);
    expect(result.data).toEqual([0, 2, 0, 4]);
  });

  it("should pass through positives unchanged", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const result = blas.relu(x);
    expect(result.data).toEqual([1, 2, 3, 4]);
  });

  it("should handle all zeros", () => {
    const x = new Matrix([0, 0, 0, 0], 2, 2);
    const result = blas.relu(x);
    expect(result.data).toEqual([0, 0, 0, 0]);
  });

  it("should handle all negatives", () => {
    const x = new Matrix([-1, -2, -3, -4], 2, 2);
    const result = blas.relu(x);
    expect(result.data).toEqual([0, 0, 0, 0]);
  });

  it("should handle large values", () => {
    const x = new Matrix([1000, -1000], 1, 2);
    const result = blas.relu(x);
    expect(result.data).toEqual([1000, 0]);
  });

  it("should preserve matrix dimensions", () => {
    const x = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const result = blas.relu(x);
    expect(result.rows).toBe(2);
    expect(result.cols).toBe(3);
  });
});

describe("ML: GELU", () => {
  it("should approximate 0 for large negative values", () => {
    const x = new Matrix([-5, -10], 1, 2);
    const result = blas.gelu(x);
    expect(result.data[0]).toBeCloseTo(0, 2);
    expect(result.data[1]).toBeCloseTo(0, 2);
  });

  it("should approximate x for large positive values", () => {
    const x = new Matrix([5, 10], 1, 2);
    const result = blas.gelu(x);
    expect(result.data[0]).toBeCloseTo(5, 1);
    expect(result.data[1]).toBeCloseTo(10, 1);
  });

  it("should be 0 at x=0", () => {
    const x = new Matrix([0], 1, 1);
    const result = blas.gelu(x);
    expect(result.data[0]).toBeCloseTo(0, 5);
  });

  it("should be smooth (no discontinuity around 0)", () => {
    const x = new Matrix([-0.1, 0, 0.1], 1, 3);
    const result = blas.gelu(x);
    // GELU(-0.1) < 0 < GELU(0.1) and all close to 0
    expect(result.data[0]).toBeLessThan(0);
    expect(result.data[2]).toBeGreaterThan(0);
  });
});

describe("ML: Sigmoid", () => {
  it("should return 0.5 at x=0", () => {
    const x = new Matrix([0], 1, 1);
    const result = blas.sigmoid(x);
    expect(result.data[0]).toBeCloseTo(0.5, 5);
  });

  it("should approach 1 for large positive values", () => {
    const x = new Matrix([10, 20], 1, 2);
    const result = blas.sigmoid(x);
    expect(result.data[0]).toBeCloseTo(1.0, 3);
    expect(result.data[1]).toBeCloseTo(1.0, 5);
  });

  it("should approach 0 for large negative values", () => {
    const x = new Matrix([-10, -20], 1, 2);
    const result = blas.sigmoid(x);
    expect(result.data[0]).toBeCloseTo(0.0, 3);
    expect(result.data[1]).toBeCloseTo(0.0, 5);
  });

  it("should be monotonically increasing", () => {
    const x = new Matrix([-2, -1, 0, 1, 2], 1, 5);
    const result = blas.sigmoid(x);
    for (let i = 1; i < result.data.length; i++) {
      expect(result.data[i]).toBeGreaterThan(result.data[i - 1]);
    }
  });

  it("should handle numerically stable for very negative inputs", () => {
    const x = new Matrix([-100], 1, 1);
    const result = blas.sigmoid(x);
    expect(result.data[0]).toBeGreaterThanOrEqual(0);
    expect(isFinite(result.data[0])).toBe(true);
  });
});

describe("ML: Tanh", () => {
  it("should return 0 at x=0", () => {
    const x = new Matrix([0], 1, 1);
    const result = blas.tanhActivation(x);
    expect(result.data[0]).toBeCloseTo(0, 5);
  });

  it("should approach 1 for large positive values", () => {
    const x = new Matrix([10], 1, 1);
    const result = blas.tanhActivation(x);
    expect(result.data[0]).toBeCloseTo(1.0, 5);
  });

  it("should approach -1 for large negative values", () => {
    const x = new Matrix([-10], 1, 1);
    const result = blas.tanhActivation(x);
    expect(result.data[0]).toBeCloseTo(-1.0, 5);
  });

  it("should be odd function: tanh(-x) = -tanh(x)", () => {
    const x = new Matrix([1, -1, 2, -2], 2, 2);
    const result = blas.tanhActivation(x);
    expect(result.data[0]).toBeCloseTo(-result.data[1], 5);
    expect(result.data[2]).toBeCloseTo(-result.data[3], 5);
  });
});

// =========================================================================
// Softmax
// =========================================================================

describe("ML: Softmax", () => {
  it("should produce probabilities that sum to 1 (row-wise)", () => {
    const x = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const result = blas.softmax(x, -1);
    // Each row should sum to 1
    for (let i = 0; i < result.rows; i++) {
      let rowSum = 0;
      for (let j = 0; j < result.cols; j++) {
        rowSum += result.data[i * result.cols + j];
      }
      expect(rowSum).toBeCloseTo(1.0, 5);
    }
  });

  it("should produce all positive values", () => {
    const x = new Matrix([-100, -200, -300], 1, 3);
    const result = blas.softmax(x, -1);
    for (const v of result.data) {
      expect(v).toBeGreaterThan(0);
    }
  });

  it("should handle uniform input (all equal)", () => {
    const x = new Matrix([1, 1, 1], 1, 3);
    const result = blas.softmax(x, -1);
    for (const v of result.data) {
      expect(v).toBeCloseTo(1 / 3, 5);
    }
  });

  it("should assign most probability to the largest value", () => {
    const x = new Matrix([1, 10, 1], 1, 3);
    const result = blas.softmax(x, -1);
    expect(result.data[1]).toBeGreaterThan(result.data[0]);
    expect(result.data[1]).toBeGreaterThan(result.data[2]);
  });

  it("should work along axis 0 (column-wise)", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const result = blas.softmax(x, 0);
    // Each column should sum to 1
    for (let j = 0; j < result.cols; j++) {
      let colSum = 0;
      for (let i = 0; i < result.rows; i++) {
        colSum += result.data[i * result.cols + j];
      }
      expect(colSum).toBeCloseTo(1.0, 5);
    }
  });

  it("should be numerically stable for large values", () => {
    const x = new Matrix([1000, 1001, 1002], 1, 3);
    const result = blas.softmax(x, -1);
    let sum = 0;
    for (const v of result.data) {
      expect(isFinite(v)).toBe(true);
      sum += v;
    }
    expect(sum).toBeCloseTo(1.0, 5);
  });

  it("default axis should be -1 (row-wise)", () => {
    const x = new Matrix([1, 2, 3], 1, 3);
    const result = blas.softmax(x);
    let sum = 0;
    for (const v of result.data) sum += v;
    expect(sum).toBeCloseTo(1.0, 5);
  });
});

// =========================================================================
// Layer Normalization
// =========================================================================

describe("ML: LayerNorm", () => {
  it("should normalize each row to zero mean", () => {
    const x = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const gamma = new Vector([1, 1, 1], 3);
    const beta = new Vector([0, 0, 0], 3);
    const result = blas.layerNorm(x, gamma, beta);

    // Each row should have approximately zero mean
    for (let i = 0; i < result.rows; i++) {
      let mean = 0;
      for (let j = 0; j < result.cols; j++) {
        mean += result.data[i * result.cols + j];
      }
      mean /= result.cols;
      expect(mean).toBeCloseTo(0, 3);
    }
  });

  it("should apply gamma scaling", () => {
    const x = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const gamma = new Vector([2, 2, 2], 3);
    const beta = new Vector([0, 0, 0], 3);
    const result1 = blas.layerNorm(x, new Vector([1, 1, 1], 3), beta);
    const result2 = blas.layerNorm(x, gamma, beta);

    // result2 should be 2x result1
    for (let i = 0; i < result1.data.length; i++) {
      expect(result2.data[i]).toBeCloseTo(2 * result1.data[i], 3);
    }
  });

  it("should apply beta shift", () => {
    const x = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const gamma = new Vector([1, 1, 1], 3);
    const beta = new Vector([10, 10, 10], 3);
    const result = blas.layerNorm(x, gamma, beta);

    // Mean of each row should be approximately 10
    for (let i = 0; i < result.rows; i++) {
      let mean = 0;
      for (let j = 0; j < result.cols; j++) {
        mean += result.data[i * result.cols + j];
      }
      mean /= result.cols;
      expect(mean).toBeCloseTo(10, 3);
    }
  });

  it("should throw if gamma size mismatches", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const gamma = new Vector([1, 1, 1], 3);
    const beta = new Vector([0, 0], 2);
    expect(() => blas.layerNorm(x, gamma, beta)).toThrow("gamma.size");
  });

  it("should throw if beta size mismatches", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const gamma = new Vector([1, 1], 2);
    const beta = new Vector([0, 0, 0], 3);
    expect(() => blas.layerNorm(x, gamma, beta)).toThrow("beta.size");
  });
});

// =========================================================================
// Batch Normalization
// =========================================================================

describe("ML: BatchNorm", () => {
  it("should normalize using running statistics (inference mode)", () => {
    const x = new Matrix([10, 20, 30, 40], 2, 2);
    const gamma = new Vector([1, 1], 2);
    const beta = new Vector([0, 0], 2);
    const runMean = new Vector([20, 30], 2);
    const runVar = new Vector([100, 100], 2);
    const result = blas.batchNorm(x, gamma, beta, runMean, runVar, 1e-5, false);
    // x_hat = (x - mean) / sqrt(var + eps)
    // (10-20)/10 = -1, (20-30)/10 = -1
    // (30-20)/10 = 1, (40-30)/10 = 1
    expect(result.data[0]).toBeCloseTo(-1, 3);
    expect(result.data[1]).toBeCloseTo(-1, 3);
    expect(result.data[2]).toBeCloseTo(1, 3);
    expect(result.data[3]).toBeCloseTo(1, 3);
  });

  it("should normalize using batch statistics (training mode)", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const gamma = new Vector([1, 1], 2);
    const beta = new Vector([0, 0], 2);
    const runMean = new Vector([0, 0], 2);
    const runVar = new Vector([1, 1], 2);
    const result = blas.batchNorm(x, gamma, beta, runMean, runVar, 1e-5, true);

    // Column means: [2, 3], variance: [1, 1]
    // x_hat[0,0] = (1-2)/1 = -1, x_hat[0,1] = (2-3)/1 = -1
    // x_hat[1,0] = (3-2)/1 = 1, x_hat[1,1] = (4-3)/1 = 1
    expect(result.data[0]).toBeCloseTo(-1, 3);
    expect(result.data[1]).toBeCloseTo(-1, 3);
    expect(result.data[2]).toBeCloseTo(1, 3);
    expect(result.data[3]).toBeCloseTo(1, 3);
  });

  it("should throw if gamma size mismatches", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const gamma = new Vector([1, 1, 1], 3);
    const beta = new Vector([0, 0], 2);
    const mean = new Vector([0, 0], 2);
    const variance = new Vector([1, 1], 2);
    expect(() => blas.batchNorm(x, gamma, beta, mean, variance)).toThrow("gamma.size");
  });

  it("should throw if beta size mismatches", () => {
    const x = new Matrix([1, 2, 3, 4], 2, 2);
    const gamma = new Vector([1, 1], 2);
    const beta = new Vector([0, 0, 0], 3);
    const mean = new Vector([0, 0], 2);
    const variance = new Vector([1, 1], 2);
    expect(() => blas.batchNorm(x, gamma, beta, mean, variance)).toThrow("beta.size");
  });
});

// =========================================================================
// Convolution
// =========================================================================

describe("ML: Conv2d", () => {
  it("should apply a 2x2 filter to a 3x3 input", () => {
    // Input: 3x3 identity-ish
    const input = new Matrix([1, 0, 0, 0, 1, 0, 0, 0, 1], 3, 3);
    // Filter: 2x2 all ones
    const weight = new Matrix([1, 1, 1, 1], 2, 2);
    const result = blas.conv2d(input, weight);
    // Output: 2x2 (no padding, stride 1)
    expect(result.rows).toBe(2);
    expect(result.cols).toBe(2);
    // Top-left: 1+0+0+1 = 2
    expect(result.data[0]).toBeCloseTo(2, 3);
  });

  it("should handle stride > 1", () => {
    const input = new Matrix([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], 4, 4);
    const weight = new Matrix([1, 0, 0, 1], 2, 2);
    const result = blas.conv2d(input, weight, null, 2);
    // Output: (4 - 2)/2 + 1 = 2 -> 2x2
    expect(result.rows).toBe(2);
    expect(result.cols).toBe(2);
  });

  it("should handle padding", () => {
    const input = new Matrix([1, 2, 3, 4], 2, 2);
    const weight = new Matrix([1, 1, 1, 1], 2, 2);
    // padding=1: input becomes 4x4 (padded with zeros)
    const result = blas.conv2d(input, weight, null, 1, 1);
    // Output: (2+2*1-2)/1 + 1 = 3 -> 3x3
    expect(result.rows).toBe(3);
    expect(result.cols).toBe(3);
  });

  it("should apply bias", () => {
    const input = new Matrix([1, 1, 1, 1], 2, 2);
    const weight = new Matrix([1], 1, 1);
    const bias = new Vector([10], 1);
    const result = blas.conv2d(input, weight, bias);
    // Each output = 1*1 + 10 = 11
    for (const v of result.data) {
      expect(v).toBeCloseTo(11, 3);
    }
  });

  it("should work without bias (null)", () => {
    const input = new Matrix([1, 1, 1, 1], 2, 2);
    const weight = new Matrix([1], 1, 1);
    const result = blas.conv2d(input, weight, null);
    for (const v of result.data) {
      expect(v).toBeCloseTo(1, 3);
    }
  });

  it("should throw for non-positive output dimensions", () => {
    const input = new Matrix([1], 1, 1);
    const weight = new Matrix([1, 1, 1, 1], 2, 2);
    expect(() => blas.conv2d(input, weight)).toThrow("non-positive");
  });
});

// =========================================================================
// Attention
// =========================================================================

describe("ML: Attention", () => {
  it("should compute scaled dot-product attention", () => {
    // Simple 2x2 Q, K, V
    const q = new Matrix([1, 0, 0, 1], 2, 2);
    const k = new Matrix([1, 0, 0, 1], 2, 2);
    const v = new Matrix([1, 0, 0, 1], 2, 2);
    const result = blas.attention(q, k, v);
    expect(result.rows).toBe(2);
    expect(result.cols).toBe(2);
    // Each row should be a valid probability-weighted combination of V rows
  });

  it("should produce output with correct shape", () => {
    const q = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const k = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const v = new Matrix([1, 2, 3, 4], 2, 2);
    const result = blas.attention(q, k, v);
    expect(result.rows).toBe(2); // seq_len from Q
    expect(result.cols).toBe(2); // d_v from V
  });

  it("should apply mask (additive)", () => {
    const q = new Matrix([1, 0, 0, 1], 2, 2);
    const k = new Matrix([1, 0, 0, 1], 2, 2);
    const v = new Matrix([1, 0, 0, 1], 2, 2);
    // Mask out position [0,1] and [1,0]
    const mask = new Matrix([0, -1e9, -1e9, 0], 2, 2);
    const result = blas.attention(q, k, v, mask);
    // With mask, each query only attends to its own key
    // Row 0 attends mainly to key 0, row 1 attends mainly to key 1
    expect(result.data[0]).toBeCloseTo(1, 1); // v[0,0]
    expect(result.data[3]).toBeCloseTo(1, 1); // v[1,1]
  });

  it("should use custom scale", () => {
    const q = new Matrix([1, 0, 0, 1], 2, 2);
    const k = new Matrix([1, 0, 0, 1], 2, 2);
    const v = new Matrix([1, 0, 0, 1], 2, 2);
    const result = blas.attention(q, k, v, null, 1.0);
    expect(result.rows).toBe(2);
    expect(result.cols).toBe(2);
  });

  it("should handle single-token sequence", () => {
    const q = new Matrix([1, 2], 1, 2);
    const k = new Matrix([3, 4], 1, 2);
    const v = new Matrix([5, 6], 1, 2);
    const result = blas.attention(q, k, v);
    // Single token: attention weight is 1.0 (only one key)
    expect(result.rows).toBe(1);
    expect(result.cols).toBe(2);
    expect(result.data[0]).toBeCloseTo(5, 3);
    expect(result.data[1]).toBeCloseTo(6, 3);
  });
});
