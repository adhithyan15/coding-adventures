import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  DctNorm,
  dst,
  DstType,
  idst,
} from "../src/index.js";

describe("Discrete Sine Transform (DST-I..IV)", () => {
  it("DST-I roundtrip with ORTHO and NONE norms", () => {
    const signal = [1.0, 2.0, 3.0, 4.0];
    const orthoCoeffs = dst(signal, DstType.I, DctNorm.ORTHO);
    const orthoRecovered = idst(orthoCoeffs, DstType.I, DctNorm.ORTHO);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(orthoRecovered[i] - signal[i]) < 1e-11);
    }

    const rawCoeffs = dst(signal, DstType.I, DctNorm.NONE);
    const rawRecovered = idst(rawCoeffs, DstType.I, DctNorm.NONE);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(rawRecovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DST-II and DST-III mutual inversion with ORTHO and NONE norms", () => {
    const signal = [2.0, -1.5, 3.0, 0.5];
    const orthoCoeffs = dst(signal, DstType.II, DctNorm.ORTHO);
    const orthoRecovered = idst(orthoCoeffs, DstType.III, DctNorm.ORTHO);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(orthoRecovered[i] - signal[i]) < 1e-11);
    }

    const rawCoeffs = dst(signal, DstType.II, DctNorm.NONE);
    const rawRecovered = idst(rawCoeffs, DstType.III, DctNorm.NONE);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(rawRecovered[i] - signal[i]) < 1e-11);
    }

    // Invert DST-III using DST-II
    const iiiCoeffs = dst(signal, DstType.III, DctNorm.ORTHO);
    const iiiRecovered = idst(iiiCoeffs, DstType.II, DctNorm.ORTHO);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(iiiRecovered[i] - signal[i]) < 1e-11);
    }
  });

  it("DST-IV roundtrip with ORTHO and NONE norms", () => {
    const signal = [1.0, 3.0, -1.0, 2.0];
    const orthoCoeffs = dst(signal, DstType.IV, DctNorm.ORTHO);
    const orthoRecovered = idst(orthoCoeffs, DstType.IV, DctNorm.ORTHO);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(orthoRecovered[i] - signal[i]) < 1e-11);
    }

    const rawCoeffs = dst(signal, DstType.IV, DctNorm.NONE);
    const rawRecovered = idst(rawCoeffs, DstType.IV, DctNorm.NONE);
    for (let i = 0; i < signal.length; i++) {
      assert.ok(Math.abs(rawRecovered[i] - signal[i]) < 1e-11);
    }
  });
});
