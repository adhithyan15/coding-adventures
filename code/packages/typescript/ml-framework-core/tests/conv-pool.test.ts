/**
 * conv-pool.test.ts — Conv2D + MaxPool2D coverage (Phase A.5).
 *
 * Conv2D is the workhorse of CNNs; we test:
 *  - Output shape formula across stride/padding configurations
 *  - 1×1 identity-conv preserves input (sanity for the im2col + matmul path)
 *  - Known-kernel hand-computed forward correctness
 *  - Backward gradient correctness via finite differences (small case)
 *  - Bias handling: with and without bias path
 *
 * MaxPool2D — sliding window max:
 *  - Output shape on non-overlapping and overlapping cases
 *  - Forward correctness on a small image (hand-computed argmax)
 *  - Backward routes grad to argmax positions; rest is zero
 *  - Overlapping windows accumulate when they share an argmax
 *
 * The finite-difference check for Conv2D backward is the strongest test —
 * it independently verifies the analytical gradient is correct.
 */

import { describe, it, expect } from "vitest";
import { Tensor, Conv2DOp, MaxPool2DOp } from "../src/index.js";

/** Helper: build a tensor from a flat array + shape. */
function t(flat: number[], shape: number[]): Tensor {
  return new Tensor(flat, { shape });
}

describe("Conv2D — forward output shape", () => {
  it("default stride=1, padding=0: outH = H - kH + 1", () => {
    const x = Tensor.zeros(1, 3, 8, 8);
    const w = Tensor.zeros(5, 3, 3, 3);
    const y = Conv2DOp.apply(x, w);
    expect(y.shape).toEqual([1, 5, 6, 6]);
  });

  it("stride=2 halves the spatial size", () => {
    const x = Tensor.zeros(2, 3, 8, 8);
    const w = Tensor.zeros(4, 3, 2, 2);
    const y = Conv2DOp.apply(x, w, null, 2, 0);
    // out = floor((8 - 2)/2) + 1 = 4
    expect(y.shape).toEqual([2, 4, 4, 4]);
  });

  it("padding=1 with 3×3 preserves spatial size (the 'same' padding trick)", () => {
    const x = Tensor.zeros(1, 2, 5, 5);
    const w = Tensor.zeros(3, 2, 3, 3);
    const y = Conv2DOp.apply(x, w, null, 1, 1);
    expect(y.shape).toEqual([1, 3, 5, 5]);
  });

  it("rejects kernel larger than padded input", () => {
    const x = Tensor.zeros(1, 1, 3, 3);
    const w = Tensor.zeros(1, 1, 5, 5);
    expect(() => Conv2DOp.apply(x, w)).toThrow(RangeError);
  });

  it("rejects in-channel mismatch", () => {
    const x = Tensor.zeros(1, 3, 5, 5);
    const w = Tensor.zeros(2, 4 /* wrong */, 3, 3); // weight expects C=4, x has 3
    expect(() => Conv2DOp.apply(x, w)).toThrow(RangeError);
  });
});

describe("Conv2D — forward correctness", () => {
  it("1×1 identity kernel preserves input (single channel)", () => {
    // x: (1, 1, 3, 3) = arange(9); weight: (1, 1, 1, 1) = [1] → output = x.
    const x = t([0, 1, 2, 3, 4, 5, 6, 7, 8], [1, 1, 3, 3]);
    const w = t([1], [1, 1, 1, 1]);
    const y = Conv2DOp.apply(x, w);
    expect(y.shape).toEqual([1, 1, 3, 3]);
    expect(y.toArray()).toEqual([0, 1, 2, 3, 4, 5, 6, 7, 8]);
  });

  it("3×3 known kernel hand-computed", () => {
    // x = 3x3 identity-ish: [[1,2,3],[4,5,6],[7,8,9]], all-ones kernel.
    // Output is a single value: sum(x) = 45.
    const x = t([1, 2, 3, 4, 5, 6, 7, 8, 9], [1, 1, 3, 3]);
    const w = t([1, 1, 1, 1, 1, 1, 1, 1, 1], [1, 1, 3, 3]);
    const y = Conv2DOp.apply(x, w);
    expect(y.shape).toEqual([1, 1, 1, 1]);
    expect(y.toArray()).toEqual([45]);
  });

  it("bias adds scalar per output channel", () => {
    const x = Tensor.zeros(1, 1, 2, 2);
    const w = Tensor.zeros(3, 1, 1, 1);
    const b = t([10, 20, 30], [3]);
    const y = Conv2DOp.apply(x, w, b);
    expect(y.shape).toEqual([1, 3, 2, 2]);
    // Channel 0 all 10s, channel 1 all 20s, channel 2 all 30s.
    const flat = y.toArray();
    expect(flat.slice(0, 4)).toEqual([10, 10, 10, 10]);
    expect(flat.slice(4, 8)).toEqual([20, 20, 20, 20]);
    expect(flat.slice(8, 12)).toEqual([30, 30, 30, 30]);
  });
});

