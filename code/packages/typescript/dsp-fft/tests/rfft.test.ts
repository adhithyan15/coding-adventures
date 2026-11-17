import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  InvalidInputError,
  irfftScalar,
  rfftScalar,
} from "../src/index.js";

describe("Real-input FFT (rfft and irfft)", () => {
  it("computes rfft on even length power-of-two (N = 8) returning N/2 + 1 = 5 bins", () => {
    const signal = [1, 2, 3, 4, 3, 2, 1, 0];
    const half = rfftScalar(signal);
    assert.equal(half.length, 2 * 5); // 5 complex bins

    const recovered = irfftScalar(half, 8);
    assert.equal(recovered.length, 8);
    for (let i = 0; i < 8; i++) {
      assert.ok(
        Math.abs(recovered[i] - signal[i]) < 1e-11,
        `mismatch at ${i}: expected ${signal[i]}, got ${recovered[i]}`
      );
    }
  });

  it("computes rfft on odd length non-pow2 (N = 7) returning N/2 + 1 = 4 bins", () => {
    const signal = [2.0, -1.0, 3.5, 0.0, 4.0, -2.5, 1.0];
    const half = rfftScalar(signal);
    assert.equal(half.length, 2 * 4); // 4 complex bins

    const recovered = irfftScalar(half, 7);
    assert.equal(recovered.length, 7);
    for (let i = 0; i < 7; i++) {
      assert.ok(Math.abs(recovered[i] - signal[i]) < 1e-11);
    }
  });

  it("rejects mismatched half-spectrum and output length in irfft", () => {
    const half = rfftScalar([1, 2, 3, 4]); // 3 bins
    assert.throws(() => irfftScalar(half, 6), InvalidInputError);
    assert.throws(() => irfftScalar(half, 0), InvalidInputError);
    assert.throws(() => irfftScalar([1, 2, 3], 4), InvalidInputError); // odd length
  });
});
