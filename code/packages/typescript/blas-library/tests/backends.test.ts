/**
 * Comprehensive tests for all seven BLAS backends.
 *
 * === Test Strategy ===
 *
 * Every backend must produce identical results for every operation. We test
 * the CPU backend first (it's the reference), then verify all GPU backends
 * match. This is a property-based approach: the GPU backends are correct
 * if and only if they agree with the CPU backend on every input.
 *
 * We use a helper `allBackends` that instantiates all seven backends and
 * runs each test function against every one.
 */

import { describe, it, expect } from "vitest";
import { Vector, Matrix, Transpose, Side } from "../src/types.js";
import type { BlasBackend } from "../src/protocol.js";
import { CpuBlas } from "../src/backends/cpu.js";
import { CudaBlas } from "../src/backends/cuda.js";
import { MetalBlas } from "../src/backends/metal.js";
import { VulkanBlas } from "../src/backends/vulkan.js";
import { OpenClBlas } from "../src/backends/opencl.js";
import { WebGpuBlas } from "../src/backends/webgpu.js";
import { OpenGlBlas } from "../src/backends/opengl.js";

// =========================================================================
// Backend instantiation
// =========================================================================

/**
 * All seven backends, instantiated once for the test suite.
 * Each GPU backend exercises a different vendor API from Layer 4.
 */
const backendEntries: [string, BlasBackend][] = [
  ["cpu", new CpuBlas()],
  ["cuda", new CudaBlas()],
  ["metal", new MetalBlas()],
  ["vulkan", new VulkanBlas()],
  ["opencl", new OpenClBlas()],
  ["webgpu", new WebGpuBlas()],
  ["opengl", new OpenGlBlas()],
];

// =========================================================================
// Helper: compare results with tolerance
// =========================================================================

/**
 * GPU backends round-trip through Float32, which loses precision compared
 * to JavaScript's Float64 numbers. We use a tolerance of 1e-4 for
 * comparing results.
 */
const TOL = 1e-4;

function expectVectorClose(actual: Vector, expected: Vector): void {
  expect(actual.size).toBe(expected.size);
  for (let i = 0; i < expected.size; i++) {
    expect(actual.data[i]).toBeCloseTo(expected.data[i], 3);
  }
}

function expectMatrixClose(actual: Matrix, expected: Matrix): void {
  expect(actual.rows).toBe(expected.rows);
  expect(actual.cols).toBe(expected.cols);
  for (let i = 0; i < expected.data.length; i++) {
    expect(actual.data[i]).toBeCloseTo(expected.data[i], 3);
  }
}

// =========================================================================
// LEVEL 1: Vector-Vector Operations
// =========================================================================

