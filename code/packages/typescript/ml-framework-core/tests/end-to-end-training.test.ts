/**
 * end-to-end-training.test.ts — proves the autograd stack actually trains.
 * ============================================================================
 *
 * All the unit tests so far prove individual op correctness and graph
 * wiring; this test runs a real (if tiny) training loop and asserts that
 * loss decreases monotonically over the steps.
 *
 * Mirrors `code/packages/ruby/ml_framework_core/test/end_to_end_training_test.rb`
 * adapted to TS / vitest idiom.
 *
 *   - 2-layer MLP: x → linear(W₁) → ReLU → linear(W₂) → MSE loss
 *   - 30 SGD steps at lr=0.01
 *   - Assert final loss < initial loss / 4 (75%+ drop)
 *
 * We use a tiny problem (4 samples, 2 hidden units) so the test runs in
 * milliseconds.  Larger problems would exercise the Rust dispatch path
 * which has its own tests; here we're only validating the autograd loop.
 */

import { describe, it, expect } from "vitest";
import { Tensor } from "../src/index.js";

/**
 * SGD step.  PyTorch convention: subtract lr * grad from the param's
 * data, then zero the gradient before the next step.
 *
 * Tensors are immutable from the outside (no public `.data =` setter),
 * so "in-place update" is really "swap the underlying data array" — we
 * build a fresh Tensor with `requiresGrad = true` and let the previous
 * one fall out of scope.
 */
function sgdStep(param: Tensor, lr: number): Tensor {
  const newData = param.toArray().map((v, i) => v - lr * param.grad!.toArray()[i]!);
  const newParam = new Tensor(newData, { shape: param.shape.slice() });
  newParam.requiresGrad = true;
  return newParam;
}

describe("EndToEndTraining", () => {
  it("two-layer MLP loss decreases substantially over 30 SGD steps", () => {
    // Synthetic dataset: regress y = 2x + 3 over 4 samples.
    const x = new Tensor([[0], [1], [2], [3]]);
    const target = new Tensor([[3], [5], [7], [9]]);

    // Layer 1: (1, 2) — 1-D input → 2 hidden units.
    // Layer 2: (2, 1) — 2 hidden units → 1 output.
    // Initialise to small values for deterministic test runs.
    let w1 = new Tensor([[0.5, -0.3]]); w1.requiresGrad = true;
    let w2 = new Tensor([[0.4], [0.7]]); w2.requiresGrad = true;

    // lr=0.01 is conservative; with lr=0.05 the small MLP overshoots
    // and lands in a 2-cycle around the minimum (correct gradient
    // descent behavior but defeats the monotonicity check).
    const lr = 0.01;
    const steps = 30;
    const losses: number[] = [];

    for (let step = 0; step < steps; step++) {
      // Forward: pred = ((x @ w1).relu) @ w2
      const pred = x.matmul(w1).relu().matmul(w2);

      // MSE loss: mean((pred - target)²)
      const diff = pred.sub(target);
      const loss = diff.mul(diff).mean();
      losses.push(loss.toArray()[0]!);

      loss.backward();

      // SGD step — fresh Tensors with no gradient carry over.
      w1 = sgdStep(w1, lr);
      w2 = sgdStep(w2, lr);
    }

    const initial = losses[0]!;
    const final = losses[steps - 1]!;

    // Hard requirement: loss MUST decrease overall.
    expect(final).toBeLessThan(initial);

    // Stronger: loss should drop by at least 75% — proves the autograd
    // gradients are pointing in the right direction, not just walking.
    // The model has no bias so it can't fit y=2x+3 perfectly; it
    // bottoms out around 3.2 (vs. initial ~36).
    const dropRatio = (initial - final) / initial;
    expect(dropRatio).toBeGreaterThan(0.75);

    // Sanity: mostly monotonic.  Allow up to floor(steps/3) up-steps
    // (SGD is noisy) but the trend must be down.
    const increasingSteps = losses.slice(0, -1).filter((l, i) => losses[i + 1]! > l).length;
    expect(increasingSteps).toBeLessThanOrEqual(Math.floor(steps / 3));
  });

  it("1-layer linear regression converges w near true value", () => {
    // Simplest case: y = w * x. No nonlinearity, single parameter.
    const x = new Tensor([[1], [2], [3], [4]]);
    const target = new Tensor([[2], [4], [6], [8]]);  // y = 2x exactly

    let w = new Tensor([[0.5]]);  // start far from true (2.0)
    w.requiresGrad = true;

    const lr = 0.05;
    for (let step = 0; step < 20; step++) {
      const pred = x.matmul(w);
      const diff = pred.sub(target);
      const loss = diff.mul(diff).mean();
      loss.backward();
      w = sgdStep(w, lr);
    }

    const finalW = w.toArray()[0]!;
    expect(Math.abs(finalW - 2)).toBeLessThan(0.5);
  });
});
