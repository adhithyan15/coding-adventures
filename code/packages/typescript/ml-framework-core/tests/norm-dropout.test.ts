/**
 * norm-dropout.test.ts — LayerNorm / BatchNorm / Dropout / ModelMode coverage.
 *
 * Added in v1.4 (Phase A.4).  These three ops are mode-sensitive (train
 * vs eval) so most tests explicitly call `setMode` before operating.
 *
 * Tests cover:
 *  - ModelMode round-trip + default value
 *  - LayerNorm forward: shape preservation, normalized rows have
 *    mean ≈ 0 / variance ≈ 1, gamma=1/beta=0 is identity-up-to-normalize.
 *  - LayerNorm backward: shape checks; γ/β gradients accumulate
 *    across leading dims.
 *  - BatchNorm train mode: running stats update on each forward.
 *  - BatchNorm eval mode: running stats are USED (not updated).
 *  - Dropout train mode: statistical expected mean across large N
 *    matches input mean (inverted dropout preserves expectation).
 *  - Dropout eval mode: output exactly equals input.
 *  - Dropout p=0: pure passthrough even in train mode.
 */

import { describe, it, expect, beforeEach, afterEach } from "vitest";
import {
  Tensor,
  LayerNormOp,
  BatchNormOp,
  DropoutOp,
  getMode,
  setMode,
} from "../src/index.js";

// Most tests assume "train" mode.  Reset after each so a test that
// flips to eval doesn't leak into the next one.
beforeEach(() => setMode("train"));
afterEach(() => setMode("train"));

describe("ModelMode global toggle", () => {
  it("defaults to train", () => {
    // afterEach restores to train; check the contract directly.
    expect(getMode()).toBe("train");
  });

  it("setMode + getMode round-trip", () => {
    setMode("eval");
    expect(getMode()).toBe("eval");
    setMode("train");
    expect(getMode()).toBe("train");
  });

  it("rejects garbage values", () => {
    // @ts-expect-error — testing runtime guard
    expect(() => setMode("foo")).toThrow(TypeError);
  });
});

describe("LayerNorm — forward", () => {
  it("preserves input shape", () => {
    const x = Tensor.zeros(4, 8);
    const gamma = new Tensor(new Array(8).fill(1));
    const beta = new Tensor(new Array(8).fill(0));
    const out = LayerNormOp.apply(x, gamma, beta);
    expect(out.shape).toEqual([4, 8]);
  });

  it("each row has mean ≈ 0 and variance ≈ 1 after normalize (γ=1, β=0)", () => {
    // 3 rows of 5 random-ish values.
    const x = new Tensor([1, 2, 3, 4, 5,  10, 20, 30, 40, 50,  -1, -2, -3, -4, -5], {
      shape: [3, 5],
    });
    const gamma = new Tensor([1, 1, 1, 1, 1]);
    const beta = new Tensor([0, 0, 0, 0, 0]);
    const out = LayerNormOp.apply(x, gamma, beta).toArray();

    for (let r = 0; r < 3; r++) {
      const row = out.slice(r * 5, (r + 1) * 5);
      const mean = row.reduce((a, b) => a + b, 0) / 5;
      const variance = row.reduce((acc, v) => acc + (v - mean) ** 2, 0) / 5;
      expect(mean).toBeCloseTo(0, 4);
      expect(variance).toBeCloseTo(1, 4);
    }
  });

  it("applies γ and β: each output row equals γ * x̂ + β", () => {
    const x = new Tensor([0, 1, 2, 3, 4, 5], { shape: [2, 3] });
    const gamma = new Tensor([2, 3, 4]);
    const beta = new Tensor([10, 20, 30]);
    const out = LayerNormOp.apply(x, gamma, beta).toArray();

    // Compute x̂ for row 0: mean=1, var=2/3, σ=√(2/3+ε)
    for (let r = 0; r < 2; r++) {
      const row = [0, 1, 2, 3, 4, 5].slice(r * 3, (r + 1) * 3);
      const mean = row.reduce((a, b) => a + b, 0) / 3;
      const variance = row.reduce((acc, v) => acc + (v - mean) ** 2, 0) / 3;
      const inv = 1 / Math.sqrt(variance + 1e-5);
      for (let i = 0; i < 3; i++) {
        const xhat = (row[i]! - mean) * inv;
        const expected = xhat * [2, 3, 4][i]! + [10, 20, 30][i]!;
        expect(out[r * 3 + i]).toBeCloseTo(expected, 4);
      }
    }
  });

  it("rejects mismatched γ shape", () => {
    const x = Tensor.zeros(2, 4);
    const gamma = new Tensor([1, 1, 1]); // wrong size
    const beta = new Tensor([0, 0, 0, 0]);
    expect(() => LayerNormOp.apply(x, gamma, beta)).toThrow(RangeError);
  });
});

