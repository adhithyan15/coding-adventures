import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  BoundaryMode,
  ConvMode,
  conv1d,
  convolve,
  correlate,
  extendIndex,
  sample,
} from "../src/index.js";

function assertClose(a: ArrayLike<number>, b: ArrayLike<number>, tol = 1e-10): void {
  assert.equal(a.length, b.length, `length mismatch: ${a.length} vs ${b.length}`);
  for (let i = 0; i < a.length; i++) {
    const diff = Math.abs(Number(a[i]) - Number(b[i]));
    assert.ok(
      diff <= tol,
      `mismatch at ${i}: ${a[i]} vs ${b[i]} (diff: ${diff})`
    );
  }
}

describe("1-D Convolution (conv1d, convolve, correlate)", () => {
  it("extendIndex handles in-bounds and boundary extensions correctly", () => {
    // In-bounds
    assert.equal(extendIndex(2, 5, BoundaryMode.ZERO), 2);
    // Out of bounds: ZERO
    assert.equal(extendIndex(-1, 5, BoundaryMode.ZERO), null);
    assert.equal(extendIndex(5, 5, BoundaryMode.ZERO), null);

    // Out of bounds: REPLICATE
    assert.equal(extendIndex(-3, 5, BoundaryMode.REPLICATE), 0);
    assert.equal(extendIndex(7, 5, BoundaryMode.REPLICATE), 4);

    // Out of bounds: WRAP
    assert.equal(extendIndex(-1, 5, BoundaryMode.WRAP), 4);
    assert.equal(extendIndex(5, 5, BoundaryMode.WRAP), 0);
    assert.equal(extendIndex(6, 5, BoundaryMode.WRAP), 1);

    // Out of bounds: REFLECT
    assert.equal(extendIndex(-1, 4, BoundaryMode.REFLECT), 1);
    assert.equal(extendIndex(-2, 4, BoundaryMode.REFLECT), 2);
    assert.equal(extendIndex(4, 4, BoundaryMode.REFLECT), 2);
    assert.equal(extendIndex(5, 4, BoundaryMode.REFLECT), 1);

    // Single element REFLECT
    assert.equal(extendIndex(3, 1, BoundaryMode.REFLECT), 0);
  });

  it("sample reads valid and out-of-bounds correctly", () => {
    const sig = [10, 20, 30];
    assert.equal(sample(sig, 1, BoundaryMode.ZERO), 20);
    assert.equal(sample(sig, -1, BoundaryMode.ZERO), 0);
    assert.equal(sample(sig, -1, BoundaryMode.REPLICATE), 10);
    assert.equal(sample(sig, 3, BoundaryMode.REPLICATE), 30);
  });

  it("conv1d preserves length and identity kernel returns signal", () => {
    const signal = [1.0, 2.0, 3.0, 4.0, 5.0];
    const identityKernel = [1.0];
    for (const mode of [
      BoundaryMode.ZERO,
      BoundaryMode.REPLICATE,
      BoundaryMode.REFLECT,
      BoundaryMode.WRAP,
    ]) {
      const out = conv1d(signal, identityKernel, mode);
      assertClose(out, signal);
    }
  });

  it("conv1d centred delta preserves signal", () => {
    const signal = [1.0, 2.0, 3.0, 4.0, 5.0];
    const kernel = [0.0, 1.0, 0.0];
    const out = conv1d(signal, kernel, BoundaryMode.ZERO);
    assertClose(out, signal);
  });

  it("conv1d performs same-size moving average", () => {
    const signal = [0, 0, 1, 1, 1, 0, 0];
    const kernel = [1 / 3, 1 / 3, 1 / 3];
    const out = conv1d(signal, kernel, BoundaryMode.ZERO);
    assert.equal(out.length, signal.length);
    assertClose(out, [0, 1 / 3, 2 / 3, 1.0, 2 / 3, 1 / 3, 0]);
  });

  it("convolve supports FULL, SAME, and VALID modes", () => {
    const signal = [1, 2, 3];
    const kernel = [0.5, 1.0];

    // Full: length = 3 + 2 - 1 = 4
    const full = convolve(signal, kernel, ConvMode.FULL);
    assert.equal(full.length, 4);
    assertClose(full, [0.5, 2.0, 3.5, 3.0]);

    // Same: length = 3
    const same = convolve(signal, kernel, ConvMode.SAME);
    assert.equal(same.length, 3);
    assert.equal(same.length, signal.length);

    // Valid: length = 3 - 2 + 1 = 2
    const valid = convolve(signal, kernel, ConvMode.VALID);
    assert.equal(valid.length, 2);
    assertClose(valid, [2.0, 3.5]);
  });

  it("correlate computes cross-correlation correctly", () => {
    const sig = [1, 2, 3, 4, 5];
    const pat = [2, 3];
    const corr = correlate(sig, pat, ConvMode.VALID);
    // For real sequences, correlate(sig, pat) with valid mode
    // (sig * rev(pat))_valid
    assert.equal(corr.length, sig.length - pat.length + 1);
    // corr[0] = sig[0]*pat[0] + sig[1]*pat[1] = 1*2 + 2*3 = 8
    // corr[1] = 2*2 + 3*3 = 13
    // corr[2] = 3*2 + 4*3 = 18
    // corr[3] = 4*2 + 5*3 = 23
    assertClose(corr, [8, 13, 18, 23]);
  });
});
