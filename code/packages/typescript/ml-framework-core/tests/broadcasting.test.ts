/**
 * broadcasting.test.ts — coverage for the Phase A.1 broadcasting layer.
 * ============================================================================
 *
 * Three layers of testing:
 *
 *   1. **Helpers in isolation** — `broadcastShapes`, `broadcastDataTo`,
 *      `unbroadcastDataTo` against hand-computed expected outputs.
 *
 *   2. **Binary ops** — the existing Add/Sub/Mul/Div now accept
 *      mismatched-but-broadcastable shapes; verify forward results
 *      and backward gradient correctness (gradients must be the
 *      shape of the ORIGINAL input, with broadcast-stretched dims
 *      summed out).
 *
 *   3. **`BroadcastOp`** — explicit broadcast as an autograd op,
 *      forward + backward.
 */

import { describe, it, expect } from "vitest";
import {
  Tensor,
  AddOp,
  SubOp,
  MulOp,
  DivOp,
  BroadcastOp,
  broadcastShapes,
  broadcastDataTo,
  unbroadcastDataTo,
} from "../src/index.js";

describe("broadcastShapes — shape math only", () => {
  it("identical shapes pass through unchanged", () => {
    expect(broadcastShapes([2, 3], [2, 3])).toEqual([2, 3]);
  });

  it("scalar broadcasts to any shape (vector)", () => {
    expect(broadcastShapes([], [4])).toEqual([4]);
    expect(broadcastShapes([4], [])).toEqual([4]);
  });

  it("(3,) and (2, 3) → (2, 3)", () => {
    expect(broadcastShapes([3], [2, 3])).toEqual([2, 3]);
    expect(broadcastShapes([2, 3], [3])).toEqual([2, 3]);
  });

  it("(5, 1, 3) and (2, 3) → (5, 2, 3)", () => {
    expect(broadcastShapes([5, 1, 3], [2, 3])).toEqual([5, 2, 3]);
  });

  it("(1, 4) and (3, 4) → (3, 4)", () => {
    expect(broadcastShapes([1, 4], [3, 4])).toEqual([3, 4]);
  });

  it("(3, 1) and (1, 4) → (3, 4)  (outer-product style)", () => {
    expect(broadcastShapes([3, 1], [1, 4])).toEqual([3, 4]);
  });

  it("incompatible shapes throw RangeError", () => {
    expect(() => broadcastShapes([2, 3], [3, 2])).toThrow(RangeError);
    expect(() => broadcastShapes([2, 3], [4])).toThrow(RangeError);
    expect(() => broadcastShapes([2, 3, 4], [5, 4])).toThrow(RangeError);
  });

  it("matching zero-sized dims pass through", () => {
    expect(broadcastShapes([0, 3], [0, 3])).toEqual([0, 3]);
  });
});

describe("broadcastDataTo — materialize a broadcasted Float32Array", () => {
  it("identical shapes: just copies (defensive, owns its memory)", () => {
    const data = new Float32Array([1, 2, 3, 4, 5, 6]);
    const out = broadcastDataTo(data, [2, 3], [2, 3]);
    expect(Array.from(out)).toEqual([1, 2, 3, 4, 5, 6]);
    expect(out).not.toBe(data); // fresh allocation
  });

  it("(3,) → (2, 3) replicates the row", () => {
    const out = broadcastDataTo(new Float32Array([10, 20, 30]), [3], [2, 3]);
    expect(Array.from(out)).toEqual([10, 20, 30, 10, 20, 30]);
  });

  it("(2, 1) → (2, 3) replicates each row 3 times", () => {
    const out = broadcastDataTo(new Float32Array([1, 2]), [2, 1], [2, 3]);
    expect(Array.from(out)).toEqual([1, 1, 1, 2, 2, 2]);
  });

  it("(1, 3) → (2, 3) replicates the single row", () => {
    const out = broadcastDataTo(new Float32Array([10, 20, 30]), [1, 3], [2, 3]);
    expect(Array.from(out)).toEqual([10, 20, 30, 10, 20, 30]);
  });

  it("(3, 1) and (1, 4) → (3, 4)  (outer-product layout)", () => {
    // left input: shape (3, 1), values [a, b, c]
    const a = broadcastDataTo(new Float32Array([1, 2, 3]), [3, 1], [3, 4]);
    // Should produce: row 0 all 1s, row 1 all 2s, row 2 all 3s
    expect(Array.from(a)).toEqual([1, 1, 1, 1, 2, 2, 2, 2, 3, 3, 3, 3]);

    // right input: shape (1, 4), values [w, x, y, z]
    const b = broadcastDataTo(new Float32Array([10, 20, 30, 40]), [1, 4], [3, 4]);
    // Should produce: same row repeated 3 times
    expect(Array.from(b)).toEqual([10, 20, 30, 40, 10, 20, 30, 40, 10, 20, 30, 40]);
  });

  it("rejects incompatible target shape", () => {
    expect(() => broadcastDataTo(new Float32Array([1, 2, 3]), [3], [4])).toThrow(RangeError);
  });
});

