import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ConvMode,
  convolve,
  overlapAdd,
  overlap_add,
} from "../src/index.js";

function assertClose(a: ArrayLike<number>, b: ArrayLike<number>, tol = 1e-9): void {
  assert.equal(a.length, b.length, `length mismatch: ${a.length} vs ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    const diff = Math.abs(Number(a[i]) - Number(b[i]));
    assert.ok(
      diff <= tol,
      `mismatch at ${i}: ${a[i]} vs ${b[i]} (diff: ${diff})`
    );
  }
}

describe("Overlap-Add Convolution (overlapAdd)", () => {
  it("matches full direct convolution across various block sizes", () => {
    const n = 120;
    const sig = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      sig[i] = (i % 7) - 3.0;
    }
    const ker = [0.1, 0.2, 0.4, 0.2, 0.1];

    const expected = convolve(sig, ker, ConvMode.FULL);

    for (const bSize of [16, 32, 64, null]) {
      const res = overlapAdd(sig, ker, bSize, ConvMode.FULL);
      const alias = overlap_add(sig, ker, bSize, ConvMode.FULL);
      assertClose(res, expected);
      assertClose(alias, expected);
    }
  });

  it("supports SAME and VALID modes correctly", () => {
    const sig = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    const ker = [0.5, 0.5];

    const expectedSame = convolve(sig, ker, ConvMode.SAME);
    const resSame = overlapAdd(sig, ker, 4, ConvMode.SAME);
    assertClose(resSame, expectedSame);

    const expectedValid = convolve(sig, ker, ConvMode.VALID);
    const resValid = overlapAdd(sig, ker, 4, ConvMode.VALID);
    assertClose(resValid, expectedValid);
  });
});
