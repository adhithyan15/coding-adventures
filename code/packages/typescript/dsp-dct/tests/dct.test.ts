import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  dct,
  DctNorm,
  DctType,
  idct,
} from "../src/index.js";

describe("1-D Discrete Cosine Transform (DCT-I..IV)", () => {
  it("DCT-II forward and inverse roundtrip with ORTHO norm", () => {
    const signal = [1, 2, 3, 4, 5, 6, 7, 8];
    const coeffs = dct(signal, DctType.II, DctNorm.ORTHO);
    const recovered = idct(coeffs, DctType.III, DctNorm.ORTHO);

    assert.equal(recovered.length, signal.length);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(recovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DCT-II forward and inverse roundtrip with NONE norm", () => {
    const signal = [3.5, -2.1, 4.0, 1.2, -0.8];
    const coeffs = dct(signal, DctType.II, DctNorm.NONE);
    const recovered = idct(coeffs, DctType.III, DctNorm.NONE);

    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(recovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DCT-III forward and inverse roundtrip (inverting with DCT-II)", () => {
    const signal = [1.0, 2.0, 3.0, 4.0];
    const coeffs = dct(signal, DctType.III, DctNorm.ORTHO);
    const recovered = idct(coeffs, DctType.II, DctNorm.ORTHO);

    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(recovered[i] - signal[i]) < 1e-11);
    }

    const unnormCoeffs = dct(signal, DctType.III, DctNorm.NONE);
    const unnormRecovered = idct(unnormCoeffs, DctType.II, DctNorm.NONE);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(unnormRecovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DCT-I forward and inverse roundtrip with ORTHO and NONE norms", () => {
    const signal = [1.0, 3.0, -2.0, 5.0];
    const orthoCoeffs = dct(signal, DctType.I, DctNorm.ORTHO);
    const orthoRecovered = idct(orthoCoeffs, DctType.I, DctNorm.ORTHO);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(orthoRecovered[i] - signal[i]) < 1e-11);
    }

    const rawCoeffs = dct(signal, DctType.I, DctNorm.NONE);
    const rawRecovered = idct(rawCoeffs, DctType.I, DctNorm.NONE);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(rawRecovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DCT-IV forward and inverse roundtrip with ORTHO and NONE norms", () => {
    const signal = [2.0, -1.0, 3.0, 0.5];
    const orthoCoeffs = dct(signal, DctType.IV, DctNorm.ORTHO);
    const orthoRecovered = idct(orthoCoeffs, DctType.IV, DctNorm.ORTHO);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(orthoRecovered[i] - signal[i]) < 1e-11);
    }

    const rawCoeffs = dct(signal, DctType.IV, DctNorm.NONE);
    const rawRecovered = idct(rawCoeffs, DctType.IV, DctNorm.NONE);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(rawRecovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DCT-II DC energy compaction property", () => {
    // Constant signal DC energy should be concentrated almost entirely in bin 0
    const dc = [2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0, 2.0];
    const coeffs = dct(dc, DctType.II, DctNorm.ORTHO);
    // X[0] for DC = 2 * sqrt(N) = 2 * sqrt(8) ≈ 5.656854
    assert.ok(Math.abs(coeffs[0] - 2.0 * Math.sqrt(8)) < 1e-11);
    for (let i = 1; i < 8; i++) {
      assert.ok(Math.abs(coeffs[i]) < 1e-11);
    }
  });
});
