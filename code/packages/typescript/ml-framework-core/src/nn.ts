/**
 * # nn.ts — neural-network layer abstractions (Phase A.6)
 *
 * Adds the two pieces that turn a pile of Tensor ops into something
 * you can call `Sequential([...]).forward(x)` on:
 *
 *   - **`Module`** — the abstract base class.  A module owns
 *     parameters (Tensors with `requiresGrad = true`) and exposes
 *     `forward(...)` for inference.  Composing modules is just
 *     putting one inside another; `parameters()` recursively collects
 *     all leaf params for the optimizer.
 *
 *   - **`Linear`** — the standard fully-connected layer:
 *     `y = x @ W + b`.  Weight shape is `(inFeatures, outFeatures)`
 *     so the forward is a single matmul with no transpose needed.
 *     This DIVERGES from PyTorch (which stores `(outFeatures,
 *     inFeatures)` and does `x @ W.T`) — but our `Tensor.transpose()`
 *     is currently a non-autograd shape op (it doesn't track
 *     gradients back through itself), so doing `x @ W.T` would
 *     silently drop the gradient on `W`.  Keeping weight in
 *     `(in, out)` orientation sidesteps that.  A future PR can
 *     add a `TransposeOp` that's autograd-aware and flip the
 *     convention back if cross-framework state-dict compatibility
 *     matters.  For v1.6 the framework's internal use cases all
 *     work fine with `(in, out)`.
 *
 *   - **`Sequential`** — composes a list of modules; forward applies
 *     them in order; `parameters()` concatenates child params.
 *
 * ## Initialization
 *
 * Linear uses Xavier-uniform: each weight ~ U(-L, L) where L =
 * √(6 / (in + out)).  Bias is zero-init.  This matches PyTorch's
 * default for `nn.Linear` and keeps activations roughly unit-variance
 * through a stack of layers, which makes training much more stable
 * than the naïve "uniform in [-1, 1]" alternative.
 */

import { Tensor } from "./tensor.js";

/**
 * Abstract module base.  Subclasses implement `parameters()` (returns
 * the list of learnable Tensors, used by optimizers) and `forward(x)`
 * (the computation graph).  A `Sequential` composes children; each
 * child contributes its own `parameters()`.
 */
export abstract class Module {
  /** All learnable parameters owned by this module + its children. */
  abstract parameters(): Tensor[];

  /** Apply this module to an input.  Subclasses override the signature. */
  abstract forward(x: Tensor): Tensor;
}

/**
 * Fully-connected (dense) layer: `y = x @ W + b`.
 *
 * - Input  shape: `(..., inFeatures)`
 * - Weight shape: `(inFeatures, outFeatures)`
 * - Bias   shape: `(outFeatures,)` (broadcast over leading dims) — optional
 * - Output shape: `(..., outFeatures)`
 *
 * Both `weight` and `bias` have `requiresGrad = true` so gradients
 * accumulate on `loss.backward()` and an optimizer can read them.
 *
 * Weight init: Xavier-uniform with limit √(6 / (in + out)).  Bias zero.
 */
export class Linear extends Module {
  public readonly weight: Tensor;
  public readonly bias: Tensor | null;

  constructor(
    public readonly inFeatures: number,
    public readonly outFeatures: number,
    useBias: boolean = true,
  ) {
    super();
    if (inFeatures < 1 || outFeatures < 1) {
      throw new RangeError(
        `Linear requires positive in/out features, got in=${inFeatures}, out=${outFeatures}`,
      );
    }
    // Xavier-uniform init: limit L = √(6 / (in + out))
    const limit = Math.sqrt(6 / (inFeatures + outFeatures));
    const wData = new Array<number>(inFeatures * outFeatures);
    for (let i = 0; i < wData.length; i++) {
      wData[i] = (Math.random() * 2 - 1) * limit;
    }
    this.weight = new Tensor(wData, { shape: [inFeatures, outFeatures] });
    this.weight.requiresGrad = true;

    if (useBias) {
      this.bias = new Tensor(new Array<number>(outFeatures).fill(0));
      this.bias.requiresGrad = true;
    } else {
      this.bias = null;
    }
  }

  parameters(): Tensor[] {
    return this.bias ? [this.weight, this.bias] : [this.weight];
  }

  forward(x: Tensor): Tensor {
    // x: (..., inF) @ weight: (inF, outF) → (..., outF)
    // Uses MatMulOp's N-D batched matmul (v1.2) so leading dims pass through.
    let y = x.matmul(this.weight);
    if (this.bias) y = y.add(this.bias); // bias (outF,) broadcasts over leading dims
    return y;
  }
}

/**
 * Compose modules in sequence: `forward(x)` returns
 * `lastLayer.forward(...secondLayer.forward(firstLayer.forward(x)))`.
 *
 * `parameters()` is the concatenation of every child's `parameters()`,
 * in declaration order.  This is the list you hand to an optimizer.
 *
 * Layers can be Linear, Sequential nesting, or anything else extending
 * `Module`.  Activation functions are NOT modules in v1.6 — they're
 * pure Tensor methods (`x.relu()`, `x.sigmoid()`, etc.), so you wrap
 * them in tiny custom modules if you want to drop them into a
 * Sequential.  Helper class for that, `Fn`, is below.
 */
export class Sequential extends Module {
  public readonly layers: Module[];

  constructor(...layers: Module[]) {
    super();
    this.layers = layers;
  }

  parameters(): Tensor[] {
    const out: Tensor[] = [];
    for (const layer of this.layers) out.push(...layer.parameters());
    return out;
  }

  forward(x: Tensor): Tensor {
    let y = x;
    for (const layer of this.layers) y = layer.forward(y);
    return y;
  }
}

/**
 * Wrap any `Tensor → Tensor` function as a Module — useful for
 * dropping activation functions into a `Sequential`:
 *
 * ```ts
 * new Sequential(
 *   new Linear(784, 128),
 *   new Fn(x => x.relu()),
 *   new Linear(128, 10),
 * );
 * ```
 *
 * `Fn` has no parameters of its own.  It's the universal escape hatch.
 */
export class Fn extends Module {
  constructor(private readonly fn: (x: Tensor) => Tensor) {
    super();
  }
  parameters(): Tensor[] {
    return [];
  }
  forward(x: Tensor): Tensor {
    return this.fn(x);
  }
}
