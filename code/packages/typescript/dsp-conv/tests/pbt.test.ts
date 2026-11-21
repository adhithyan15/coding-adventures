import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";
import {
  ConvMode,
  conv1d,
  convolve,
  fftConvolve,
  overlapAdd,
} from "../src/index.js";

function nextUnit(rng: SplitMix64): number {
  return Number(rng.nextU64() >> 11n) / 9007199254740992.0;
}

function nextRange(rng: SplitMix64, min: number, max: number): number {
  return min + (max - min) * nextUnit(rng);
}

function randomArray(rng: SplitMix64, len: number, min: number = -10, max: number = 10): Float64Array {
  const arr = new Float64Array(len);
  for (let i = 0; i < len; i++) {
    arr[i] = nextRange(rng, min, max);
  }
  return arr;
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
  const rng = new SplitMix64(987654321n);

  it("Commutativity: convolve(a, b) == convolve(b, a)", () => {
    for (let iter = 0; iter < 15; iter++) {
      const lenA = 3 + rng.bounded(17);
      const lenB = 3 + rng.bounded(17);
      const a = randomArray(rng, lenA);
      const b = randomArray(rng, lenB);

      const ab = convolve(a, b, ConvMode.FULL);
      const ba = convolve(b, a, ConvMode.FULL);
      assertMaxDiff(ab, ba, 1e-11);
    }
  });

  it("Linearity / Distributivity: convolve(a + b, c) == convolve(a, c) + convolve(b, c)", () => {
    for (let iter = 0; iter < 15; iter++) {
      const len = 5 + rng.bounded(13);
      const lenC = 3 + rng.bounded(7);
      const a = randomArray(rng, len);
      const b = randomArray(rng, len);
      const c = randomArray(rng, lenC);

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
      const len = 4 + rng.bounded(26);
      const signal = randomArray(rng, len);
      const out = conv1d(signal, [1.0]);
      assertMaxDiff(out, signal, 1e-12);
    }
  });

  it("Cross-algorithm equivalence: direct == FFT == overlapAdd", () => {
    for (let iter = 0; iter < 10; iter++) {
      const lenSig = 16 + rng.bounded(48);
      const lenKer = 4 + rng.bounded(12);
      const sig = randomArray(rng, lenSig);
      const ker = randomArray(rng, lenKer);

      const direct = convolve(sig, ker, ConvMode.FULL);
      const fftRes = fftConvolve(sig, ker, ConvMode.FULL);
      const olaRes = overlapAdd(sig, ker, 16, ConvMode.FULL);

      assertMaxDiff(fftRes, direct, 1e-9);
      assertMaxDiff(olaRes, direct, 1e-9);
    }
  });
});