describe("Level 1: SAXPY", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: y = 2*x + y`, () => {
      const x = new Vector([1, 2, 3], 3);
      const y = new Vector([4, 5, 6], 3);
      const result = blas.saxpy(2.0, x, y);
      expectVectorClose(result, new Vector([6, 9, 12], 3));
    });

    it(`${name}: alpha=0 returns y`, () => {
      const x = new Vector([1, 2, 3], 3);
      const y = new Vector([4, 5, 6], 3);
      const result = blas.saxpy(0.0, x, y);
      expectVectorClose(result, y);
    });

    it(`${name}: alpha=1 is vector addition`, () => {
      const x = new Vector([1, 2, 3], 3);
      const y = new Vector([4, 5, 6], 3);
      const result = blas.saxpy(1.0, x, y);
      expectVectorClose(result, new Vector([5, 7, 9], 3));
    });

    it(`${name}: alpha=-1 is vector subtraction`, () => {
      const x = new Vector([1, 2, 3], 3);
      const y = new Vector([4, 5, 6], 3);
      const result = blas.saxpy(-1.0, x, y);
      expectVectorClose(result, new Vector([3, 3, 3], 3));
    });

    it(`${name}: single element`, () => {
      const x = new Vector([3], 1);
      const y = new Vector([7], 1);
      const result = blas.saxpy(2.0, x, y);
      expectVectorClose(result, new Vector([13], 1));
    });
  }

  it("cpu: throws on dimension mismatch", () => {
    const blas = new CpuBlas();
    const x = new Vector([1, 2], 2);
    const y = new Vector([1, 2, 3], 3);
    expect(() => blas.saxpy(1.0, x, y)).toThrow("dimension mismatch");
  });
});

describe("Level 1: SDOT", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: dot product of simple vectors`, () => {
      const x = new Vector([1, 2, 3], 3);
      const y = new Vector([4, 5, 6], 3);
      const result = blas.sdot(x, y);
      // 1*4 + 2*5 + 3*6 = 4 + 10 + 18 = 32
      expect(result).toBeCloseTo(32, 3);
    });

    it(`${name}: dot product with zeros`, () => {
      const x = new Vector([0, 0, 0], 3);
      const y = new Vector([4, 5, 6], 3);
      expect(blas.sdot(x, y)).toBeCloseTo(0, 3);
    });

    it(`${name}: orthogonal vectors`, () => {
      const x = new Vector([1, 0], 2);
      const y = new Vector([0, 1], 2);
      expect(blas.sdot(x, y)).toBeCloseTo(0, 3);
    });

    it(`${name}: single element`, () => {
      const x = new Vector([3], 1);
      const y = new Vector([4], 1);
      expect(blas.sdot(x, y)).toBeCloseTo(12, 3);
    });
  }

  it("cpu: throws on dimension mismatch", () => {
    const blas = new CpuBlas();
    expect(() => blas.sdot(new Vector([1], 1), new Vector([1, 2], 2))).toThrow("dimension mismatch");
  });
});

describe("Level 1: SNRM2", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: norm of [3, 4] = 5`, () => {
      const x = new Vector([3, 4], 2);
      expect(blas.snrm2(x)).toBeCloseTo(5, 3);
    });

    it(`${name}: norm of unit vector = 1`, () => {
      const x = new Vector([1, 0, 0], 3);
      expect(blas.snrm2(x)).toBeCloseTo(1, 3);
    });

    it(`${name}: norm of zero vector = 0`, () => {
      const x = new Vector([0, 0, 0], 3);
      expect(blas.snrm2(x)).toBeCloseTo(0, 3);
    });

    it(`${name}: norm of single element`, () => {
      const x = new Vector([5], 1);
      expect(blas.snrm2(x)).toBeCloseTo(5, 3);
    });
  }
});

describe("Level 1: SSCAL", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: scale by 2`, () => {
      const x = new Vector([1, 2, 3], 3);
      const result = blas.sscal(2.0, x);
      expectVectorClose(result, new Vector([2, 4, 6], 3));
    });

    it(`${name}: scale by 0`, () => {
      const x = new Vector([1, 2, 3], 3);
      const result = blas.sscal(0.0, x);
      expectVectorClose(result, new Vector([0, 0, 0], 3));
    });

    it(`${name}: scale by -1`, () => {
      const x = new Vector([1, -2, 3], 3);
      const result = blas.sscal(-1.0, x);
      expectVectorClose(result, new Vector([-1, 2, -3], 3));
    });
  }
});

describe("Level 1: SASUM", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: absolute sum`, () => {
      const x = new Vector([1, -2, 3, -4], 4);
      expect(blas.sasum(x)).toBeCloseTo(10, 3);
    });

    it(`${name}: all positive`, () => {
      const x = new Vector([1, 2, 3], 3);
      expect(blas.sasum(x)).toBeCloseTo(6, 3);
    });

    it(`${name}: zero vector`, () => {
      const x = new Vector([0, 0, 0], 3);
      expect(blas.sasum(x)).toBeCloseTo(0, 3);
    });
  }
});

describe("Level 1: ISAMAX", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: index of max absolute value`, () => {
      const x = new Vector([1, -5, 3], 3);
      expect(blas.isamax(x)).toBe(1);
    });

    it(`${name}: first element is max`, () => {
      const x = new Vector([10, 2, 3], 3);
      expect(blas.isamax(x)).toBe(0);
    });

    it(`${name}: last element is max`, () => {
      const x = new Vector([1, 2, 10], 3);
      expect(blas.isamax(x)).toBe(2);
    });

    it(`${name}: single element returns 0`, () => {
      const x = new Vector([42], 1);
      expect(blas.isamax(x)).toBe(0);
    });
  }
});

