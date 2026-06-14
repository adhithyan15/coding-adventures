/**
 * autograd.test.ts — exercises Function.apply + Tensor#backward.
 *
 * Mirrors `code/packages/ruby/ml_framework_core/test/autograd_test.rb`
 * adapted to vitest idiom.  Uses only the `Identity` Function subclass
 * so the tests don't depend on op-specific math — PR #3 adds the real
 * ops and their own parity/gradient tests.
 *
 * Sections:
 *  - apply() wiring: requiresGrad propagation, grad_fn attachment
 *  - Tensor#backward sanity: must require requiresGrad, must accept seed
 *  - End-to-end: backward populates leaf .grad
 *  - Accumulation: repeated backward sums grads
 *  - Chain: multi-step graphs propagate
 *  - Shared parent: `x → Id → a, x → Id → b` accumulates
 *  - Function introspection
 *  - onesLike / zerosLike factories
 */

import { describe, it, expect } from "vitest";
import { Tensor, Function, Identity } from "../src/index.js";

describe("Function.apply — wiring", () => {
  it("propagates requiresGrad through Identity", () => {
    const x = new Tensor([1, 2, 3]);
    x.requiresGrad = true;
    const y = Identity.apply(x);

    expect(y.requiresGrad).toBe(true);
    expect(y.gradFn).toBeInstanceOf(Identity);
    expect(y.gradFn.parents).toEqual([x]);
  });

  it("does not attach gradFn when no input requires grad", () => {
    const x = new Tensor([1, 2, 3]);
    const y = Identity.apply(x);
    expect(y.requiresGrad).toBe(false);
    expect(y.gradFn).toBeNull();
  });

  it("apply() returns a NEW tensor (object identity, not value equality)", () => {
    const x = new Tensor([1, 2]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    expect(y).not.toBe(x);
    expect(y.equals(x)).toBe(true);
  });

  it("subclass without forward throws on apply", () => {
    class BadFn extends Function {
      forward(): Tensor {
        throw new Error("not implemented");
      }
      backward(): (Tensor | null)[] {
        throw new Error("not implemented");
      }
    }
    expect(() => BadFn.apply(new Tensor([1]))).toThrow();
  });

  it("subclass without backward throws on tensor.backward()", () => {
    class NoBack extends Function {
      forward(...inputs: unknown[]): Tensor {
        const x = inputs[0] as Tensor;
        return new Tensor(Array.from(x.data), { shape: x.shape.slice() });
      }
      backward(): (Tensor | null)[] {
        throw new Error("subclass must implement backward");
      }
    }
    const x = new Tensor([1]);
    x.requiresGrad = true;
    const y = NoBack.apply(x);
    expect(() => y.backward()).toThrow();
  });
});

describe("Tensor#backward — sanity", () => {
  it("throws when called on a non-grad tensor", () => {
    const x = new Tensor([1, 2, 3]);
    expect(x.requiresGrad).toBe(false);
    expect(() => x.backward()).toThrow();
  });

  it("throws when seed grad shape doesn't match", () => {
    const x = new Tensor([1, 2]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    expect(() => y.backward(Tensor.ones(3))).toThrow(RangeError);
  });

  it("returns undefined (void)", () => {
    const x = new Tensor([1, 2]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    expect(y.backward()).toBeUndefined();
  });
});

describe("Tensor#backward — end-to-end", () => {
  it("identity backward writes ones to leaf .grad", () => {
    const x = new Tensor([1, 2, 3]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    y.backward();
    // d(identity(x))/dx == 1; with seed = ones, x.grad = ones.
    expect(x.grad).not.toBeNull();
    expect(x.grad!.toArray()).toEqual([1, 1, 1]);
  });

  it("identity backward with explicit seed grad", () => {
    const x = new Tensor([1, 2]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    y.backward(Tensor.full([2], 5));
    // Identity's local derivative is 1; seed = [5, 5] passes through.
    expect(x.grad!.toArray()).toEqual([5, 5]);
  });

  it("backward twice accumulates into leaf grad", () => {
    const x = new Tensor([1, 2]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    y.backward();
    y.backward();
    // PyTorch convention: repeated backward sums into .grad.
    expect(x.grad!.toArray()).toEqual([2, 2]);
  });

  it("chain of identities propagates", () => {
    const x = new Tensor([1, 2, 3]);
    x.requiresGrad = true;
    const y = Identity.apply(Identity.apply(Identity.apply(x)));
    y.backward();
    // Chain rule with all-identity is still all-ones.
    expect(x.grad!.toArray()).toEqual([1, 1, 1]);
  });

  it("shared parent accumulates from both paths", () => {
    // Build:  x ─┬─ Id ─→ a
    //            └─ Id ─→ b
    // Run backward on each independently; the SAME leaf x.grad accumulates.
    const x = new Tensor([1, 2]);
    x.requiresGrad = true;
    const a = Identity.apply(x);
    const b = Identity.apply(x);
    a.backward();
    b.backward();
    expect(x.grad!.toArray()).toEqual([2, 2]);
  });

  it("leaf without requiresGrad is skipped", () => {
    const x = new Tensor([1, 2]); // no requiresGrad
    const y = new Tensor([1, 2]);
    y.requiresGrad = true;
    const z1 = Identity.apply(x);
    expect(z1.requiresGrad).toBe(false);
    expect(x.grad).toBeNull();

    const w = Identity.apply(y);
    w.backward();
    expect(y.grad!.toArray()).toEqual([1, 1]);
  });
});

describe("Function — introspection", () => {
  it("toString shows class name and parent count", () => {
    const x = new Tensor([1]);
    x.requiresGrad = true;
    const y = Identity.apply(x);
    expect(y.gradFn.toString()).toContain("Identity");
    expect(y.gradFn.toString()).toContain("parents=1");
  });

  it("default state on freshly-constructed Function", () => {
    const fn = new Identity();
    expect(fn.parents).toEqual([]);
    expect(fn.savedForBackward).toEqual({});
  });
});

describe("Tensor — onesLike / zerosLike", () => {
  it("onesLike", () => {
    const x = Tensor.zeros(2, 3);
    const o = Tensor.onesLike(x);
    expect(o.shape).toEqual(x.shape);
    expect(o.toArray()).toEqual([1, 1, 1, 1, 1, 1]);
  });

  it("zerosLike", () => {
    const x = Tensor.ones(4);
    const z = Tensor.zerosLike(x);
    expect(z.shape).toEqual(x.shape);
    expect(z.toArray()).toEqual([0, 0, 0, 0]);
  });
});
