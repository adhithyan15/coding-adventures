/**
 * nn-optim.test.ts — Optimizer + Module/Linear/Sequential (Phase A.6).
 *
 * What's covered:
 *  - Optimizer base: zeroGrad clears every param.grad; constructor
 *    rejects empty params.
 *  - SGD: a single step on a quadratic loss moves the param toward
 *    the minimum.
 *  - Adam: same quadratic converges; bias-correction at t=1 gives
 *    an update with magnitude ≈ lr (NOT lr*(1-β1) which would be the
 *    uncorrected naive form).
 *  - Linear: shape, parameters() list with/without bias, forward
 *    output shape, gradient flow back into weight and bias.
 *  - Sequential: forward chains in order; parameters() collects from
 *    all children; integration test trains a tiny 2-layer MLP on a
 *    handcrafted regression to verify loss actually decreases.
 *  - Fn wrapper: no params, identity-ish forward.
 */

import { describe, it, expect } from "vitest";
import { Tensor, SGD, Adam, Linear, Sequential, Fn, Optimizer } from "../src/index.js";

/** Helper: create a leaf tensor with requiresGrad. */
function param(data: number[], shape?: number[]): Tensor {
  const t = shape ? new Tensor(data, { shape }) : new Tensor(data);
  t.requiresGrad = true;
  return t;
}

describe("Optimizer base", () => {
  it("rejects empty params list", () => {
    expect(() => new SGD([], 0.1)).toThrow(RangeError);
  });

  it("zeroGrad sets every param.grad to null", () => {
    const a = param([1, 2, 3]);
    const b = param([4, 5, 6]);
    a.grad = new Tensor([0.1, 0.2, 0.3]);
    b.grad = new Tensor([0.4, 0.5, 0.6]);
    const opt = new SGD([a, b], 0.1);
    opt.zeroGrad();
    expect(a.grad).toBeNull();
    expect(b.grad).toBeNull();
  });
});

describe("SGD", () => {
  it("step moves a param toward its quadratic minimum", () => {
    // Loss = (x - 5)² → dL/dx = 2*(x - 5).  Starting at x=0, gradient = -10.
    // After one SGD step with lr=0.1: x = 0 - 0.1*(-10) = 1.  Closer to 5.
    const x = param([0]);
    x.grad = new Tensor([-10]);
    const opt = new SGD([x], 0.1);
    opt.step();
    expect(x.data[0]).toBeCloseTo(1.0, 6);
  });

  it("skips params with null grad", () => {
    const a = param([1, 2, 3]);
    const b = param([4, 5, 6]);
    // a has grad; b does not.
    a.grad = new Tensor([1, 1, 1]);
    const opt = new SGD([a, b], 0.5);
    opt.step();
    expect(Array.from(a.data)).toEqual([0.5, 1.5, 2.5]);
    expect(Array.from(b.data)).toEqual([4, 5, 6]); // unchanged
  });

  it("rejects non-positive lr", () => {
    const a = param([1]);
    expect(() => new SGD([a], 0)).toThrow(RangeError);
    expect(() => new SGD([a], -0.1)).toThrow(RangeError);
  });
});

describe("Adam", () => {
  it("step 1 with constant grad: |update| ≈ lr (bias-corrected, NOT lr*(1-β1))", () => {
    // Verify bias correction is applied at t=1.
    // grad = 1 constant; lr=0.01.  Without bias correction the first
    // step would be ~lr*(1-β1)/sqrt(1-β2)/(...), which is much smaller.
    // With bias correction: m̂=g, v̂=g², so update = lr * g / (|g| + eps) ≈ lr.
    const x = param([0]);
    x.grad = new Tensor([1]);
    const opt = new Adam([x], 0.01);
    const before = x.data[0]!;
    opt.step();
    const delta = before - x.data[0]!;
    // delta should be ≈ lr = 0.01, NOT lr*(1-0.9) = 0.001.
    expect(delta).toBeCloseTo(0.01, 4);
  });

  it("step counter increments", () => {
    const x = param([0]);
    x.grad = new Tensor([1]);
    const opt = new Adam([x]);
    expect(opt.stepCount).toBe(0);
    opt.step();
    expect(opt.stepCount).toBe(1);
    opt.step();
    expect(opt.stepCount).toBe(2);
  });

  it("rejects out-of-range hyperparams", () => {
    const a = param([1]);
    expect(() => new Adam([a], -0.1)).toThrow(RangeError);
    expect(() => new Adam([a], 0.001, 1.0)).toThrow(RangeError); // beta1 must be < 1
    expect(() => new Adam([a], 0.001, 0.9, -0.1)).toThrow(RangeError);
  });

  it("converges on a quadratic loss", () => {
    // L = (x - 7)². Use analytical grad to skip building an autograd subgraph.
    const x = param([0]);
    const opt = new Adam([x], 0.1);
    for (let i = 0; i < 200; i++) {
      x.grad = new Tensor([2 * (x.data[0]! - 7)]);
      opt.step();
    }
    expect(x.data[0]).toBeCloseTo(7, 1);
  });
});

