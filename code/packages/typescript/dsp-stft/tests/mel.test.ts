import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  hzToMel,
  melFilterbank,
  melSpectrogram,
  melToHz,
  mfcc,
  WindowType,
} from "../src/index.js";

describe("Mel scale, filterbank, and MFCC", () => {
  it("hzToMel and melToHz roundtrip cleanly", () => {
    for (const freq of [0.0, 100.0, 440.0, 1000.0, 4000.0, 8000.0]) {
      const mel = hzToMel(freq);
      const recon = melToHz(mel);
      assert.ok(Math.abs(recon - freq) < 1e-4);
    }
  });

  it("melFilterbank generates non-negative weights summing to 1.0", () => {
    const nMels = 40;
    const nFft = 512;
    const fs = 16000;
    const fb = melFilterbank(nMels, nFft, fs);
    const bins = nFft / 2 + 1;

    assert.equal(fb.length, nMels * bins);

    for (let m = 0; m < nMels; m++) {
      let rowSum = 0.0;
      for (let k = 0; k < bins; k++) {
        const w = fb[m * bins + k];
        assert.ok(w >= 0.0);
        rowSum += w;
      }
      // If row has support, must sum to 1.0
      if (rowSum > 0.0) {
        assert.ok(Math.abs(rowSum - 1.0) < 1e-10);
      }
    }
  });

  it("melSpectrogram outputs [numFrames, nMels] matrix", () => {
    const n = 1024;
    const nFft = 256;
    const hop = 128;
    const nMels = 20;
    const fs = 8000;
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) signal[i] = Math.sin((2 * Math.PI * 440 * i) / fs);

    const melSpec = melSpectrogram(signal, nFft, hop, nMels, fs, WindowType.HANN);
    const expectedFrames = 1 + (n - nFft) / hop; // 7
    assert.equal(melSpec.length, expectedFrames * nMels);

    for (let i = 0; i < melSpec.length; i++) {
      assert.ok(melSpec[i] >= 0.0);
    }
  });

  it("mfcc computes [numFrames, nMfcc] cepstral coefficients", () => {
    const n = 1024;
    const nFft = 256;
    const hop = 128;
    const nMels = 40;
    const nMfcc = 13;
    const fs = 16000;
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) signal[i] = Math.sin((2 * Math.PI * 880 * i) / fs);

    const coeffs = mfcc(signal, nFft, hop, nMels, nMfcc, fs, WindowType.HANN);
    const expectedFrames = 1 + (n - nFft) / hop; // 7
    assert.equal(coeffs.length, expectedFrames * nMfcc);

    for (let i = 0; i < coeffs.length; i++) {
      assert.ok(Number.isFinite(coeffs[i]));
    }
  });
});