describe("Level 1: SCOPY", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: copies data correctly`, () => {
      const x = new Vector([1, 2, 3], 3);
      const result = blas.scopy(x);
      expectVectorClose(result, x);
    });

    it(`${name}: copy is independent (deep copy)`, () => {
      const x = new Vector([1, 2, 3], 3);
      const result = blas.scopy(x);
      expect(result).not.toBe(x);
    });

    it(`${name}: single element copy`, () => {
      const x = new Vector([42], 1);
      const result = blas.scopy(x);
      expectVectorClose(result, x);
    });
  }
});

describe("Level 1: SSWAP", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: swaps two vectors`, () => {
      const x = new Vector([1, 2, 3], 3);
      const y = new Vector([4, 5, 6], 3);
      const [newX, newY] = blas.sswap(x, y);
      expectVectorClose(newX, y);
      expectVectorClose(newY, x);
    });

    it(`${name}: swap single element`, () => {
      const x = new Vector([10], 1);
      const y = new Vector([20], 1);
      const [newX, newY] = blas.sswap(x, y);
      expectVectorClose(newX, new Vector([20], 1));
      expectVectorClose(newY, new Vector([10], 1));
    });
  }

  it("cpu: throws on dimension mismatch", () => {
    const blas = new CpuBlas();
    expect(() => blas.sswap(new Vector([1], 1), new Vector([1, 2], 2))).toThrow("dimension mismatch");
  });
});

// =========================================================================
// LEVEL 2: Matrix-Vector Operations
// =========================================================================

describe("Level 2: SGEMV", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: y = A*x (no trans)`, () => {
      // A = [[1,2],[3,4]], x = [1,1], y = [0,0]
      const a = new Matrix([1, 2, 3, 4], 2, 2);
      const x = new Vector([1, 1], 2);
      const y = new Vector([0, 0], 2);
      const result = blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y);
      // [1*1+2*1, 3*1+4*1] = [3, 7]
      expectVectorClose(result, new Vector([3, 7], 2));
    });

    it(`${name}: y = A^T*x (transposed)`, () => {
      // A = [[1,2],[3,4]], A^T = [[1,3],[2,4]]
      // x = [1,1], result = [4, 6]
      const a = new Matrix([1, 2, 3, 4], 2, 2);
      const x = new Vector([1, 1], 2);
      const y = new Vector([0, 0], 2);
      const result = blas.sgemv(Transpose.TRANS, 1.0, a, x, 0.0, y);
      expectVectorClose(result, new Vector([4, 6], 2));
    });

    it(`${name}: y = alpha*A*x + beta*y`, () => {
      const a = new Matrix([1, 2, 3, 4], 2, 2);
      const x = new Vector([1, 1], 2);
      const y = new Vector([10, 20], 2);
      const result = blas.sgemv(Transpose.NO_TRANS, 2.0, a, x, 1.0, y);
      // 2*[3,7] + 1*[10,20] = [16, 34]
      expectVectorClose(result, new Vector([16, 34], 2));
    });

    it(`${name}: non-square matrix`, () => {
      // A = [[1,2,3],[4,5,6]] (2x3), x = [1,1,1], y = [0,0]
      const a = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
      const x = new Vector([1, 1, 1], 3);
      const y = new Vector([0, 0], 2);
      const result = blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y);
      expectVectorClose(result, new Vector([6, 15], 2));
    });
  }
});

describe("Level 2: SGER", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: outer product A = alpha*x*y^T + A`, () => {
      const x = new Vector([1, 2], 2);
      const y = new Vector([3, 4, 5], 3);
      const a = new Matrix([0, 0, 0, 0, 0, 0], 2, 3);
      const result = blas.sger(1.0, x, y, a);
      // [[1*3, 1*4, 1*5], [2*3, 2*4, 2*5]] = [[3,4,5],[6,8,10]]
      expectMatrixClose(result, new Matrix([3, 4, 5, 6, 8, 10], 2, 3));
    });

    it(`${name}: rank-1 update adds to existing matrix`, () => {
      const x = new Vector([1, 1], 2);
      const y = new Vector([1, 1], 2);
      const a = new Matrix([10, 20, 30, 40], 2, 2);
      const result = blas.sger(1.0, x, y, a);
      expectMatrixClose(result, new Matrix([11, 21, 31, 41], 2, 2));
    });
  }
});

