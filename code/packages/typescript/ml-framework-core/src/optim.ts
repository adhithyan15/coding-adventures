/**
 * # optim.ts — gradient-descent optimizers (Phase A.6)
 *
 * Once a model has `parameters()` (each a Tensor with `requiresGrad =
 * true`) and you've called `loss.backward()`, the gradients sit in
 * `param.grad`.  An Optimizer turns those gradients into parameter
 * updates.  PyTorch convention: optimizer holds references to the
 * params, mutates `param.data` in place during `step()`.
 *
 * Two optimizers ship in v1.6:
 *
 *   - **SGD** — `param -= lr * grad`.  The simplest possible update.
 *     Great for getting started, lousy for non-trivial problems.
 *   - **Adam** — adaptive moment estimation with bias correction.
 *     Effectively the default for modern deep learning.  Tracks
 *     per-parameter first (m) and second (v) moment estimates,
 *     applies bias correction so early steps aren't tiny.
 *
 * ## In-place mutation
 *
 * The Tensor type declares `data` as `readonly` — that prevents
 * reassigning the buffer, but element writes (`p.data[i] = ...`)
 * are still allowed.  This matches PyTorch's `data.add_(grad,
 * alpha=-lr)` style: optimizer updates skip autograd and just
 * write to the underlying memory.
 */

import type { Tensor } from "./tensor.js";

/**
 * Common base — owns the parameter list and provides `zeroGrad()`.
 * Subclasses implement `step()` with their update rule.
 *
 * Why `zeroGrad` sets `param.grad = null` rather than zeroing in
 * place: matches the framework's existing accumulation semantics —
 * `backwardImpl` allocates `param.grad` on first contribution and
 * accumulates via `add()` thereafter.  Setting to null is just
 * "forget what you accumulated"; the next `backward()` starts fresh.
 */
export abstract class Optimizer {
  constructor(public readonly params: Tensor[]) {
    if (params.length === 0) {
      throw new RangeError("Optimizer requires at least one parameter");
    }
  }

  /** Clear all parameter grads.  Call before each forward/backward step. */
  zeroGrad(): void {
    for (const p of this.params) p.grad = null;
  }

  /** Apply one gradient-descent step to each parameter. */
  abstract step(): void;
}

/**
 * Plain stochastic gradient descent.  `param -= lr * grad`.
 *
 * No momentum, no weight decay in v1.6 — those can be added as
 * optional constructor args in a future PR without breaking the API.
 */
export class SGD extends Optimizer {
  constructor(params: Tensor[], public lr: number) {
    super(params);
    if (lr <= 0) throw new RangeError(`SGD lr must be > 0, got ${lr}`);
  }

  step(): void {
    for (const p of this.params) {
      const g = p.grad;
      if (!g) continue; // no gradient accumulated this step
      const data = p.data; // Float32Array — element writes allowed despite readonly ref
      const gd = g.data;
      for (let i = 0; i < p.numel; i++) {
        data[i]! -= this.lr * gd[i]!;
      }
    }
  }
}

/**
 * Adam — adaptive moment estimation (Kingma & Ba 2014).
 *
 * Per parameter, maintains two running estimates:
 *
 *   m_t = β1 * m_{t-1} + (1 - β1) * g_t           ← first moment (mean of grads)
 *   v_t = β2 * v_{t-1} + (1 - β2) * g_t²          ← second moment (mean of grad²)
 *
 * Bias-corrected estimates:
 *
 *   m̂ = m_t / (1 - β1^t)
 *   v̂ = v_t / (1 - β2^t)
 *
 * Update:
 *
 *   param -= lr * m̂ / (√v̂ + ε)
 *
 * Defaults match PyTorch's `torch.optim.Adam`: lr=1e-3, betas=(0.9, 0.999), eps=1e-8.
 * No AMSGrad, no weight decay in v1.6.
 */
export class Adam extends Optimizer {
  // Per-parameter moment buffers.  Float32Arrays sized to each param's numel.
  private readonly m: Float32Array[];
  private readonly v: Float32Array[];
  // Step counter — bumped at the start of each step.
  private t = 0;

  constructor(
    params: Tensor[],
    public lr: number = 1e-3,
    public beta1: number = 0.9,
    public beta2: number = 0.999,
    public eps: number = 1e-8,
  ) {
    super(params);
    if (lr <= 0) throw new RangeError(`Adam lr must be > 0, got ${lr}`);
    if (beta1 < 0 || beta1 >= 1) throw new RangeError(`Adam beta1 must be in [0, 1), got ${beta1}`);
    if (beta2 < 0 || beta2 >= 1) throw new RangeError(`Adam beta2 must be in [0, 1), got ${beta2}`);
    if (eps <= 0) throw new RangeError(`Adam eps must be > 0, got ${eps}`);
    // Zero-init moment buffers (Float32Array default).  One per param,
    // sized to the param's numel.
    this.m = params.map((p) => new Float32Array(p.numel));
    this.v = params.map((p) => new Float32Array(p.numel));
  }

  /** Current step number (post-increment value after the latest step()). */
  get stepCount(): number {
    return this.t;
  }

  step(): void {
    this.t++;
    // Bias-correction denominators — same for all params at this t.
    const b1c = 1 - Math.pow(this.beta1, this.t);
    const b2c = 1 - Math.pow(this.beta2, this.t);
    for (let pi = 0; pi < this.params.length; pi++) {
      const p = this.params[pi]!;
      const g = p.grad;
      if (!g) continue;
      const data = p.data;
      const gd = g.data;
      const m = this.m[pi]!;
      const v = this.v[pi]!;
      for (let i = 0; i < p.numel; i++) {
        const gi = gd[i]!;
        m[i] = this.beta1 * m[i]! + (1 - this.beta1) * gi;
        v[i] = this.beta2 * v[i]! + (1 - this.beta2) * gi * gi;
        const mHat = m[i]! / b1c;
        const vHat = v[i]! / b2c;
        data[i]! -= (this.lr * mHat) / (Math.sqrt(vHat) + this.eps);
      }
    }
  }
}
