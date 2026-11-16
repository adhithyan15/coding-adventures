import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  Complex,
  ComplexArray,
  ComplexTensor,
  InvalidInputError,
  LengthMismatchError,
  OddInterleavedLengthError,
} from "../src/index.js";

describe("ComplexArray operations", () => {
  it("initializes empty array and checks isEmpty and length", () => {
    const empty = new ComplexArray();
    assert.equal(empty.length, 0);
    assert.ok(empty.isEmpty());
    assert.deepEqual(empty.shape, [0, 2]);
    assert.equal(empty.asInterleaved().length, 0);
  });

  it("constructs from real array with zeros for imaginary part", () => {
    const arr = ComplexArray.fromReal([1, 2, 3]);
    assert.equal(arr.length, 3);
    assert.deepEqual(Array.from(arr.real()), [1, 2, 3]);
    assert.deepEqual(Array.from(arr.imag()), [0, 0, 0]);
    assert.deepEqual(Array.from(arr.asInterleaved()), [1, 0, 2, 0, 3, 0]);
  });

  it("constructs from paired real and imag arrays", () => {
    const arr = ComplexArray.fromRealImag([1, 2], [3, 4]);
    assert.equal(arr.length, 2);
    assert.deepEqual(Array.from(arr.real()), [1, 2]);
    assert.deepEqual(Array.from(arr.imag()), [3, 4]);
    assert.deepEqual(Array.from(arr.asInterleaved()), [1, 3, 2, 4]);

    const fromInter = ComplexArray.fromInterleaved([1, 3, 2, 4]);
    assert.ok(arr.equals(fromInter));
  });

  it("constructs from zeros", () => {
    const arr = ComplexArray.zeros(4);
    assert.equal(arr.length, 4);
    for (let i = 0; i < 4; i++) {
      const c = arr.get(i);
      assert.equal(c.re, 0);
      assert.equal(c.im, 0);
    }
  });

  it("gets and sets elements with negative index support", () => {
    const arr = ComplexArray.fromRealImag([10, 20, 30], [1, 2, 3]);
    const first = arr.get(0);
    assert.equal(first.re, 10);
    assert.equal(first.im, 1);

    const last = arr.get(-1);
    assert.equal(last.re, 30);
    assert.equal(last.im, 3);

    arr.set(1, new Complex(99, 88));
    assert.equal(arr.get(1).re, 99);
    assert.equal(arr.get(1).im, 88);

    arr.set(-1, { re: 77, im: 66 });
    assert.equal(arr.get(2).re, 77);
    assert.equal(arr.get(2).im, 66);
  });

  it("calculates element-wise magnitude and phase", () => {
    const arr = ComplexArray.fromRealImag([3, 0], [4, 2]);
    const mags = arr.magnitude();
    assert.equal(mags[0], 5);
    assert.equal(mags[1], 2);

    const phases = arr.phase();
    assert.ok(Math.abs(phases[0] - Math.atan2(4, 3)) < 1e-12);
    assert.ok(Math.abs(phases[1] - Math.PI / 2) < 1e-12);
  });

  it("calculates conjugate, scale, and arithmetic", () => {
    const a = ComplexArray.fromRealImag([1, 2], [3, 4]);
    const conj = a.conjugate();
    assert.deepEqual(Array.from(conj.imag()), [-3, -4]);

    // conjugate twice is identity (claim from warrant)
    const conj2 = conj.conjugate();
    assert.ok(a.equals(conj2));

    const scaled = a.scale(2);
    assert.deepEqual(Array.from(scaled.real()), [2, 4]);
    assert.deepEqual(Array.from(scaled.imag()), [6, 8]);

    const b = ComplexArray.fromRealImag([5, 6], [7, 8]);
    const sum = a.add(b);
    assert.deepEqual(Array.from(sum.real()), [6, 8]);
    assert.deepEqual(Array.from(sum.imag()), [10, 12]);

    const diff = b.sub(a);
    assert.deepEqual(Array.from(diff.real()), [4, 4]);
    assert.deepEqual(Array.from(diff.imag()), [4, 4]);

    const prod = a.mul(b);
    // (1 + 3i)(5 + 7i) = 5 - 21 + 7i + 15i = -16 + 22i
    // (2 + 4i)(6 + 8i) = 12 - 32 + 16i + 24i = -20 + 40i
    assert.deepEqual(Array.from(prod.real()), [-16, -20]);
    assert.deepEqual(Array.from(prod.imag()), [22, 40]);

    const quot = prod.div(b);
    assert.ok(quot.equals(a, 1e-10));
  });

  it("supports iteration and toComplexList", () => {
    const arr = ComplexArray.fromRealImag([1, 2], [3, 4]);
    const list = arr.toComplexList();
    assert.equal(list.length, 2);
    assert.equal(list[0].re, 1);
    assert.equal(list[1].im, 4);

    const collected: Complex[] = [];
    for (const c of arr) {
      collected.push(c);
    }
    assert.equal(collected.length, 2);
    assert.equal(collected[0].re, 1);
  });

  it("validates ComplexTensor alias parity", () => {
    assert.equal(ComplexTensor, ComplexArray);
    const tensor = ComplexTensor.fromReal([1, 2]);
    assert.ok(tensor instanceof ComplexArray);
  });
});
