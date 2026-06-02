/**
 * tensor.test.ts — vitest coverage for the v0.1 Tensor class.
 *
 * Mirrors `code/packages/ruby/ml_framework_core/test/tensor_test.rb` but
 * adapted to TypeScript / Vitest idiom (`describe` + `it` blocks rather
 * than minitest's class-per-section convention).
 *
 * Sections:
 *  - Construction (flat, nested, scalar, explicit shape, ragged rejection,
 *    dtype rejection)
 *  - Factories (zeros / ones / full / eye / arange / randn / fromArray)
 *  - Shape ops (reshape / transpose / flatten / squeeze / unsqueeze)
 *  - Element-wise math (add / sub / mul / div / pow / neg, scalar broadcast,
 *    shape-mismatch + type rejection)
 *  - Equality + inspect
 *  - Round-trip properties
 *  - Version constant
 */

import { describe, it, expect } from "vitest";
import { Tensor, inferShape, flattenToFloat32, VERSION } from "../src/index.js";

describe("Tensor — construction", () => {
  it("constructs from a flat array with explicit shape", () => {
    const t = new Tensor([1, 2, 3, 4], { shape: [2, 2] });
    expect(t.shape).toEqual([2, 2]);
    expect(t.numel).toBe(4);
    expect(t.ndim).toBe(2);
    expect(t.dtype).toBe("f32");
    expect(t.toArray()).toEqual([1, 2, 3, 4]);
  });

  it("constructs from nested array and infers shape", () => {
    const t = new Tensor([[1, 2, 3], [4, 5, 6]]);
    expect(t.shape).toEqual([2, 3]);
    expect(t.toArray()).toEqual([1, 2, 3, 4, 5, 6]);
  });

  it("constructs from deeply-nested array", () => {
    const t = new Tensor([[[1, 2], [3, 4]], [[5, 6], [7, 8]]]);
    expect(t.shape).toEqual([2, 2, 2]);
    expect(t.numel).toBe(8);
  });

  it("constructs from a scalar with explicit shape", () => {
    const t = new Tensor(7, { shape: [1] });
    expect(t.shape).toEqual([1]);
    expect(t.toArray()).toEqual([7]);
  });

  it("throws on ragged nested arrays", () => {
    expect(() => new Tensor([[1, 2], [3]])).toThrow(TypeError);
  });

  it("throws on data length mismatch with explicit shape", () => {
    expect(() => new Tensor([1, 2, 3], { shape: [2, 2] })).toThrow(RangeError);
  });

  it("throws on unsupported dtype", () => {
    expect(() => new Tensor([1, 2, 3], { dtype: "f64" as never })).toThrow(TypeError);
  });

  it("throws when nested data contains non-numbers", () => {
    expect(() => new Tensor([[1, 2], [3, "oops"]] as never)).toThrow(TypeError);
  });
});