describe("Conv2D — backward shapes and correctness", () => {
  it("returns grads of correct shape for x, w, (bias)", () => {
    const x = Tensor.zeros(2, 3, 5, 5); x.requiresGrad = true;
    const w = Tensor.zeros(4, 3, 3, 3); w.requiresGrad = true;
    const b = Tensor.zeros(4); b.requiresGrad = true;
    const y = Conv2DOp.apply(x, w, b);
    y.backward();
    expect(x.grad?.shape).toEqual([2, 3, 5, 5]);
    expect(w.grad?.shape).toEqual([4, 3, 3, 3]);
    expect(b.grad?.shape).toEqual([4]);
  });

  it("bias gradient = sum of grad over (N, outH, outW) per output channel", () => {
    const x = Tensor.zeros(2, 1, 4, 4); x.requiresGrad = true;
    const w = Tensor.zeros(3, 1, 2, 2);
    const b = Tensor.zeros(3); b.requiresGrad = true;
    const y = Conv2DOp.apply(x, w, b); // shape (2, 3, 3, 3) with all-zero kernel
    // grad seed = ones-like, shape (2, 3, 3, 3).  Sum over (N=2, outH=3, outW=3) = 2*9 = 18 per channel.
    y.backward();
    expect(Array.from(b.grad!.data)).toEqual([18, 18, 18]);
  });

  it("backward matches finite-difference gradient (small case)", () => {
    // Tiny case so the O(numel) per-cell perturbation finishes quickly.
    // x: (1, 2, 3, 3); w: (2, 2, 2, 2); b: (2)
    const N = 1, C = 2, H = 3, W = 3, outC = 2, kH = 2, kW = 2;
    const xData = Array.from({ length: N * C * H * W }, (_, i) => 0.1 + 0.07 * i);
    const wData = Array.from({ length: outC * C * kH * kW }, (_, i) => -0.2 + 0.05 * i);

    // Define loss = sum(out) so dL/dout = ones-like.
    const lossOf = (xd: number[], wd: number[]): number => {
      const xx = t(xd, [N, C, H, W]);
      const ww = t(wd, [outC, C, kH, kW]);
      const out = Conv2DOp.apply(xx, ww);
      let s = 0;
      for (let i = 0; i < out.numel; i++) s += out.data[i]!;
      return s;
    };

    // Analytical backward.
    const x = t(xData, [N, C, H, W]); x.requiresGrad = true;
    const w = t(wData, [outC, C, kH, kW]); w.requiresGrad = true;
    const out = Conv2DOp.apply(x, w);
    out.backward();
    const dxA = Array.from(x.grad!.data);
    const dwA = Array.from(w.grad!.data);

    // Finite-difference dL/dx with central differences, step h.
    const h = 1e-3;
    for (let i = 0; i < xData.length; i++) {
      const xp = xData.slice(); xp[i]! += h;
      const xm = xData.slice(); xm[i]! -= h;
      const fd = (lossOf(xp, wData) - lossOf(xm, wData)) / (2 * h);
      expect(Math.abs(dxA[i]! - fd)).toBeLessThan(1e-2); // generous tolerance for f32
    }
    for (let i = 0; i < wData.length; i++) {
      const wp = wData.slice(); wp[i]! += h;
      const wm = wData.slice(); wm[i]! -= h;
      const fd = (lossOf(xData, wp) - lossOf(xData, wm)) / (2 * h);
      expect(Math.abs(dwA[i]! - fd)).toBeLessThan(1e-2);
    }
  });
});

