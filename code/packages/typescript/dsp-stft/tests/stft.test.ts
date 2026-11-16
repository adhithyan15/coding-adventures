import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { stft, WindowType } from "../src/index.js";

describe("Short-Time Fourier Transform (stft)", () => {
  it("computes STFT with strict-mode framing shape", () => {
    const n = 1024;
    const nFft = 256;
    const hop = 128;
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      signal[i] = Math.sin((2 * Math.PI * i * 4) / nFft);
    }

    const spec = stft(signal, nFft, hop, WindowType.HANN);
    // Expected frames = 1 + (1024 - 256) / 128 = 7
    const expectedFrames = 7;
    const bins = nFft / 2 + 1; // 129
    assert.equal(spec.length, expectedFrames * bins * 2);
  });

  it("peaks at the correct bin for a pure sine tone", () => {
    const n = 512;
    const nFft = 128;
    const hop = 64;
    const targetBin = 8;
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      signal[i] = Math.cos((2 * Math.PI * targetBin * i) / nFft);
    }

    const spec = stft(signal, nFft, hop, WindowType.RECTANGULAR);
    const bins = nFft / 2 + 1;
    // Check frame 2
    const frame2Offset = 2 * (bins * 2);
    let maxMag = 0.0;
    let maxBin = -1;
    for (let k = 0; k < bins; k++) {
      const re = spec[frame2Offset + 2 * k];
      const im = spec[frame2Offset + 2 * k + 1];
      const mag = Math.sqrt(re * re + im * im);
      if (mag > maxMag) {
        maxMag = mag;
        maxBin = k;
      }
    }
    assert.equal(maxBin, targetBin);
  });
});