describe("unbroadcastDataTo — sum gradient back to original shape", () => {
  it("identical shapes: just copies", () => {
    const data = new Float32Array([1, 2, 3, 4]);
    const out = unbroadcastDataTo(data, [2, 2], [2, 2]);
    expect(Array.from(out)).toEqual([1, 2, 3, 4]);
  });

  it("(2, 3) → (3,) sums along axis 0", () => {
    //  [[1, 2, 3],
    //   [4, 5, 6]]
    // sums along axis 0 → [5, 7, 9]
    const out = unbroadcastDataTo(new Float32Array([1, 2, 3, 4, 5, 6]), [2, 3], [3]);
    expect(Array.from(out)).toEqual([5, 7, 9]);
  });

  it("(2, 3) → (2, 1) sums along axis 1", () => {
    //  [[1, 2, 3], [4, 5, 6]]  → row sums [6, 15], shape (2, 1)
    const out = unbroadcastDataTo(new Float32Array([1, 2, 3, 4, 5, 6]), [2, 3], [2, 1]);
    expect(Array.from(out)).toEqual([6, 15]);
  });

  it("(3, 4) → (3, 1) sums each row to 1 column", () => {
    // input is row-major (3, 4):
    //  [[1, 2, 3, 4],
    //   [5, 6, 7, 8],
    //   [9, 10, 11, 12]]
    // row sums → [10, 26, 42]
    const out = unbroadcastDataTo(
      new Float32Array([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12]),
      [3, 4],
      [3, 1],
    );
    expect(Array.from(out)).toEqual([10, 26, 42]);
  });

  it("(5, 2, 3) → (2, 3) sums the leading axis", () => {
    // 30 elements; sums of slabs [0..6), [6..12), [12..18), [18..24), [24..30)
    const data = new Float32Array(30);
    for (let i = 0; i < 30; i++) data[i] = i + 1;
    const out = unbroadcastDataTo(data, [5, 2, 3], [2, 3]);
    // For each (i, j), sum over k of data[k*6 + i*3 + j]
    // k=0..4: i=0,j=0 → 1+7+13+19+25 = 65; i=0,j=1 → 2+8+14+20+26=70; etc.
    expect(Array.from(out)).toEqual([65, 70, 75, 80, 85, 90]);
  });
});

describe("Binary ops accept broadcastable shapes", () => {
  it("Add: (2, 3) + (3,) broadcasts correctly", () => {
    const a = new Tensor([[1, 2, 3], [4, 5, 6]]);
    const b = new Tensor([10, 20, 30]);
    const c = AddOp.apply(a, b);
    expect(c.shape).toEqual([2, 3]);
    expect(c.toArray()).toEqual([11, 22, 33, 14, 25, 36]);
  });

  it("Sub: (2, 1) - (1, 3) broadcasts to (2, 3)", () => {
    const a = new Tensor([[10], [20]]); // (2, 1)
    const b = new Tensor([[1, 2, 3]]);   // (1, 3)
    const c = SubOp.apply(a, b);
    expect(c.shape).toEqual([2, 3]);
    expect(c.toArray()).toEqual([9, 8, 7, 19, 18, 17]);
  });

  it("Mul: (3, 1) * (1, 4) is an outer product", () => {
    const a = new Tensor([[1], [2], [3]]); // (3, 1)
    const b = new Tensor([[10, 20, 30, 40]]); // (1, 4)
    const c = MulOp.apply(a, b);
    expect(c.shape).toEqual([3, 4]);
    expect(c.toArray()).toEqual([10, 20, 30, 40, 20, 40, 60, 80, 30, 60, 90, 120]);
  });

  it("Div: (2, 3) / (3,) broadcasts correctly", () => {
    const a = new Tensor([[10, 20, 30], [40, 50, 60]]);
    const b = new Tensor([1, 2, 3]);
    const c = DivOp.apply(a, b);
    expect(c.shape).toEqual([2, 3]);
    expect(c.toArray()).toEqual([10, 10, 10, 40, 25, 20]);
  });

  it("incompatible shapes still throw", () => {
    expect(() => AddOp.apply(new Tensor([[1, 2, 3]]), new Tensor([[1, 2]]))).toThrow(RangeError);
  });
});

