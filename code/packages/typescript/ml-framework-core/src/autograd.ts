/**
 * # autograd.ts — reverse-mode automatic differentiation for Tensor
 *
 * Adds two pieces of machinery on top of v0.1's Tensor:
 *
 *   1. The `Function` base class.  Every differentiable op (Add, MatMul,
 *      ReLU, ...) is a `Function` subclass that defines `forward` and
 *      `backward`.  PR #3 implements the ~15 specific subclasses; here we
 *      provide the base class and one `Identity` subclass for testing.
 *
 *   2. The `backwardImpl` function — a free standalone function that
 *      implements topological sort + reverse walk on the autograd graph.
 *      `Tensor#backward` (defined in tensor.ts) delegates to it.
 *
 * ## Why backward() is a free function, not a method
 *
 * The walker needs to read every Tensor's `gradFn` and write its `grad`.
 * If it lived inside `Tensor`, we'd have either (a) a circular dependency
 * between tensor.ts and autograd.ts, or (b) a giant tensor.ts that owns
 * both storage and graph logic.  Splitting into a free function in
 * autograd.ts keeps the modules small and lets Tensor stay focused on
 * "I am a Float32Array with a shape."
 *
 * The `tensor.ts` file does have a thin `Tensor#backward(grad?)` method
 * but it's a one-liner that imports and calls `backwardImpl`.
 *
 * ## Algorithm (same as Ruby pilot + Python reference)
 *
 *   1. If `grad` is undefined, default to ones-like (PyTorch convention;
 *      strictly only valid for scalar outputs but we allow any shape).
 *   2. DFS post-order to build the topological list upstream through the
 *      `gradFn` chain.
 *   3. Walk topo list in REVERSE.  For each non-leaf, call
 *      `gradFn.backward(nodeGrad)` to get per-input grads, accumulate
 *      them into a per-parent `gradMap` keyed on object identity (via a
 *      JS `Map`, which uses reference equality for object keys).
 *   4. For each leaf with `requiresGrad`, copy/accumulate the gradient
 *      into the public `.grad` slot.  Supports repeated `backward()`
 *      without zero_grad (PyTorch convention).
 *
 * O(V + E) where V is operations, E is tensor edges.
 */

import { Tensor } from "./tensor.js";

// ---------------------------------------------------------------------------
// Function base class — extend for every differentiable op.
// ---------------------------------------------------------------------------

export abstract class Function {
  /**
   * The tensor inputs that produced the output.  `backward()` returns
   * gradients in the same order.  Set by `apply()`; subclasses should
   * not touch.
   */
  public parents: Tensor[] = [];

  /**
   * Free-form storage for whatever the subclass wants to cache in
   * `forward()` for use in `backward()` — typically the input tensors,
   * the output, or shape metadata.  Symbol-keyed `Map`-like Record so
   * keys are namespaced per-instance.
   */
  public savedForBackward: Record<string, unknown> = {};

  /**
   * The canonical entry point for invoking an op.
   *
   *   1. Instantiate the Function (so it can hold state for backward).
   *   2. Filter `inputs` to Tensors and stash them as `parents`.  Non-Tensor
   *      args (e.g. Pow's scalar exponent) pass through to `forward` but
   *      don't appear in the autograd graph — they don't have gradients.
   *   3. Run `forward(...inputs)`.
   *   4. If any Tensor input has `requiresGrad`, mark the output the same
   *      way and wire `output.gradFn = fn` so `backwardImpl` can walk us.
   *
   * The static method signature uses a generic `T extends Function` so
   * subclasses don't lose their concrete type when calling `MyOp.apply(...)`.
   */
  static apply<T extends Function>(
    this: new () => T,
    ...inputs: unknown[]
  ): Tensor {
    const fn = new this();

    fn.parents = inputs.filter((i): i is Tensor => i instanceof Tensor);

    const output = fn.forward(...inputs);

    const needsGrad = inputs.some((i) => i instanceof Tensor && i.requiresGrad);
    if (needsGrad) {
      output.requiresGrad = true;
      output.gradFn = fn;
    }
    return output;
  }

  /** Subclasses override.  Compute the forward result. */
  abstract forward(...inputs: unknown[]): Tensor;

