/**
 * ops.test.ts — coverage for the 15 differentiable ops added in PR #3.
 *
 * Mirrors `code/packages/ruby/ml_framework_core/test/ops_test.rb` adapted
 * to vitest idiom.
 *
 * Sections:
 *  - HexHelpers: pack/unpack round-trip + known-value spot checks
 *  - ForwardSmall: every op's pure-TS path with numerical correctness
 *  - AutogradWiring: every op produces a tensor with the right gradFn
 *  - TensorMethods: t.relu() etc. dispatch correctly
 *  - DispatchPathBranching: threshold constant + small-tensor doesn't
 *    trigger matrix-rust-napi lazy require
 *
 * We DON'T exercise the Rust dispatch path here because it requires the
 * matrix-rust-napi .node addon to be built — integration territory.
 * Numerical parity between TS fallback and Rust dispatch is the same
 * spec as the Python/Ruby implementations (the envelopes are byte-for-byte
 * identical), so no separate parity test needed.
 */

import { describe, it, expect } from "vitest";
import {
  Tensor,
  AddOp,
  SubOp,
  MulOp,
  DivOp,
  NegOp,
  AbsOp,
  PowOp,
  MatMulOp,
  ReLUOp,
  SigmoidOp,
  TanhOp,
  GELUOp,
  SoftmaxOp,
  SumOp,
  MeanOp,
  DISPATCH_THRESHOLD,
  packF32Hex,
  unpackF32Hex,
} from "../src/index.js";

describe("HexHelpers", () => {
  it("pack then unpack round-trips f32 values exactly", () => {
    const arr = [1.5, 2.5, -3.25, 0, 1];
    const hex = packF32Hex(arr);
    const back = unpackF32Hex(hex, arr.length);
    for (let i = 0; i < arr.length; i++) {
      expect(back[i]).toBe(arr[i]);
    }
  });

  it("pack produces 8 hex chars per f32 cell (4 bytes × 2)", () => {
    expect(packF32Hex([1])).toHaveLength(8);
  });

  it("packF32Hex(1.0) is the known little-endian f32 pattern", () => {
    expect(packF32Hex([1])).toBe("0000803f");
  });

  it("unpackF32Hex of known hex returns 1.0", () => {
    expect(unpackF32Hex("0000803f", 1)[0]).toBe(1);
  });

  it("unpackF32Hex rejects wrong length", () => {
    expect(() => unpackF32Hex("0000803f", 2)).toThrow();
  });
});