describe("Binary op backward unbroadcasts gradient to original input shape", () => {
  it("Add (2, 3) + (3,) backward: bias grad is summed along batch axis", () => {
    const a = new Tensor([[1, 2, 3], [4, 5, 6]]); a.requiresGrad = true;
    const b = new Tensor([10, 20, 30]); b.requiresGrad = true;
    AddOp.apply(a, b).backward();
    // d/dA = ones(2, 3); d/dB = sum(ones, axis=0) = (2, 2, 2)
    expect(a.grad!.shape).toEqual([2, 3]);
    expect(a.grad!.toArray()).toEqual([1, 1, 1, 1, 1, 1]);
    expect(b.grad!.shape).toEqual([3]);
    expect(b.grad!.toArray()).toEqual([2, 2, 2]);
  });

  it("Mul (2, 1) * (1, 3) outer-product backward", () => {
    const a = new Tensor([[2], [3]]); a.requiresGrad = true;        // (2, 1)
    const b = new Tensor([[10, 20, 30]]); b.requiresGrad = true;    // (1, 3)
    MulOp.apply(a, b).backward();
    // d/dA on broadcast shape = b broadcast → sum along axis 1 → (2, 1):
    //   row 0: 10+20+30 = 60; row 1: same = 60
    expect(a.grad!.shape).toEqual([2, 1]);
    expect(a.grad!.toArray()).toEqual([60, 60]);
    // d/dB on broadcast shape = a broadcast → sum along axis 0 → (1, 3):
    //   col 0: 2+3 = 5; col 1: same; col 2: same
    expect(b.grad!.shape).toEqual([1, 3]);
    expect(b.grad!.toArray()).toEqual([5, 5, 5]);
  });

  it("Sub (3,) - (2, 3) backward signs are correct", () => {
    const a = new Tensor([1, 2, 3]); a.requiresGrad = true;
    const b = new Tensor([[10, 20, 30], [40, 50, 60]]); b.requiresGrad = true;
    SubOp.apply(a, b).backward();
    // dL/dA = ones(2, 3) → unbroadcast to (3,) = sum along axis 0 = (2, 2, 2)
    expect(a.grad!.toArray()).toEqual([2, 2, 2]);
    // dL/dB = -ones(2, 3) → already (2, 3), no unbroadcasting
    expect(b.grad!.toArray()).toEqual([-1, -1, -1, -1, -1, -1]);
  });
});

describe("BroadcastOp — explicit broadcast as autograd op", () => {
  it("forward broadcasts (3,) → (2, 3)", () => {
    const x = new Tensor([10, 20, 30]);
    const y = BroadcastOp.apply(x, [2, 3]);
    expect(y.shape).toEqual([2, 3]);
    expect(y.toArray()).toEqual([10, 20, 30, 10, 20, 30]);
  });

  it("backward sums gradient back to input shape", () => {
    const x = new Tensor([10, 20, 30]); x.requiresGrad = true;
    const y = BroadcastOp.apply(x, [2, 3]);
    y.backward();
    // gradient with default seed = ones(2, 3); sum along axis 0 → (2, 2, 2)
    expect(x.grad!.shape).toEqual([3]);
    expect(x.grad!.toArray()).toEqual([2, 2, 2]);
  });

  it("backward with explicit seed grad respects the gradient values", () => {
    const x = new Tensor([1, 2]); x.requiresGrad = true;     // (2,)
    const y = BroadcastOp.apply(x, [3, 2]);                  // (3, 2)
    // seed grad = [[1, 10], [100, 1000], [10000, 100000]]
    const seed = new Tensor([[1, 10], [100, 1000], [10000, 100000]]);
    y.backward(seed);
    // x.grad = column sums of seed = [10101, 101010]
    expect(x.grad!.toArray()).toEqual([10101, 101010]);
  });

  it("can chain through binary ops naturally", () => {
    // y = broadcast(bias, (2, 3)) + x
    const bias = new Tensor([0.5, 0.5, 0.5]); bias.requiresGrad = true;
    const x = new Tensor([[1, 2, 3], [4, 5, 6]]);
    const b = BroadcastOp.apply(bias, [2, 3]);
    const y = AddOp.apply(b, x);
    y.backward();
    // dL/dBias = sum(ones(2, 3), axis=0) = (2, 2, 2)
    expect(bias.grad!.toArray()).toEqual([2, 2, 2]);
  });
});
