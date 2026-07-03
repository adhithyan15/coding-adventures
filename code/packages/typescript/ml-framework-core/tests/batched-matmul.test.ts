/**
 * batched-matmul.test.ts — N-D batched MatMul forward + backward.
 *
 * Added in v1.2 (Phase A.2).  Exercises:
 *  - Forward: equal-batch, broadcast-right, broadcast-left, multi-batch,
 *    multi-batch with broadcasting.
 *  - Numerical correctness against hand-computed reference values.
 *  - Backward: per-batch slice formulas, and gradient unbroadcast to the
 *    original (broadcast) parent shape so shared operands get summed grads.
 *  - Shape validation: rank ≥ 2 enforced, inner-dim mismatch rejected,
 *    incompatible batch shapes rejected.
 *
 * Reference for hand-computed values: numpy.matmul on identical inputs.
 */

import { describe, it, expect } from "vitest";
import { Tensor, MatMulOp } from "../src/index.js";

/** Helper: build a 2-D tensor by enumerating rows × cols starting from `start`. */
function make2D(rows: number, cols: number, start: number = 0): Tensor {
  return new Tensor(
    Array.from({ length: rows * cols }, (_, i) => start + i),
    { shape: [rows, cols] },
  );
}

/** Build a tensor and flip `requiresGrad` — Tensor options don't accept it. */
function withGrad(data: number[], shape: number[]): Tensor {
  const t = new Tensor(data, { shape });
  t.requiresGrad = true;
  return t;
}

describe("Batched MatMul — forward shapes", () => {
  it("(B, M, K) @ (B, K, N) → (B, M, N)  equal batch", () => {
    // Batch 2 of (2, 3) @ (3, 4) → (2, 2, 4).
    const a = new Tensor(Array.from({ length: 2 * 2 * 3 }, (_, i) => i), {
      shape: [2, 2, 3],
    });
    const b = new Tensor(Array.from({ length: 2 * 3 * 4 }, (_, i) => i), {
      shape: [2, 3, 4],
    });
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([2, 2, 4]);

    // Hand-check batch 0: a_b0 = [[0,1,2],[3,4,5]],  b_b0 = [[0,1,2,3],[4,5,6,7],[8,9,10,11]].
    //   row 0 col 0 = 0*0 + 1*4 + 2*8 = 20
    //   row 0 col 3 = 0*3 + 1*7 + 2*11 = 29
    //   row 1 col 0 = 3*0 + 4*4 + 5*8 = 56
    //   row 1 col 3 = 3*3 + 4*7 + 5*11 = 92
    const flat = out.toArray();
    expect(flat[0]).toBe(20);
    expect(flat[3]).toBe(29);
    expect(flat[4]).toBe(56);
    expect(flat[7]).toBe(92);

    // Batch 1: a_b1 starts at 6, b_b1 starts at 12.
    //   a_b1 = [[6,7,8],[9,10,11]], b_b1 = [[12,13,14,15],[16,17,18,19],[20,21,22,23]]
    //   row 0 col 0 = 6*12 + 7*16 + 8*20 = 72 + 112 + 160 = 344
    expect(flat[8]).toBe(344);
  });

  it("(B, M, K) @ (K, N) → (B, M, N)  broadcasts right operand", () => {
    // Batch 3 on the left; single matrix on the right.
    const a = new Tensor(Array.from({ length: 3 * 2 * 2 }, (_, i) => i), {
      shape: [3, 2, 2],
    });
    const b = make2D(2, 2); // [[0,1],[2,3]]
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([3, 2, 2]);

    // Batch 0: [[0,1],[2,3]] @ [[0,1],[2,3]] = [[2, 3], [6, 11]]
    expect(out.toArray().slice(0, 4)).toEqual([2, 3, 6, 11]);
  });

  it("(M, K) @ (B, K, N) → (B, M, N)  broadcasts left operand", () => {
    const a = make2D(2, 2); // [[0,1],[2,3]]
    const b = new Tensor(Array.from({ length: 3 * 2 * 2 }, (_, i) => i), {
      shape: [3, 2, 2],
    });
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([3, 2, 2]);

    // Batch 0: [[0,1],[2,3]] @ [[0,1],[2,3]] = [[2,3],[6,11]]
    expect(out.toArray().slice(0, 4)).toEqual([2, 3, 6, 11]);
  });

  it("(B1, B2, M, K) @ (B1, B2, K, N) → (B1, B2, M, N)  multi-batch", () => {
    const a = new Tensor(
      Array.from({ length: 2 * 3 * 2 * 2 }, (_, i) => i),
      { shape: [2, 3, 2, 2] },
    );
    const b = new Tensor(
      Array.from({ length: 2 * 3 * 2 * 2 }, (_, i) => i),
      { shape: [2, 3, 2, 2] },
    );
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([2, 3, 2, 2]);
    // First slice (b1=0, b2=0): [[0,1],[2,3]] @ [[0,1],[2,3]] = [[2,3],[6,11]]
    expect(out.toArray().slice(0, 4)).toEqual([2, 3, 6, 11]);
  });

  it("(B1, 1, M, K) @ (1, B2, K, N) → (B1, B2, M, N)  multi-batch broadcast", () => {
    // B1 = 2 (left-only), B2 = 3 (right-only).
    const a = new Tensor(Array.from({ length: 2 * 1 * 2 * 2 }, (_, i) => i), {
      shape: [2, 1, 2, 2],
    });
    const b = new Tensor(Array.from({ length: 1 * 3 * 2 * 2 }, (_, i) => i), {
      shape: [1, 3, 2, 2],
    });
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([2, 3, 2, 2]);
    // (b1=0, b2=0): a slice = [[0,1],[2,3]], b slice = [[0,1],[2,3]] → [[2,3],[6,11]]
    expect(out.toArray().slice(0, 4)).toEqual([2, 3, 6, 11]);
  });
});