describe("MaxPool2D — forward", () => {
  it("output shape: floor((H - kH)/stride) + 1", () => {
    const x = Tensor.zeros(1, 3, 8, 8);
    const y = MaxPool2DOp.apply(x, 2, 2);
    expect(y.shape).toEqual([1, 3, 4, 4]); // default stride = kH = 2
  });

  it("overlapping windows (stride < kernel)", () => {
    const x = Tensor.zeros(1, 1, 4, 4);
    const y = MaxPool2DOp.apply(x, 3, 3, 1);
    // outH = (4 - 3)/1 + 1 = 2
    expect(y.shape).toEqual([1, 1, 2, 2]);
  });

  it("picks max in each window — hand-computed", () => {
    // 4x4 image; 2x2 non-overlap.  Top-left window max = 5, top-right = 7, etc.
    const x = t(
      [
        1, 2,  3, 4,
        5, 6,  7, 8,
        9, 10, 11, 12,
        13, 14, 15, 16,
      ],
      [1, 1, 4, 4],
    );
    const y = MaxPool2DOp.apply(x, 2, 2);
    expect(y.toArray()).toEqual([6, 8, 14, 16]);
  });
});

describe("MaxPool2D — backward", () => {
  it("routes grad ONLY to argmax positions; rest zero", () => {
    const x = t(
      [
        1, 2, 3, 4,
        5, 6, 7, 8,
        9, 10, 11, 12,
        13, 14, 15, 16,
      ],
      [1, 1, 4, 4],
    );
    x.requiresGrad = true;
    const y = MaxPool2DOp.apply(x, 2, 2);
    // grad seed = ones-like (shape (1,1,2,2)) → each output cell gets grad 1.
    y.backward();
    // Argmax positions in flat (1,1,4,4): 5 (val 6), 7 (val 8), 13 (val 14), 15 (val 16).
    const dx = Array.from(x.grad!.data);
    const expected = [
      0, 0, 0, 0,
      0, 1, 0, 1,
      0, 0, 0, 0,
      0, 1, 0, 1,
    ];
    expect(dx).toEqual(expected);
  });

  it("overlapping windows: same argmax accumulates grad", () => {
    // 3x3 input, 2x2 kernel, stride 1 → output (2, 2).
    // Construct a peak: input[1,1] = 100; everything else 1.
    const x = t([1, 1, 1, 1, 100, 1, 1, 1, 1], [1, 1, 3, 3]);
    x.requiresGrad = true;
    const y = MaxPool2DOp.apply(x, 2, 2, 1);
    expect(y.shape).toEqual([1, 1, 2, 2]);
    // All 4 output cells argmax to the same position (1, 1) = flat index 4.
    y.backward();
    const dx = Array.from(x.grad!.data);
    expect(dx[4]).toBe(4); // accumulated 4 times
    // Everything else 0.
    expect(dx.filter((_, i) => i !== 4)).toEqual([0, 0, 0, 0, 0, 0, 0, 0]);
  });
});

describe("Tensor convenience methods", () => {
  it("x.conv2d(w, b) matches Conv2DOp.apply(x, w, b)", () => {
    const x = t([0, 1, 2, 3, 4, 5, 6, 7, 8], [1, 1, 3, 3]);
    const w = t([1], [1, 1, 1, 1]);
    const a = x.conv2d(w);
    const b = Conv2DOp.apply(x, w);
    expect(a.toArray()).toEqual(b.toArray());
  });

  it("x.maxPool2d(kH, kW) matches MaxPool2DOp.apply(x, kH, kW)", () => {
    const x = t([1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16], [1, 1, 4, 4]);
    const a = x.maxPool2d(2, 2);
    const b = MaxPool2DOp.apply(x, 2, 2);
    expect(a.toArray()).toEqual(b.toArray());
  });
});
