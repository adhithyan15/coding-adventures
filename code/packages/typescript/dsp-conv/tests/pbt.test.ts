import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ConvMode,
  conv1d,
  convolve,
  fftConvolve,
  overlapAdd,
} from "../src/index.js";

class SimpleRng {
  private state: bigint;
  constructor(seed: bigint = 123456789n) {
    this.state = seed;
  }
  next(): number {
    this.state = (this.state * 6364136223846793005n + 1442695040888963407n) & 0xffffffffffffffffn;
    return Number(this.state >> 32n) / 4294967296.0;
  }
  range(min: number, max: number): number {
    return min + (max - min) * this.next();
  }
  array(len: number, min: number = -10, max: number = 10): Float64Array {
    const arr = new Float64Array(len);
    for (let i = 0; i < len; i++) {
      arr[i] = this.range(min, max);
    }
    return arr;
  }
}

function assertMaxDiff(a: ArrayLike<number>, b: ArrayLike<number>, tol: number): void {
  assert.equal(a.length, b.length);
  for (let i = 0; i < a.length; i++) {
    const diff = Math.abs(Number(a[i]) - Number(b[i]));
    assert.ok(
      diff <= tol,
      `exceeded tolerance at ${i}: |${a[i]} - ${b[i]}| = ${diff} > ${tol}`
    );
  }
}

describe("Property-Based Testing (dsp-conv algebraic laws)", () => {
  const rng = new SimpleRng(987654321n);

  it("Commutativity: convolve(a, b) == convolve(b, a)", () => {
    for (let iter = 0; iter < 15; iter++) {
      const lenA = Math.floor(rng.range(3, 20));
      const lenB = Math.floor(rng.range(3, 20));
      const a = rng.array(lenA);
      const b = rng.array(lenB);

      const ab = convolve(a, b, ConvMode.FULL);
      const ba = convolve(b, a, ConvMode.FULL);
      assertMaxDiff(ab, ba, 1e-11);
    }
  });

  it("Linearity / Distributivity: convolve(a + b, c) == convolve(a, c) + convolve(b, c)", () => {
    for (let iter = 0; iter < 15; iter++) {
      const len = Math.floor(rng.range(5, 18));
      const lenC = Math.floor(rng.range(3, 10));
      const a = rng.array(len);
      const b = rng.array(len);
      const c = rng.array(lenC);

      const aPlusB = new Float64Array(len);
      for (let i = 0; i < len; i++) aPlusB[i] = a[i] + b[i];

      const lhs = convolve(aPlusB, c, ConvMode.FULL);
      const ac = convolve(a, c, ConvMode.FULL);
      const bc = convolve(b, c, ConvMode.FULL);

      const rhs = new Float64Array(lhs.length);
      for (let i = 0; i < rhs.length; i++) rhs[i] = ac[i] + bc[i];

      assertMaxDiff(lhs, rhs, 1e-10);
    }
  });

  it("Impulse Identity: convolve(signal, [1.0], SAME) == signal", () => {
    for (let iter = 0; iter < 15; iter++) {
      const len = Math.floor(rng.range(4, 30));
      const signal = rng.array(len);
      const out = conv1d(signal, [1.0]);
      assertMaxDiff(out, signal, 1e-12);
    }
  });

  it("Cross-algorithm equivalence: direct == FFT == overlapAdd", () => {
    for (let iter = 0; iter < 10; iter++) {
      const lenSig = Math.floor(rng.range(16, 64));
      const lenKer = Math.floor(rng.range(4, 16));
      const sig = rng.array(lenSig);
      const ker = rng.array(lenKer);

      const direct = convolve(sig, ker, ConvMode.FULL);
      const fftRes = fftConvolve(sig, ker, ConvMode.FULL);
      const olaRes = overlapAdd(sig, ker, 16, ConvMode.FULL);

      assertMaxDiff(fftRes, direct, 1e-9);
      assertMaxDiff(olaRes, direct, 1e-9);
    }
  });
});