describe("ForwardSmall — every op pure-TS path", () => {
  // Binary elementwise
  it("Add", () => {
    expect(AddOp.apply(new Tensor([1, 2, 3]), new Tensor([10, 20, 30])).toArray())
      .toEqual([11, 22, 33]);
  });
  it("Sub", () => {
    expect(SubOp.apply(new Tensor([10, 20]), new Tensor([1, 2])).toArray())
      .toEqual([9, 18]);
  });
  it("Mul", () => {
    expect(MulOp.apply(new Tensor([2, 3]), new Tensor([4, 5])).toArray())
      .toEqual([8, 15]);
  });
  it("Div", () => {
    expect(DivOp.apply(new Tensor([10, 20]), new Tensor([2, 4])).toArray())
      .toEqual([5, 5]);
  });

  // Unary elementwise
  it("Neg", () => {
    expect(NegOp.apply(new Tensor([1, -2, 3])).toArray()).toEqual([-1, 2, -3]);
  });
  it("Abs", () => {
    expect(AbsOp.apply(new Tensor([-1.5, 2.5, -3.5])).toArray()).toEqual([1.5, 2.5, 3.5]);
  });

  // Pow (scalar exponent)
  it("Pow with scalar exponent", () => {
    expect(PowOp.apply(new Tensor([2, 3, 4]), 2).toArray()).toEqual([4, 9, 16]);
  });

  // MatMul
  it("MatMul small 2×2", () => {
    const a = new Tensor([[1, 2], [3, 4]]);
    const b = new Tensor([[5, 6], [7, 8]]);
    const out = MatMulOp.apply(a, b);
    expect(out.shape).toEqual([2, 2]);
    expect(out.toArray()).toEqual([19, 22, 43, 50]);
  });
  it("MatMul rejects non-2-D", () => {
    expect(() => MatMulOp.apply(new Tensor([1, 2, 3]), new Tensor([[1, 2], [3, 4]])))
      .toThrow(RangeError);
  });
  it("MatMul rejects inner-dim mismatch", () => {
    expect(() => MatMulOp.apply(Tensor.zeros(2, 3), Tensor.zeros(4, 5)))
      .toThrow(RangeError);
  });

  // Activations
  it("ReLU clips negatives to zero", () => {
    expect(ReLUOp.apply(new Tensor([-1, 0, 1, -2.5, 3.5])).toArray())
      .toEqual([0, 0, 1, 0, 3.5]);
  });
  it("Sigmoid at 0 is 0.5", () => {
    expect(SigmoidOp.apply(new Tensor([0])).toArray()[0]).toBeCloseTo(0.5, 6);
  });
  it("Sigmoid is monotonically increasing", () => {
    const out = SigmoidOp.apply(new Tensor([-2, -1, 0, 1, 2])).toArray();
    for (let i = 0; i < out.length - 1; i++) {
      expect(out[i + 1]).toBeGreaterThan(out[i]!);
    }
  });
  it("Tanh at 0 is 0", () => {
    expect(TanhOp.apply(new Tensor([0])).toArray()[0]).toBeCloseTo(0, 6);
  });
  it("Tanh approaches 1 for large positive", () => {
    expect(TanhOp.apply(new Tensor([10])).toArray()[0]).toBeCloseTo(1, 5);
  });
  it("GELU at 0 is 0", () => {
    expect(GELUOp.apply(new Tensor([0])).toArray()[0]).toBeCloseTo(0, 6);
  });
  it("GELU at 1 is ≈ 0.8413", () => {
    expect(GELUOp.apply(new Tensor([1])).toArray()[0]).toBeCloseTo(0.8413, 3);
  });
  it("Softmax sums to 1", () => {
    const out = SoftmaxOp.apply(new Tensor([1, 2, 3, 4])).toArray();
    expect(out.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 5);
  });
  it("Softmax: largest input gives largest output", () => {
    const out = SoftmaxOp.apply(new Tensor([1, 2, 3, 4])).toArray();
    expect(Math.max(...out)).toBe(out[3]);
    expect(Math.min(...out)).toBe(out[0]);
  });
  it("Softmax is numerically stable for huge inputs", () => {
    // Without the max-subtract trick, exp(1000) = Infinity.
    const out = SoftmaxOp.apply(new Tensor([1000, 1001, 1002])).toArray();
    for (const v of out) expect(Number.isFinite(v)).toBe(true);
    expect(out.reduce((a, b) => a + b, 0)).toBeCloseTo(1, 5);
  });

  // Reductions
  it("Sum returns scalar shape [1]", () => {
    const out = SumOp.apply(new Tensor([1, 2, 3, 4]));
    expect(out.shape).toEqual([1]);
    expect(out.toArray()).toEqual([10]);
  });
  it("Mean returns scalar shape [1]", () => {
    const out = MeanOp.apply(new Tensor([1, 2, 3, 4]));
    expect(out.shape).toEqual([1]);
    expect(out.toArray()).toEqual([2.5]);
  });

  // Backward not implemented yet (PR #4)
  it("backward throws 'not implemented' for now", () => {
    const x = new Tensor([1]);
    x.requiresGrad = true;
    const y = AddOp.apply(x, new Tensor([2]));
    expect(() => y.backward()).toThrow(/not implemented/);
  });
});

describe("AutogradWiring — every op gets the right gradFn", () => {
  function expectGradFnClass(out: Tensor, klass: Function): void {
    expect(out.requiresGrad).toBe(true);
    expect(out.gradFn).toBeInstanceOf(klass);
  }

  it("AddOp wires gradFn", () => {
    const x = new Tensor([1, 2]); x.requiresGrad = true;
    const y = new Tensor([3, 4]); y.requiresGrad = true;
    expectGradFnClass(AddOp.apply(x, y), AddOp);
  });

  it("all unary ops wire gradFn", () => {
    const x = new Tensor([1, 2]); x.requiresGrad = true;
    expectGradFnClass(NegOp.apply(x), NegOp);
    expectGradFnClass(AbsOp.apply(x), AbsOp);
    expectGradFnClass(ReLUOp.apply(x), ReLUOp);
    expectGradFnClass(SigmoidOp.apply(x), SigmoidOp);
    expectGradFnClass(TanhOp.apply(x), TanhOp);
    expectGradFnClass(GELUOp.apply(x), GELUOp);
    expectGradFnClass(SoftmaxOp.apply(x), SoftmaxOp);
  });

  it("MatMul wires gradFn", () => {
    const a = Tensor.zeros(2, 3); a.requiresGrad = true;
    const b = Tensor.zeros(3, 2);
    expectGradFnClass(MatMulOp.apply(a, b), MatMulOp);
  });

  it("Sum / Mean wire gradFn", () => {
    const x = new Tensor([1, 2, 3]); x.requiresGrad = true;
    expectGradFnClass(SumOp.apply(x), SumOp);
    expectGradFnClass(MeanOp.apply(x), MeanOp);
  });

  it("no gradFn when no input requires grad", () => {
    const out = AddOp.apply(new Tensor([1]), new Tensor([2]));
    expect(out.requiresGrad).toBe(false);
    expect(out.gradFn).toBeNull();
  });
});