describe("Batched MatMul — shape validation", () => {
  it("rejects mismatched inner dim", () => {
    expect(() =>
      MatMulOp.apply(Tensor.zeros(2, 3, 4), Tensor.zeros(2, 5, 6)),
    ).toThrow(RangeError);
  });

  it("rejects incompatible batch shapes", () => {
    // (2, M, K) @ (3, K, N): batch dims 2 vs 3 can't broadcast.
    expect(() =>
      MatMulOp.apply(Tensor.zeros(2, 2, 3), Tensor.zeros(3, 3, 2)),
    ).toThrow(RangeError);
  });
});

describe("Batched MatMul — backward gradient correctness", () => {
  it("equal-batch backward returns per-slice grads of correct shape", () => {
    const a = withGrad(
      Array.from({ length: 2 * 2 * 3 }, (_, i) => i + 1),
      [2, 2, 3],
    );
    const b = withGrad(
      Array.from({ length: 2 * 3 * 2 }, (_, i) => i + 1),
      [2, 3, 2],
    );
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([2, 2, 2]);
    out.backward();
    expect(a.grad?.shape).toEqual([2, 2, 3]);
    expect(b.grad?.shape).toEqual([2, 3, 2]);
  });

  it("broadcast-right backward sums grad across batch back to (K, N)", () => {
    // (B, M, K) @ (K, N) — shared (K, N) operand should receive sum of per-slice grads.
    const a = withGrad(Array.from({ length: 3 * 2 * 2 }, () => 1), [3, 2, 2]);
    const b = withGrad([1, 0, 0, 1], [2, 2]);
    const out = MatMulOp.apply(a, b);
    out.backward(); // ones-grad of shape (3, 2, 2)
    expect(a.grad?.shape).toEqual([3, 2, 2]);
    expect(b.grad?.shape).toEqual([2, 2]);
    // For each batch slice, dL/dB_slice = A_slice^T @ grad_slice.
    // A_slice = ones(2,2), grad_slice = ones(2,2) → A^T @ grad = [[2,2],[2,2]].
    // Sum over 3 batch slices → [[6,6],[6,6]].
    expect(Array.from(b.grad!.data)).toEqual([6, 6, 6, 6]);
  });

  it("broadcast-left backward sums grad across batch back to (M, K)", () => {
    const a = withGrad([1, 0, 0, 1], [2, 2]);
    const b = withGrad(Array.from({ length: 3 * 2 * 2 }, () => 1), [3, 2, 2]);
    const out = MatMulOp.apply(a, b);
    out.backward();
    expect(a.grad?.shape).toEqual([2, 2]);
    expect(b.grad?.shape).toEqual([3, 2, 2]);
    // dL/dA_slice = grad_slice @ B_slice^T.  B_slice = ones, grad_slice = ones → [[2,2],[2,2]].
    // Summed over 3 slices → [[6,6],[6,6]].
    expect(Array.from(a.grad!.data)).toEqual([6, 6, 6, 6]);
  });
});

describe("Batched MatMul — 2-D still works (no regression)", () => {
  it("2-D matmul matches the v1.0 baseline exactly", () => {
    const a = new Tensor([[1, 2], [3, 4]]);
    const b = new Tensor([[5, 6], [7, 8]]);
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([2, 2]);
    expect(out.toArray()).toEqual([19, 22, 43, 50]);
  });

  it("2-D matmul backward matches the v1.0 baseline", () => {
    const a = new Tensor([[1, 2], [3, 4]]);
    a.requiresGrad = true;
    const b = new Tensor([[5, 6], [7, 8]]);
    b.requiresGrad = true;
    const out = MatMulOp.apply(a, b);
    out.backward();
    expect(a.grad?.shape).toEqual([2, 2]);
    expect(b.grad?.shape).toEqual([2, 2]);
    // grad ones(2,2).  dL/dA = ones @ B^T = [[11, 15], [11, 15]]; dL/dB = A^T @ ones = [[4, 4], [6, 6]].
    expect(Array.from(a.grad!.data)).toEqual([11, 15, 11, 15]);
    expect(Array.from(b.grad!.data)).toEqual([4, 4, 6, 6]);
  });
});