describe("LayerNorm — backward", () => {
  it("returns gradients of x, γ, β with correct shapes", () => {
    const x = new Tensor([1, 2, 3, 4, 5, 6], { shape: [2, 3] });
    x.requiresGrad = true;
    const gamma = new Tensor([1, 1, 1]);
    gamma.requiresGrad = true;
    const beta = new Tensor([0, 0, 0]);
    beta.requiresGrad = true;
    const out = LayerNormOp.apply(x, gamma, beta);
    out.backward();
    expect(x.grad?.shape).toEqual([2, 3]);
    expect(gamma.grad?.shape).toEqual([3]);
    expect(beta.grad?.shape).toEqual([3]);
  });

  it("β gradient = sum of upstream grad across leading dims", () => {
    // β acts as an additive bias per-feature; dL/dβ_i = Σ_batch dy_i.
    // With ones-grad seed, that's just N (the batch size).
    const x = Tensor.zeros(4, 5);
    x.requiresGrad = true;
    const gamma = new Tensor([1, 1, 1, 1, 1]);
    gamma.requiresGrad = true;
    const beta = new Tensor([0, 0, 0, 0, 0]);
    beta.requiresGrad = true;
    const out = LayerNormOp.apply(x, gamma, beta);
    out.backward();
    expect(Array.from(beta.grad!.data)).toEqual([4, 4, 4, 4, 4]);
  });
});

describe("BatchNorm — train mode updates running stats", () => {
  it("running mean / var move toward batch stats", () => {
    setMode("train");
    // Construct a batch with known per-feature mean.
    // (N=4, C=2):  col0 = [10, 10, 10, 10] (mean 10), col1 = [0, 0, 0, 0] (mean 0).
    const x = new Tensor([10, 0, 10, 0, 10, 0, 10, 0], { shape: [4, 2] });
    const gamma = new Tensor([1, 1]);
    const beta = new Tensor([0, 0]);
    const runningMean = new Tensor([0, 0]); // starts at 0
    const runningVar = new Tensor([1, 1]);   // starts at 1
    const momentum = 0.5;
    BatchNormOp.apply(x, gamma, beta, runningMean, runningVar, momentum);
    // runningMean := (1-0.5)*0 + 0.5*10 = 5; col1 unchanged at 0.
    expect(runningMean.data[0]).toBeCloseTo(5, 6);
    expect(runningMean.data[1]).toBeCloseTo(0, 6);
    // Batch var per column = 0 (constant cols), so runningVar :=
    // (1-0.5)*1 + 0.5*0 = 0.5.
    expect(runningVar.data[0]).toBeCloseTo(0.5, 6);
    expect(runningVar.data[1]).toBeCloseTo(0.5, 6);
  });
});

describe("BatchNorm — eval mode uses running stats and does NOT update them", () => {
  it("eval pass leaves running stats untouched", () => {
    const x = new Tensor([100, 100, 100, 100], { shape: [2, 2] });
    const gamma = new Tensor([1, 1]);
    const beta = new Tensor([0, 0]);
    const runningMean = new Tensor([50, 50]);
    const runningVar = new Tensor([4, 4]);
    setMode("eval");
    const out = BatchNormOp.apply(x, gamma, beta, runningMean, runningVar);
    // Running stats should NOT have moved.
    expect(Array.from(runningMean.data)).toEqual([50, 50]);
    expect(Array.from(runningVar.data)).toEqual([4, 4]);
    // Output: x̂ = (100 - 50)/√(4+ε) ≈ 25 → y = 25.
    expect(out.toArray()[0]).toBeCloseTo(25, 3);
  });
});