// =========================================================================
// LEVEL 3: Matrix-Matrix Operations
// =========================================================================

describe("Level 3: SGEMM", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: C = A*B (simple 2x2)`, () => {
      const a = new Matrix([1, 2, 3, 4], 2, 2);
      const b = new Matrix([5, 6, 7, 8], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c,
      );
      // [[1*5+2*7, 1*6+2*8], [3*5+4*7, 3*6+4*8]] = [[19,22],[43,50]]
      expectMatrixClose(result, new Matrix([19, 22, 43, 50], 2, 2));
    });

    it(`${name}: C = A^T * B`, () => {
      // A = [[1,3],[2,4]], A^T = [[1,2],[3,4]]
      const a = new Matrix([1, 3, 2, 4], 2, 2);
      const b = new Matrix([5, 6, 7, 8], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.sgemm(
        Transpose.TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c,
      );
      // A^T = [[1,2],[3,4]], result = [[19,22],[43,50]]
      expectMatrixClose(result, new Matrix([19, 22, 43, 50], 2, 2));
    });

    it(`${name}: C = A * B^T`, () => {
      const a = new Matrix([1, 2, 3, 4], 2, 2);
      // B = [[5,7],[6,8]], B^T = [[5,6],[7,8]]
      const b = new Matrix([5, 7, 6, 8], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.sgemm(
        Transpose.NO_TRANS, Transpose.TRANS, 1.0, a, b, 0.0, c,
      );
      // B^T = [[5,6],[7,8]], result = [[19,22],[43,50]]
      expectMatrixClose(result, new Matrix([19, 22, 43, 50], 2, 2));
    });

    it(`${name}: C = alpha*A*B + beta*C`, () => {
      const a = new Matrix([1, 0, 0, 1], 2, 2); // identity
      const b = new Matrix([5, 6, 7, 8], 2, 2);
      const c = new Matrix([1, 1, 1, 1], 2, 2);
      const result = blas.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 2.0, a, b, 3.0, c,
      );
      // 2*I*B + 3*C = 2*[[5,6],[7,8]] + 3*[[1,1],[1,1]] = [[13,15],[17,19]]
      expectMatrixClose(result, new Matrix([13, 15, 17, 19], 2, 2));
    });

    it(`${name}: non-square matrices (2x3 * 3x2)`, () => {
      const a = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
      const b = new Matrix([7, 8, 9, 10, 11, 12], 3, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c,
      );
      // [[1*7+2*9+3*11, 1*8+2*10+3*12], [4*7+5*9+6*11, 4*8+5*10+6*12]]
      // = [[58,64],[139,154]]
      expectMatrixClose(result, new Matrix([58, 64, 139, 154], 2, 2));
    });

    it(`${name}: 1x1 matrices`, () => {
      const a = new Matrix([3], 1, 1);
      const b = new Matrix([4], 1, 1);
      const c = new Matrix([0], 1, 1);
      const result = blas.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c,
      );
      expectMatrixClose(result, new Matrix([12], 1, 1));
    });

    it(`${name}: identity matrix multiplication`, () => {
      const I = new Matrix([1, 0, 0, 1], 2, 2);
      const b = new Matrix([5, 6, 7, 8], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, I, b, 0.0, c,
      );
      expectMatrixClose(result, b);
    });
  }

  it("cpu: throws on inner dimension mismatch", () => {
    const blas = new CpuBlas();
    const a = new Matrix([1, 2, 3, 4], 2, 2);
    const b = new Matrix([1, 2, 3, 4, 5, 6], 3, 2);
    const c = new Matrix([0, 0, 0, 0], 2, 2);
    expect(() =>
      blas.sgemm(Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c),
    ).toThrow("dimension mismatch");
  });
});

describe("Level 3: SSYMM", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: LEFT: C = A*B where A is symmetric`, () => {
      // A is symmetric: [[1,2],[2,1]]
      const a = new Matrix([1, 2, 2, 1], 2, 2);
      const b = new Matrix([1, 0, 0, 1], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.ssymm(Side.LEFT, 1.0, a, b, 0.0, c);
      expectMatrixClose(result, new Matrix([1, 2, 2, 1], 2, 2));
    });

    it(`${name}: RIGHT: C = B*A where A is symmetric`, () => {
      const a = new Matrix([1, 2, 2, 1], 2, 2);
      const b = new Matrix([1, 0, 0, 1], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const result = blas.ssymm(Side.RIGHT, 1.0, a, b, 0.0, c);
      expectMatrixClose(result, new Matrix([1, 2, 2, 1], 2, 2));
    });

    it(`${name}: with alpha and beta`, () => {
      const a = new Matrix([1, 0, 0, 1], 2, 2); // identity is symmetric
      const b = new Matrix([5, 6, 7, 8], 2, 2);
      const c = new Matrix([1, 1, 1, 1], 2, 2);
      const result = blas.ssymm(Side.LEFT, 2.0, a, b, 3.0, c);
      // 2*I*B + 3*C = [[13,15],[17,19]]
      expectMatrixClose(result, new Matrix([13, 15, 17, 19], 2, 2));
    });
  }
});