describe("Tensor — factories", () => {
  it("zeros() returns all-zero tensor of the requested shape", () => {
    const t = Tensor.zeros(2, 3);
    expect(t.shape).toEqual([2, 3]);
    expect(t.toArray()).toEqual([0, 0, 0, 0, 0, 0]);
  });

  it("ones() returns all-one tensor", () => {
    const t = Tensor.ones(3);
    expect(t.shape).toEqual([3]);
    expect(t.toArray()).toEqual([1, 1, 1]);
  });

  it("full([shape], value) fills uniformly", () => {
    const t = Tensor.full([2, 2], 7.5);
    expect(t.shape).toEqual([2, 2]);
    expect(t.toArray()).toEqual([7.5, 7.5, 7.5, 7.5]);
  });

  it("eye(n) returns the n×n identity matrix", () => {
    const t = Tensor.eye(3);
    expect(t.shape).toEqual([3, 3]);
    expect(t.toArray()).toEqual([1, 0, 0, 0, 1, 0, 0, 0, 1]);
  });

  it("eye(n, m) returns a rectangular diagonal-ones matrix", () => {
    const t = Tensor.eye(2, 3);
    expect(t.shape).toEqual([2, 3]);
    expect(t.toArray()).toEqual([1, 0, 0, 0, 1, 0]);
  });

  it("arange(stop) returns [0, stop)", () => {
    expect(Tensor.arange(5).toArray()).toEqual([0, 1, 2, 3, 4]);
  });

  it("arange(start, stop) returns [start, stop)", () => {
    expect(Tensor.arange(2, 5).toArray()).toEqual([2, 3, 4]);
  });

  it("arange with positive step", () => {
    expect(Tensor.arange(0, 10, 2).toArray()).toEqual([0, 2, 4, 6, 8]);
  });

  it("arange with negative step", () => {
    expect(Tensor.arange(5, 0, -1).toArray()).toEqual([5, 4, 3, 2, 1]);
  });

  it("arange rejects zero step", () => {
    expect(() => Tensor.arange(0, 5, 0)).toThrow(RangeError);
  });

  it("arange rejects non-finite bounds", () => {
    expect(() => Tensor.arange(0, Number.POSITIVE_INFINITY)).toThrow(RangeError);
    expect(() => Tensor.arange(0, Number.NaN)).toThrow(RangeError);
    expect(() => Tensor.arange(Number.NEGATIVE_INFINITY, 0, -1)).toThrow(RangeError);
  });

  it("fromArray() is equivalent to the constructor", () => {
    const a = Tensor.fromArray([[1, 2], [3, 4]]);
    const b = new Tensor([[1, 2], [3, 4]]);
    expect(a.equals(b)).toBe(true);
  });

  it("randn(shape) respects the requested shape", () => {
    const t = Tensor.randn([3, 4], 42);
    expect(t.shape).toEqual([3, 4]);
    expect(t.numel).toBe(12);
    // Values aren't all the same (extremely unlikely with N(0,1) samples).
    const unique = new Set(t.toArray());
    expect(unique.size).toBeGreaterThan(1);
  });

  it("randn(seed) is deterministic for the same seed", () => {
    const a = Tensor.randn([5], 123);
    const b = Tensor.randn([5], 123);
    expect(a.toArray()).toEqual(b.toArray());
  });

  it("randn(no seed) is non-deterministic", () => {
    const a = Tensor.randn([5]);
    const b = Tensor.randn([5]);
    expect(a.toArray()).not.toEqual(b.toArray());
  });

  it("randn mean is roughly zero (sanity check on Box-Muller)", () => {
    // 1000 samples — |mean| should be well under 0.2 for N(0, 1).
    const t = Tensor.randn([1000], 7);
    const arr = t.toArray();
    const mean = arr.reduce((a, b) => a + b, 0) / arr.length;
    expect(Math.abs(mean)).toBeLessThan(0.2);
  });
});