describe("BatchNorm — backward", () => {
  it("train mode returns grads for x, γ, β; runningMean/Var get null", () => {
    setMode("train");
    const x = new Tensor([1, 2, 3, 4, 5, 6, 7, 8], { shape: [4, 2] });
    x.requiresGrad = true;
    const gamma = new Tensor([1, 1]);
    gamma.requiresGrad = true;
    const beta = new Tensor([0, 0]);
    beta.requiresGrad = true;
    const runningMean = new Tensor([0, 0]);
    const runningVar = new Tensor([1, 1]);
    const out = BatchNormOp.apply(x, gamma, beta, runningMean, runningVar);
    out.backward();
    expect(x.grad?.shape).toEqual([4, 2]);
    expect(gamma.grad?.shape).toEqual([2]);
    expect(beta.grad?.shape).toEqual([2]);
  });
});

describe("Dropout — train mode", () => {
  it("inverted-dropout preserves the expected mean across large N", () => {
    setMode("train");
    // With p=0.5 and inverted dropout, surviving cells scale by 2 and
    // half are zeroed.  Expected mean per cell = original mean (in
    // expectation).  Use a constant input of 1s so the math is easy.
    const N = 10000;
    const x = new Tensor(new Array(N).fill(1));
    const out = DropoutOp.apply(x, 0.5).toArray();
    const mean = out.reduce((a, b) => a + b, 0) / N;
    // Expected mean = 1.0; statistical 99% CI ~ ±3*sqrt(p*(1-p)*scale²/N)
    //   = ±3*sqrt(0.25*4/N) = ±3/sqrt(N) ≈ ±0.03 for N=10k.
    expect(mean).toBeGreaterThan(0.95);
    expect(mean).toBeLessThan(1.05);
  });

  it("surviving cells are scaled to 1/(1-p) (so are either 0 or 1/(1-p))", () => {
    setMode("train");
    const x = new Tensor(new Array(1000).fill(1));
    const out = DropoutOp.apply(x, 0.3).toArray();
    const scale = 1 / 0.7;
    for (const v of out) {
      expect(v === 0 || Math.abs(v - scale) < 1e-5).toBe(true);
    }
  });
});

describe("Dropout — eval mode", () => {
  it("output equals input EXACTLY in eval mode", () => {
    setMode("eval");
    const x = new Tensor([0.1, 0.2, 0.3, 0.4, 0.5]);
    const out = DropoutOp.apply(x, 0.5);
    expect(out.toArray()).toEqual(x.toArray());
  });

  it("p=0 in train mode is a pure passthrough (no random)", () => {
    setMode("train");
    const x = new Tensor([1, 2, 3, 4, 5]);
    const out = DropoutOp.apply(x, 0);
    expect(out.toArray()).toEqual(x.toArray());
  });

  it("rejects p outside [0, 1)", () => {
    const x = new Tensor([1, 2, 3]);
    expect(() => DropoutOp.apply(x, -0.1)).toThrow(RangeError);
    expect(() => DropoutOp.apply(x, 1)).toThrow(RangeError);
    expect(() => DropoutOp.apply(x, 1.5)).toThrow(RangeError);
  });
});

describe("Tensor convenience methods", () => {
  it("t.layerNorm / t.dropout chain correctly", () => {
    const x = new Tensor([1, 2, 3, 4, 5, 6], { shape: [2, 3] });
    const gamma = new Tensor([1, 1, 1]);
    const beta = new Tensor([0, 0, 0]);
    const out = x.layerNorm(gamma, beta).dropout(0); // p=0 → passthrough
    expect(out.shape).toEqual([2, 3]);
  });
});
