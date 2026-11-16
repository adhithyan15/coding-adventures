import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  fftScalar,
  ifftScalar,
  NotPowerOfTwoError,
} from "../src/index.js";

describe("Radix-2 Cooley-Tukey FFT", () => {
  it("transforms impulse: fft([1, 0, 0, 0]) -> [1, 1, 1, 1]", () => {
    // Interleaved [re, im]: [1, 0, 0, 0, 0, 0, 0, 0] (length 4)
    const impulse = [1, 0, 0, 0, 0, 0, 0, 0];
    const spec = fftScalar(impulse);
    assert.equal(spec.length, 8);
    for (let k = 0; k < 4; k++) {
      assert.ok(Math.abs(spec[2 * k] - 1.0) < 1e-12);
      assert.ok(Math.abs(spec[2 * k + 1]) < 1e-12);
    }
  });

  it("transforms DC signal: fft([1, 1, 1, 1]) -> [4, 0, 0, 0]", () => {
    const dc = [1, 0, 1, 0, 1, 0, 1, 0];
    const spec = fftScalar(dc);
    assert.ok(Math.abs(spec[0] - 4.0) < 1e-12);
    assert.ok(Math.abs(spec[1]) < 1e-12);
    for (let k = 1; k < 4; k++) {
      assert.ok(Math.abs(spec[2 * k]) < 1e-12);
      assert.ok(Math.abs(spec[2 * k + 1]) < 1e-12);
    }
  });

  it("performs forward-then-inverse round trip", () => {
    const n = 16;
    const signal: number[] = [];
    for (let i = 0; i < n; i++) {
      signal.push(Math.sin((2 * Math.PI * 3 * i) / n));
      signal.push(Math.cos((2 * Math.PI * 2 * i) / n));
    }
    const spec = fftScalar(signal);
    const recovered = ifftScalar(spec);

    for (let i = 0; i < 2 * n; i++) {
      assert.ok(
        Math.abs(recovered[i] - signal[i]) < 1e-12,
        `mismatch at ${i}: expected ${signal[i]}, got ${recovered[i]}`
      );
    }
  });

  it("rejects non-power-of-two lengths", () => {
    // 3 complex elements (length 6)
    const nonPow2 = [1, 0, 2, 0, 3, 0];
    assert.throws(
      () => fftScalar(nonPow2),
      (err: unknown) => err instanceof NotPowerOfTwoError && err.n === 3
    );
    assert.throws(
      () => ifftScalar(nonPow2),
      (err: unknown) => err instanceof NotPowerOfTwoError && err.n === 3
    );
  });
});