describe("Tensor — shape ops", () => {
  it("reshape preserves data", () => {
    const t = Tensor.arange(6).reshape([2, 3]);
    expect(t.shape).toEqual([2, 3]);
    expect(t.toArray()).toEqual([0, 1, 2, 3, 4, 5]);
  });

  it("reshape with mismatched numel throws", () => {
    expect(() => Tensor.arange(6).reshape([2, 4])).toThrow(RangeError);
  });

  it("reshape round-trips through new shape", () => {
    const t = Tensor.arange(12);
    expect(t.equals(t.reshape([3, 4]).reshape([12]))).toBe(true);
  });

  it("flatten produces a 1-D tensor", () => {
    const t = new Tensor([[1, 2], [3, 4]]).flatten();
    expect(t.shape).toEqual([4]);
    expect(t.toArray()).toEqual([1, 2, 3, 4]);
  });

  it("transpose 2-D default swaps the two dims", () => {
    const t = new Tensor([[1, 2, 3], [4, 5, 6]]).transpose();
    expect(t.shape).toEqual([3, 2]);
    expect(t.toArray()).toEqual([1, 4, 2, 5, 3, 6]);
  });

  it("transpose with explicit perm", () => {
    const t = new Tensor([[1, 2], [3, 4]]).transpose(1, 0);
    expect(t.toArray()).toEqual([1, 3, 2, 4]);
  });

  it("transpose twice is the identity", () => {
    const t = new Tensor([[1, 2, 3], [4, 5, 6]]);
    expect(t.equals(t.transpose().transpose())).toBe(true);
  });

  it("transpose rejects invalid perm", () => {
    const t = new Tensor([[1, 2], [3, 4]]);
    expect(() => t.transpose(0, 0)).toThrow(RangeError);
    expect(() => t.transpose(0, 1, 2)).toThrow(RangeError);
  });

  it("transpose 3-D reverses all dims", () => {
    // shape (2, 3, 4) with values 0..23 → perm (2, 1, 0) gives shape (4, 3, 2)
    const data = Array.from({ length: 24 }, (_, i) => i);
    const t = new Tensor(data, { shape: [2, 3, 4] });
    const u = t.transpose(2, 1, 0);
    expect(u.shape).toEqual([4, 3, 2]);
    // out[i, j, k] = in[k, j, i].  Spot-check a few:
    //   out[0, 0, 0] = in[0, 0, 0] = 0
    //   out[3, 2, 1] = in[1, 2, 3] = 1*12 + 2*4 + 3 = 23
    //   out[1, 0, 1] = in[1, 0, 1] = 1*12 + 0*4 + 1 = 13
    const flat = u.toArray();
    // out stride is (3*2, 2, 1) = (6, 2, 1)
    expect(flat[0 * 6 + 0 * 2 + 0]).toBe(0);
    expect(flat[3 * 6 + 2 * 2 + 1]).toBe(23);
    expect(flat[1 * 6 + 0 * 2 + 1]).toBe(13);
  });

  it("transpose 3-D with arbitrary perm", () => {
    // (2, 3, 4) → perm (1, 2, 0) gives (3, 4, 2): out[a, b, c] = in[c, a, b]
    const data = Array.from({ length: 24 }, (_, i) => i);
    const t = new Tensor(data, { shape: [2, 3, 4] });
    const u = t.transpose(1, 2, 0);
    expect(u.shape).toEqual([3, 4, 2]);
    // out stride (4*2, 2, 1) = (8, 2, 1); in stride (3*4, 4, 1) = (12, 4, 1)
    const flat = u.toArray();
    // out[2, 3, 1] = in[1, 2, 3] = 12 + 8 + 3 = 23 → at out index 2*8+3*2+1 = 23
    expect(flat[2 * 8 + 3 * 2 + 1]).toBe(23);
    // out[0, 0, 1] = in[1, 0, 0] = 12 → at out index 1
    expect(flat[1]).toBe(12);
  });

  it("transpose 4-D round-trip with inverse perm is the identity", () => {
    const data = Array.from({ length: 2 * 3 * 4 * 5 }, (_, i) => i);
    const t = new Tensor(data, { shape: [2, 3, 4, 5] });
    // perm (2, 0, 3, 1); inverse satisfies inv[perm[i]] = i → inv = (1, 3, 0, 2)
    const u = t.transpose(2, 0, 3, 1);
    expect(u.shape).toEqual([4, 2, 5, 3]);
    const back = u.transpose(1, 3, 0, 2);
    expect(back.shape).toEqual([2, 3, 4, 5]);
    expect(back.toArray()).toEqual(t.toArray());
  });

  it("transpose with identity perm is the identity", () => {
    const t = new Tensor(Array.from({ length: 24 }, (_, i) => i), { shape: [2, 3, 4] });
    expect(t.transpose(0, 1, 2).toArray()).toEqual(t.toArray());
  });

  it("squeeze with no arg drops all size-1 dims", () => {
    const t = Tensor.zeros(1, 3, 1, 2);
    expect(t.squeeze().shape).toEqual([3, 2]);
  });

  it("squeeze with explicit axis drops only that axis", () => {
    const t = Tensor.zeros(1, 3, 1);
    expect(t.squeeze(0).shape).toEqual([3, 1]);
    expect(t.squeeze(2).shape).toEqual([1, 3]);
  });

  it("squeeze with negative axis", () => {
    const t = Tensor.zeros(3, 1);
    expect(t.squeeze(-1).shape).toEqual([3]);
  });

  it("squeeze of non-unit axis throws", () => {
    expect(() => Tensor.zeros(3, 2).squeeze(0)).toThrow(RangeError);
  });

  it("unsqueeze inserts size-1 axis", () => {
    const t = Tensor.zeros(3);
    expect(t.unsqueeze(0).shape).toEqual([1, 3]);
    expect(t.unsqueeze(1).shape).toEqual([3, 1]);
    expect(t.unsqueeze(-1).shape).toEqual([3, 1]);
  });

  it("unsqueeze then squeeze round-trips", () => {
    const t = Tensor.arange(6).reshape([2, 3]);
    expect(t.equals(t.unsqueeze(0).squeeze(0))).toBe(true);
  });
});