describe("Level 3: sgemmBatched", () => {
  for (const [name, blas] of backendEntries) {
    it(`${name}: batched GEMM with 2 operations`, () => {
      const a1 = new Matrix([1, 0, 0, 1], 2, 2);
      const b1 = new Matrix([5, 6, 7, 8], 2, 2);
      const c1 = new Matrix([0, 0, 0, 0], 2, 2);

      const a2 = new Matrix([2, 0, 0, 2], 2, 2);
      const b2 = new Matrix([1, 1, 1, 1], 2, 2);
      const c2 = new Matrix([0, 0, 0, 0], 2, 2);

      const results = blas.sgemmBatched(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0,
        [a1, a2], [b1, b2], 0.0, [c1, c2],
      );

      expect(results.length).toBe(2);
      expectMatrixClose(results[0], new Matrix([5, 6, 7, 8], 2, 2));
      expectMatrixClose(results[1], new Matrix([2, 2, 2, 2], 2, 2));
    });

    it(`${name}: single batch`, () => {
      const a = new Matrix([1, 2, 3, 4], 2, 2);
      const b = new Matrix([5, 6, 7, 8], 2, 2);
      const c = new Matrix([0, 0, 0, 0], 2, 2);
      const results = blas.sgemmBatched(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0,
        [a], [b], 0.0, [c],
      );
      expect(results.length).toBe(1);
      expectMatrixClose(results[0], new Matrix([19, 22, 43, 50], 2, 2));
    });
  }
});

// =========================================================================
// Backend identity checks
// =========================================================================