describe("Linear", () => {
  it("shape: weight (in, out), bias (out,)", () => {
    const lin = new Linear(3, 5);
    expect(lin.weight.shape).toEqual([3, 5]);
    expect(lin.bias?.shape).toEqual([5]);
  });

  it("parameters() returns [weight, bias] when bias=true", () => {
    const lin = new Linear(3, 5);
    const ps = lin.parameters();
    expect(ps.length).toBe(2);
    expect(ps[0]).toBe(lin.weight);
    expect(ps[1]).toBe(lin.bias);
  });

  it("parameters() returns [weight] only when bias=false", () => {
    const lin = new Linear(3, 5, false);
    const ps = lin.parameters();
    expect(ps.length).toBe(1);
    expect(ps[0]).toBe(lin.weight);
    expect(lin.bias).toBeNull();
  });

  it("forward shape: (batch, in) → (batch, out)", () => {
    const lin = new Linear(4, 7);
    const x = Tensor.zeros(8, 4);
    const y = lin.forward(x);
    expect(y.shape).toEqual([8, 7]);
  });

  it("forward produces gradient flow back to weight + bias", () => {
    const lin = new Linear(3, 2);
    const x = Tensor.zeros(5, 3); // doesn't require grad
    const y = lin.forward(x).sum();
    y.backward();
    expect(lin.weight.grad).not.toBeNull();
    expect(lin.bias?.grad).not.toBeNull();
    expect(lin.weight.grad?.shape).toEqual([3, 2]);
    expect(lin.bias?.grad?.shape).toEqual([2]);
  });

  it("rejects non-positive in/out", () => {
    expect(() => new Linear(0, 5)).toThrow(RangeError);
    expect(() => new Linear(5, 0)).toThrow(RangeError);
  });
});

describe("Sequential", () => {
  it("forward chains layers in order", () => {
    // Two Linear layers: 3 → 4 → 2.  Output shape should be (batch, 2).
    const seq = new Sequential(new Linear(3, 4), new Linear(4, 2));
    const x = Tensor.zeros(6, 3);
    expect(seq.forward(x).shape).toEqual([6, 2]);
  });

  it("parameters() concatenates all children's parameters in declaration order", () => {
    const l1 = new Linear(2, 3);  // 2 params
    const l2 = new Linear(3, 1, false); // 1 param
    const seq = new Sequential(l1, l2);
    const ps = seq.parameters();
    expect(ps).toEqual([l1.weight, l1.bias, l2.weight]);
  });

  it("training loop on a tiny MLP reduces loss (integration)", () => {
    // Tiny regression: learn y = sum(x).  Two-layer MLP with ReLU between.
    // Input dim 3, hidden dim 8, output dim 1.  Train on 20 random samples.
    const model = new Sequential(
      new Linear(3, 8),
      new Fn((x) => x.relu()),
      new Linear(8, 1),
    );
    const opt = new Adam(model.parameters(), 0.05);

    // Generate fixed data so the test is deterministic across runs of
    // Math.random — we precompute and hold both inputs and targets.
    const N = 20;
    const xData: number[][] = [];
    const yTarget: number[] = [];
    for (let i = 0; i < N; i++) {
      const a = (i % 5) * 0.1, b = ((i * 3) % 7) * 0.1, c = ((i * 11) % 13) * 0.1;
      xData.push([a, b, c]);
      yTarget.push(a + b + c);
    }

    const computeLoss = (): number => {
      let total = 0;
      for (let i = 0; i < N; i++) {
        const x = new Tensor(xData[i]!, { shape: [1, 3] });
        const yHat = model.forward(x); // shape (1, 1)
        const diff = yHat.data[0]! - yTarget[i]!;
        total += diff * diff;
      }
      return total / N;
    };

    const lossStart = computeLoss();

    for (let epoch = 0; epoch < 30; epoch++) {
      for (let i = 0; i < N; i++) {
        const x = new Tensor(xData[i]!, { shape: [1, 3] });
        const yTrue = new Tensor([yTarget[i]!], { shape: [1, 1] });
        const yHat = model.forward(x);
        const diff = yHat.sub(yTrue);
        const loss = diff.mul(diff).sum();
        opt.zeroGrad();
        loss.backward();
        opt.step();
      }
    }

    const lossEnd = computeLoss();
    // Initial loss is whatever Xavier-init gives; final loss must be
    // meaningfully smaller — at least a 2× reduction.
    expect(lossEnd).toBeLessThan(lossStart * 0.5);
  });
});

describe("Fn", () => {
  it("has no parameters", () => {
    const fn = new Fn((x) => x.relu());
    expect(fn.parameters()).toEqual([]);
  });

  it("applies the wrapped function in forward", () => {
    const fn = new Fn((x) => x.relu());
    const x = new Tensor([-1, 0, 1, -2, 2]);
    expect(fn.forward(x).toArray()).toEqual([0, 0, 1, 0, 2]);
  });
});

describe("Optimizer integration via type", () => {
  // Compile-time check that SGD/Adam are assignable to Optimizer base.
  it("SGD and Adam are both Optimizers", () => {
    const a = param([1]);
    a.grad = new Tensor([1]);
    const optimizers: Optimizer[] = [new SGD([a], 0.1), new Adam([a])];
    for (const o of optimizers) {
      expect(typeof o.step).toBe("function");
      expect(typeof o.zeroGrad).toBe("function");
    }
  });
});