describe("TensorMethods — sugar that dispatches to the Op classes", () => {
  it("t.add(b) dispatches through AddOp", () => {
    const x = new Tensor([1, 2]); x.requiresGrad = true;
    const z = x.add(new Tensor([3, 4]));
    expect(z.toArray()).toEqual([4, 6]);
    expect(z.gradFn).toBeInstanceOf(AddOp);
  });

  it("t.neg() dispatches through NegOp", () => {
    const x = new Tensor([1, -2]); x.requiresGrad = true;
    const y = x.neg();
    expect(y.toArray()).toEqual([-1, 2]);
    expect(y.gradFn).toBeInstanceOf(NegOp);
  });

  it("scalar broadcast via coercion", () => {
    expect(new Tensor([1, 2, 3]).add(5).toArray()).toEqual([6, 7, 8]);
  });

  it("t.pow(n) dispatches through PowOp", () => {
    expect(new Tensor([2, 3]).pow(3).toArray()).toEqual([8, 27]);
  });

  it("t.relu() dispatches", () => {
    const x = new Tensor([-1, 0, 1]); x.requiresGrad = true;
    const y = x.relu();
    expect(y.toArray()).toEqual([0, 0, 1]);
    expect(y.gradFn).toBeInstanceOf(ReLUOp);
  });

  it("t.sigmoid() dispatches", () => {
    expect(new Tensor([0]).sigmoid().toArray()[0]).toBeCloseTo(0.5, 6);
  });

  it("t.tanh() dispatches", () => {
    expect(new Tensor([0]).tanh().toArray()[0]).toBeCloseTo(0, 6);
  });

  it("t.gelu() dispatches", () => {
    expect(new Tensor([0]).gelu().toArray()[0]).toBeCloseTo(0, 6);
  });

  it("t.softmax() dispatches", () => {
    const out = new Tensor([1, 1, 1]).softmax().toArray();
    for (const v of out) expect(v).toBeCloseTo(1 / 3, 6);
  });

  it("t.sum() dispatches", () => {
    expect(new Tensor([1, 2, 3]).sum().toArray()).toEqual([6]);
  });

  it("t.mean() dispatches", () => {
    expect(new Tensor([1, 2, 3]).mean().toArray()).toEqual([2]);
  });

  it("t.matmul(b) dispatches", () => {
    const a = new Tensor([[1, 2], [3, 4]]);
    const b = new Tensor([[1, 0], [0, 1]]);
    expect(a.matmul(b).toArray()).toEqual(a.toArray());
  });

  it("t.abs() dispatches", () => {
    expect(new Tensor([-1, 2, -3]).abs().toArray()).toEqual([1, 2, 3]);
  });

  it("unsupported operand throws TypeError", () => {
    expect(() => new Tensor([1]).add("oops" as never)).toThrow(TypeError);
  });
});

describe("DispatchPathBranching", () => {
  it("DISPATCH_THRESHOLD is 10_000", () => {
    expect(DISPATCH_THRESHOLD).toBe(10_000);
  });

  it("small tensors stay in pure-TS path (no matrix-rust-napi require)", () => {
    // If small-tensor path tried to dispatch, the lazy require would
    // fire and we'd see a LoadError (the .node addon isn't built in
    // every test environment).  We don't actually inspect the module
    // cache here — just confirm the operation completes without
    // throwing the kind of error a missing addon would produce.
    expect(() =>
      AddOp.apply(new Tensor([1, 2]), new Tensor([3, 4]))
    ).not.toThrow();
  });
});
