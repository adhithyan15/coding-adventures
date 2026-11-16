import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  ConvMode,
  convolve,
  fftConvolve,
  fft_convolve,
  nextPow2,
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

describe("FFT-based Convolution (fftConvolve)", () => {
  it("nextPow2 computes correct power-of-two ceil", () => {
    assert.equal(nextPow2(0), 1);
    assert.equal(nextPow2(1), 1);
    assert.equal(nextPow2(2), 2);
    assert.equal(nextPow2(3), 4);
    assert.equal(nextPow2(4), 4);
    assert.equal(nextPow2(5), 8);
    assert.equal(nextPow2(100), 128);
  });

  it("fftConvolve matches direct convolve in FULL mode", () => {
    const sig = [1.0, -2.0, 3.5, 0.5, -1.0, 2.0, 4.0, -3.0];
    const ker = [0.1, 0.2, 0.4, 0.2, 0.1];

    const direct = convolve(sig, ker, ConvMode.FULL);
    const fftRes = fftConvolve(sig, ker, ConvMode.FULL);
    const fftAlias = fft_convolve(sig, ker, ConvMode.FULL);

    assertClose(fftRes, direct);
    assertClose(fftAlias, direct);
  });

  it("fftConvolve matches direct convolve in SAME and VALID modes", () => {
    const sig = [1.0, 3.0, 2.0, -1.0, 4.0, -2.0, 5.0];
    const ker = [0.5, -0.5, 0.2];

    const directSame = convolve(sig, ker, ConvMode.SAME);
    const fftSame = fftConvolve(sig, ker, ConvMode.SAME);
    assertClose(fftSame, directSame);

    const directValid = convolve(sig, ker, ConvMode.VALID);
    const fftValid = fftConvolve(sig, ker, ConvMode.VALID);
    assertClose(fftValid, directValid);
  });

  it("handles long signals efficiently with accurate results", () => {
    const n = 256;
    const sig = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      sig[i] = Math.sin((2 * Math.PI * i) / 16);
    }
    const ker = [0.25, 0.5, 0.25];

    const direct = convolve(sig, ker, ConvMode.FULL);
    const fftRes = fftConvolve(sig, ker, ConvMode.FULL);
    assertClose(fftRes, direct);
  });
});
