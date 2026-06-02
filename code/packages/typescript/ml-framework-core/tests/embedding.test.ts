/**
 * embedding.test.ts — EmbeddingOp lookup-table coverage.
 *
 * Added in v1.3 (Phase A.3).  Exercises:
 *   - Forward shape: (vocab, dim) + indices (B,) → (B, dim);
 *                    (vocab, dim) + indices (B, S) → (B, S, dim).
 *   - Forward numerical correctness on hand-computed lookups.
 *   - Backward shape: grad-weight always has weight shape.
 *   - Backward scatter-add for REPEATED indices (the key correctness
 *     test — naive set-not-sum would silently lose gradients).
 *   - Out-of-range index validation (negative + ≥ vocab_size).
 *   - Tensor.embedding() fluent convenience method.
 *
 * Why the scatter-add test matters: nearly every real-world input has
 * repeated tokens.  A buggy embedding that overwrites instead of
 * accumulates passes the shape tests but trains to garbage on real
 * data.  The test below catches that class of bug directly.
 */

import { describe, it, expect } from "vitest";
import { Tensor, EmbeddingOp } from "../src/index.js";

describe("EmbeddingOp — forward shapes", () => {
  it("(10, 4) weight + (3,) indices → (3, 4) output", () => {
    const weight = new Tensor(
      Array.from({ length: 10 * 4 }, (_, i) => i),
      { shape: [10, 4] },
    );
    const indices = new Tensor([0, 2, 9]);
    const out = EmbeddingOp.apply(weight, indices);
    expect(out.shape).toEqual([3, 4]);
    // Row 0 = [0,1,2,3], row 2 = [8,9,10,11], row 9 = [36,37,38,39].
    expect(out.toArray()).toEqual([0, 1, 2, 3, 8, 9, 10, 11, 36, 37, 38, 39]);
  });

  it("(10, 4) weight + (2, 5) indices → (2, 5, 4) output", () => {
    const weight = new Tensor(
      Array.from({ length: 10 * 4 }, (_, i) => i),
      { shape: [10, 4] },
    );
    const indices = new Tensor([0, 1, 2, 3, 4, 5, 6, 7, 8, 9], { shape: [2, 5] });
    const out = EmbeddingOp.apply(weight, indices);
    expect(out.shape).toEqual([2, 5, 4]);
    // Position (0, 0) = row 0 = [0,1,2,3]; position (1, 4) = row 9 = [36,37,38,39].
    expect(out.toArray().slice(0, 4)).toEqual([0, 1, 2, 3]);
    expect(out.toArray().slice(-4)).toEqual([36, 37, 38, 39]);
  });

  it("(vocab=5, dim=3) + scalar indices (shape []) → shape [3]", () => {
    const weight = new Tensor(
      Array.from({ length: 5 * 3 }, (_, i) => i),
      { shape: [5, 3] },
    );
    const indices = new Tensor([2], { shape: [] }); // 0-D
    const out = EmbeddingOp.apply(weight, indices);
    expect(out.shape).toEqual([3]);
    // Row 2 = [6, 7, 8].
    expect(out.toArray()).toEqual([6, 7, 8]);
  });
});