describe("Backend identity", () => {
  it("cpu backend has correct name", () => {
    const blas = new CpuBlas();
    expect(blas.name).toBe("cpu");
    expect(blas.deviceName).toContain("CPU");
  });

  it("cuda backend has correct name", () => {
    const blas = new CudaBlas();
    expect(blas.name).toBe("cuda");
    expect(blas.deviceName).toBeDefined();
  });

  it("metal backend has correct name", () => {
    const blas = new MetalBlas();
    expect(blas.name).toBe("metal");
    expect(blas.deviceName).toBeDefined();
  });

  it("vulkan backend has correct name", () => {
    const blas = new VulkanBlas();
    expect(blas.name).toBe("vulkan");
    expect(blas.deviceName).toBe("Vulkan Device");
  });

  it("opencl backend has correct name", () => {
    const blas = new OpenClBlas();
    expect(blas.name).toBe("opencl");
    expect(blas.deviceName).toBeDefined();
  });

  it("webgpu backend has correct name", () => {
    const blas = new WebGpuBlas();
    expect(blas.name).toBe("webgpu");
    expect(blas.deviceName).toBe("WebGPU Device");
  });

  it("opengl backend has correct name", () => {
    const blas = new OpenGlBlas();
    expect(blas.name).toBe("opengl");
    expect(blas.deviceName).toBe("OpenGL Device");
  });
});

// =========================================================================
// Cross-backend equivalence tests
// =========================================================================

describe("Cross-backend equivalence", () => {
  const cpu = new CpuBlas();
  const gpuBackends: [string, BlasBackend][] = backendEntries.filter(
    ([n]) => n !== "cpu",
  );

  it("all backends produce same SAXPY result", () => {
    const x = new Vector([1.5, 2.5, 3.5, 4.5], 4);
    const y = new Vector([0.1, 0.2, 0.3, 0.4], 4);
    const cpuResult = cpu.saxpy(2.5, x, y);

    for (const [name, blas] of gpuBackends) {
      const gpuResult = blas.saxpy(2.5, x, y);
      expectVectorClose(gpuResult, cpuResult);
    }
  });

  it("all backends produce same SDOT result", () => {
    const x = new Vector([1.5, 2.5, 3.5], 3);
    const y = new Vector([4.5, 5.5, 6.5], 3);
    const cpuResult = cpu.sdot(x, y);

    for (const [name, blas] of gpuBackends) {
      expect(blas.sdot(x, y)).toBeCloseTo(cpuResult, 2);
    }
  });

  it("all backends produce same SGEMM result", () => {
    const a = new Matrix([1, 2, 3, 4, 5, 6, 7, 8, 9], 3, 3);
    const b = new Matrix([9, 8, 7, 6, 5, 4, 3, 2, 1], 3, 3);
    const c = new Matrix([0, 0, 0, 0, 0, 0, 0, 0, 0], 3, 3);
    const cpuResult = cpu.sgemm(
      Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c,
    );

    for (const [name, blas] of gpuBackends) {
      const gpuResult = blas.sgemm(
        Transpose.NO_TRANS, Transpose.NO_TRANS, 1.0, a, b, 0.0, c,
      );
      expectMatrixClose(gpuResult, cpuResult);
    }
  });

  it("all backends produce same SGEMV result", () => {
    const a = new Matrix([1, 2, 3, 4, 5, 6], 2, 3);
    const x = new Vector([1, 2, 3], 3);
    const y = new Vector([0, 0], 2);
    const cpuResult = cpu.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y);

    for (const [name, blas] of gpuBackends) {
      const gpuResult = blas.sgemv(Transpose.NO_TRANS, 1.0, a, x, 0.0, y);
      expectVectorClose(gpuResult, cpuResult);
    }
  });

  it("all backends produce same SGER result", () => {
    const x = new Vector([1, 2, 3], 3);
    const y = new Vector([4, 5], 2);
    const a = new Matrix([1, 1, 1, 1, 1, 1], 3, 2);
    const cpuResult = cpu.sger(2.0, x, y, a);

    for (const [name, blas] of gpuBackends) {
      const gpuResult = blas.sger(2.0, x, y, a);
      expectMatrixClose(gpuResult, cpuResult);
    }
  });
});
