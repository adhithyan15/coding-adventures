import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { istft, stft, WindowType } from "../src/index.js";

describe("Inverse STFT (istft) & Overlap-Add Reconstruction", () => {
  it("perfectly reconstructs periodic signal under COLA condition", () => {
    const n = 1024;
    const nFft = 256;
    const hop = 64; // 75% overlap satisfies COLA for Hann
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      signal[i] = Math.sin((2 * Math.PI * i * 3) / nFft) + 0.5 * Math.cos((2 * Math.PI * i * 7) / nFft);
    }

    const spec = stft(signal, nFft, hop, WindowType.HANN);
    const recon = istft(spec, nFft, hop, WindowType.HANN, n);

    assert.equal(recon.length, n);

    // Interior samples where full overlap-add coverage exists:
    // from nFft to n - nFft
    const start = nFft;
    const end = n - nFft;
    for (let i = start; i < end; i++) {
      const diff = Math.abs(recon[i] - signal[i]);
      assert.ok(
        diff < 1e-4,
        `reconstruction error at ${i}: expected ${signal[i]}, got ${recon[i]} (diff: ${diff})`
      );
    }
  });

  it("handles default outputLength matching frame bounds", () => {
    const signal = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    const nFft = 8;
    const hop = 4;
    const spec = stft(signal, nFft, hop, WindowType.RECTANGULAR);
    const recon = istft(spec, nFft, hop, WindowType.RECTANGULAR);
    assert.ok(recon.length > 0);
  });
});