describe("EmbeddingOp — backward", () => {
  it("grad-weight shape matches weight shape", () => {
    const weight = new Tensor(
      Array.from({ length: 8 * 5 }, (_, i) => i * 0.1),
      { shape: [8, 5] },
    );
    weight.requiresGrad = true;
    const indices = new Tensor([0, 3, 7]);
    const out = EmbeddingOp.apply(weight, indices);
    out.backward();
    expect(weight.grad?.shape).toEqual([8, 5]);
  });

  it("scatter-add: REPEATED indices accumulate gradients (does NOT overwrite)", () => {
    // The killer correctness test.  weight (5, 2), indices [1, 1, 3].
    // Output shape (3, 2).  Default backward seeds grad with ones — shape (3, 2).
    // Expected grad-weight:
    //   row 0: [0, 0]          ← no index pointed here
    //   row 1: [2, 2]          ← TWO occurrences (positions 0 and 1) each contributing [1, 1]
    //   row 2: [0, 0]
    //   row 3: [1, 1]          ← one occurrence (position 2) contributing [1, 1]
    //   row 4: [0, 0]
    const weight = new Tensor(
      Array.from({ length: 5 * 2 }, () => 0),
      { shape: [5, 2] },
    );
    weight.requiresGrad = true;
    const indices = new Tensor([1, 1, 3]);
    const out = EmbeddingOp.apply(weight, indices);
    out.backward(); // ones seed of shape (3, 2)
    expect(weight.grad?.shape).toEqual([5, 2]);
    expect(Array.from(weight.grad!.data)).toEqual([
      0, 0,    // row 0
      2, 2,    // row 1: 2 contributions
      0, 0,    // row 2
      1, 1,    // row 3
      0, 0,    // row 4
    ]);
  });

  it("scatter-add with explicit non-ones gradient", () => {
    // weight (4, 2), indices [0, 0, 2] with grad = [[1,2],[3,4],[5,6]].
    // Expected grad-weight:
    //   row 0: [1+3, 2+4] = [4, 6]
    //   row 1: [0, 0]
    //   row 2: [5, 6]
    //   row 3: [0, 0]
    const weight = new Tensor(
      Array.from({ length: 4 * 2 }, () => 0),
      { shape: [4, 2] },
    );
    weight.requiresGrad = true;
    const indices = new Tensor([0, 0, 2]);
    const out = EmbeddingOp.apply(weight, indices);
    const customGrad = new Tensor([1, 2, 3, 4, 5, 6], { shape: [3, 2] });
    out.backward(customGrad);
    expect(Array.from(weight.grad!.data)).toEqual([
      4, 6,
      0, 0,
      5, 6,
      0, 0,
    ]);
  });

  it("indices receive no gradient (parent grad is null)", () => {
    // Both tensors are parents of the op, but indices is non-differentiable.
    // Verify that flagging indices with requiresGrad doesn't trigger a backward
    // crash — backward simply returns null for that parent.
    const weight = new Tensor([0, 0, 0, 0, 0, 0], { shape: [3, 2] });
    weight.requiresGrad = true;
    const indices = new Tensor([0, 1]);
    indices.requiresGrad = true; // user error, but should not crash
    const out = EmbeddingOp.apply(weight, indices);
    expect(() => out.backward()).not.toThrow();
    expect(indices.grad).toBeNull();
    expect(weight.grad?.shape).toEqual([3, 2]);
  });
});

describe("EmbeddingOp — validation", () => {
  it("rejects non-2-D weight", () => {
    expect(() =>
      EmbeddingOp.apply(new Tensor([1, 2, 3]), new Tensor([0])),
    ).toThrow(RangeError);
    expect(() =>
      EmbeddingOp.apply(Tensor.zeros(2, 3, 4), new Tensor([0])),
    ).toThrow(RangeError);
  });

  it("rejects negative index", () => {
    const weight = Tensor.zeros(5, 3);
    expect(() => EmbeddingOp.apply(weight, new Tensor([0, -1]))).toThrow(RangeError);
  });

  it("rejects index >= vocab_size", () => {
    const weight = Tensor.zeros(5, 3);
    expect(() => EmbeddingOp.apply(weight, new Tensor([5]))).toThrow(RangeError);
    expect(() => EmbeddingOp.apply(weight, new Tensor([100]))).toThrow(RangeError);
  });

  it("accepts boundary indices 0 and vocab_size - 1", () => {
    const weight = Tensor.zeros(5, 3);
    expect(() => EmbeddingOp.apply(weight, new Tensor([0]))).not.toThrow();
    expect(() => EmbeddingOp.apply(weight, new Tensor([4]))).not.toThrow();
  });
});

describe("Tensor.embedding() fluent method", () => {
  it("weight.embedding(indices) matches EmbeddingOp.apply(weight, indices)", () => {
    const weight = new Tensor(
      Array.from({ length: 4 * 2 }, (_, i) => i),
      { shape: [4, 2] },
    );
    const indices = new Tensor([0, 2, 3]);
    const a = weight.embedding(indices);
    const b = EmbeddingOp.apply(weight, indices);
    expect(a.shape).toEqual(b.shape);
    expect(a.toArray()).toEqual(b.toArray());
  });
});
