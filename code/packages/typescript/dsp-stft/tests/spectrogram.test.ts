import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  logSpectrogram,
  powerSpectrogram,
  spectrogram,
  WindowType,
} from "../src/index.js";

describe("Spectrograms (power and log)", () => {
  it("power spectrogram values are strictly non-negative", () => {
    const n = 512;
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      signal[i] = Math.sin(0.1 * i);
    }

    const pSpec = powerSpectrogram(signal, 128, 64, WindowType.HANN);
    const pAlias = spectrogram(signal, 128, 64, WindowType.HANN);

    assert.equal(pSpec.length, pAlias.length);
    for (let i = 0; i < pSpec.length; i++) {
      assert.ok(pSpec[i] >= 0.0);
      assert.equal(pSpec[i], pAlias[i]);
    }
  });

  it("logSpectrogram computes logarithm with finite epsilon floor", () => {
    const signal = new Float64Array(256); // All zeros
    const logSpec = logSpectrogram(signal, 64, 32, WindowType.HANN);
    // All silent bins should equal ln(1e-10) ≈ -23.02585
    for (let i = 0; i < logSpec.length; i++) {
      assert.ok(Number.isFinite(logSpec[i]));
      assert.ok(Math.abs(logSpec[i] - Math.log(1e-10)) < 1e-4);
    }
  });
});