describe("Tensor — element-wise math", () => {
  it("add tensors", () => {
    const a = new Tensor([1, 2, 3]);
    const b = new Tensor([10, 20, 30]);
    expect(a.add(b).toArray()).toEqual([11, 22, 33]);
  });

  it("add scalar broadcasts", () => {
    expect(new Tensor([1, 2, 3]).add(10).toArray()).toEqual([11, 12, 13]);
  });

  it("sub", () => {
    expect(new Tensor([5, 7, 9]).sub(new Tensor([1, 2, 3])).toArray()).toEqual([4, 5, 6]);
  });

  it("mul", () => {
    expect(new Tensor([2, 3, 4]).mul(new Tensor([5, 6, 7])).toArray()).toEqual([10, 18, 28]);
  });

  it("div", () => {
    expect(new Tensor([10, 20, 30]).div(new Tensor([2, 4, 5])).toArray()).toEqual([5, 5, 6]);
  });

  it("pow with scalar exponent", () => {
    expect(new Tensor([2, 3, 4]).pow(2).toArray()).toEqual([4, 9, 16]);
  });

  it("neg flips signs", () => {
    expect(new Tensor([1, -2, 3]).neg().toArray()).toEqual([-1, 2, -3]);
  });

  it("shape mismatch throws RangeError", () => {
    expect(() => Tensor.zeros(2, 3).add(Tensor.zeros(3, 2))).toThrow(RangeError);
  });

  it("unsupported operand throws TypeError", () => {
    expect(() => Tensor.zeros(2).add("not a number" as never)).toThrow(TypeError);
  });
});

describe("Tensor — equality and toString", () => {
  it("equals compares shape and data", () => {
    expect(new Tensor([1, 2, 3]).equals(new Tensor([1, 2, 3]))).toBe(true);
    expect(new Tensor([1, 2, 3]).equals(new Tensor([1, 2, 4]))).toBe(false);
    expect(new Tensor([1, 2, 3]).equals(new Tensor([1, 2, 3], { shape: [3, 1] }))).toBe(false);
  });

  it("equalsClose handles f32-precision noise", () => {
    const a = new Tensor([1.0, 2.0]);
    const b = new Tensor([1.0 + 1e-7, 2.0 - 1e-7]);
    expect(a.equals(b)).toBe(false);
    expect(a.equalsClose(b, 1e-6)).toBe(true);
  });

  it("toString includes shape and dtype", () => {
    const s = Tensor.zeros(2, 3).toString();
    expect(s).toContain("shape=[2, 3]");
    expect(s).toContain("dtype=f32");
  });

  it("toString truncates long data", () => {
    const s = Tensor.arange(100).toString();
    expect(s).toContain("…");
  });
});

describe("Tensor — round-trip properties", () => {
  it("toNested round-trips 2-D", () => {
    const original = [[1, 2, 3], [4, 5, 6]];
    expect(new Tensor(original).toNested()).toEqual(original);
  });

  it("toNested round-trips 3-D", () => {
    const original = [[[1, 2], [3, 4]], [[5, 6], [7, 8]]];
    expect(new Tensor(original).toNested()).toEqual(original);
  });

  it("reshape preserves toArray", () => {
    const t = Tensor.arange(12);
    expect(t.toArray()).toEqual(t.reshape([3, 4]).toArray());
  });
});

describe("Tensor — helper functions", () => {
  it("inferShape returns [] for non-arrays", () => {
    expect(inferShape(5)).toEqual([]);
    expect(inferShape(null)).toEqual([]);
  });

  it("inferShape returns [0] for empty arrays", () => {
    expect(inferShape([])).toEqual([0]);
  });

  it("inferShape handles 3-D nesting", () => {
    expect(inferShape([[[1, 2], [3, 4]], [[5, 6], [7, 8]]])).toEqual([2, 2, 2]);
  });

  it("flattenToFloat32 enforces expected length", () => {
    expect(() => flattenToFloat32([1, 2, 3], 4)).toThrow(RangeError);
  });

  it("flattenToFloat32 handles nested input", () => {
    expect(Array.from(flattenToFloat32([[1, 2], [3, 4]], 4))).toEqual([1, 2, 3, 4]);
  });
});

describe("Version", () => {
  it("VERSION is defined and well-formed", () => {
    expect(typeof VERSION).toBe("string");
    expect(VERSION).toMatch(/^\d+\.\d+\.\d+$/);
  });
});