  /**
   * Subclasses override.  Given the gradient of the loss w.r.t. this
   * Function's output, return one gradient per `parents[i]` (in the same
   * order).  Use `null` in a slot for any parent that doesn't need a
   * gradient (rare).
   */
  abstract backward(outputGrad: Tensor): (Tensor | null)[];

  /** Friendly default for inspect / console.log output. */
  toString(): string {
    return `<${this.constructor.name} parents=${this.parents.length}>`;
  }
}

// ---------------------------------------------------------------------------
// Identity — the simplest possible Function subclass.
//
// forward(x)  → fresh Tensor with the same data as x
// backward(g) → [g]   (gradient passes through unchanged)
//
// Used by the autograd test suite to exercise the apply() / backward()
// machinery without depending on op-specific math.  The real ops land
// in PR #3 (ops.ts).
// ---------------------------------------------------------------------------

export class Identity extends Function {
  forward(...inputs: unknown[]): Tensor {
    const x = inputs[0];
    if (!(x instanceof Tensor)) {
      throw new TypeError("Identity.forward expects a Tensor argument");
    }
    // Return a NEW Tensor so object identity (`y === x`) is false even
    // though `y.equals(x)` is true.  Constructors copy, so this works.
    return new Tensor(Array.from(x.data), { shape: x.shape.slice() });
  }

  backward(outputGrad: Tensor): (Tensor | null)[] {
    // d/dx(x) = 1; gradient passes through unchanged.
    return [outputGrad];
  }
}

// ---------------------------------------------------------------------------
// backwardImpl — the actual reverse-mode autodiff walker.
//
// Called from `Tensor#backward(grad?)` in tensor.ts.  Lives here so the
// algorithm code (which is the meat of autograd) sits next to the
// Function base class.
// ---------------------------------------------------------------------------

export function backwardImpl(start: Tensor, grad?: Tensor): void {
  if (!start.requiresGrad) {
    throw new Error("backward() called on a tensor that doesn't require grad");
  }

  const seed = grad ?? Tensor.onesLike(start);
  if (!shapesMatch(seed.shape, start.shape)) {
    throw new RangeError(
      `backward grad shape [${seed.shape.join(", ")}] != tensor shape [${start.shape.join(", ")}]`,
    );
  }

  // Topological order: each tensor appears AFTER its parents.  Walking in
  // reverse processes children before parents.
  const topoOrder: Tensor[] = [];
  const visited = new Set<Tensor>();   // identity-based, perfect for our needs
  function buildTopo(t: Tensor): void {
    if (visited.has(t)) return;
    visited.add(t);
    if (t.gradFn) {
      for (const p of t.gradFn.parents) buildTopo(p);
    }
    topoOrder.push(t);
  }
  buildTopo(start);

  // gradMap: Tensor → accumulated gradient.  Map uses reference equality
  // for object keys, which is exactly what we need — the same Tensor
  // reaching multiple paths (e.g. `Add(x, x)`) collapses to one entry.
  const gradMap = new Map<Tensor, Tensor>();
  gradMap.set(start, seed);

  for (let i = topoOrder.length - 1; i >= 0; i--) {
    const node = topoOrder[i]!;
    const nodeGrad = gradMap.get(node);
    if (!nodeGrad) continue;

    if (!node.gradFn) {
      // Leaf — store/accumulate into the public grad slot.
      if (!node.requiresGrad) continue;

      if (!node.grad) {
        // First time seeing this leaf — copy the accumulated grad.
        node.grad = new Tensor(Array.from(nodeGrad.data), { shape: nodeGrad.shape.slice() });
      } else {
        // Accumulate: supports repeated backward() without zero_grad
        // (caller is expected to zero between steps when training).
        node.grad = node.grad.add(nodeGrad);
      }
      continue;
    }

    // Non-leaf — ask the Function for per-input grads, distribute them.
    const inputGrads = node.gradFn.backward(nodeGrad);
    for (let j = 0; j < node.gradFn.parents.length; j++) {
      const parent = node.gradFn.parents[j]!;
      const inputGrad = inputGrads[j];
      if (!inputGrad) continue;

      const existing = gradMap.get(parent);
      gradMap.set(parent, existing ? existing.add(inputGrad) : inputGrad);
    }
  }
}

function shapesMatch(a: readonly number[], b: readonly number[]): boolean {
  if (a.length !== b.length) return false;
  for (let i = 0; i < a.length; i++) {
    if (a[i] !== b[i]) return false;
  }
  return true;
}
