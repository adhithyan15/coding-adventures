import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { istft, stft, WindowType } from "../src/index.js";

class SimpleRng {
  private state: bigint;
  constructor(seed: bigint = 987654321n) {
    this.state = seed;
  }
  next(): number {
    this.state = (this.state * 6364136223846793005n + 1442695040888963407n) & 0xffffffffffffffffn;
    return Number(this.state >> 32n) / 4294967296.0;
  }
  range(min: number, max: number): number {
    return min + (max - min) * this.next();
  }
  array(len: number, min: number = -1, max: number = 1): Float64Array {
    const arr = new Float64Array(len);
    for (let i = 0; i < len; i++) {
      arr[i] = this.range(min, max);
    }
    return arr;
  }
}

describe("Property-Based Testing (dsp-stft algebraic laws)", () => {
  const rng = new SimpleRng(555666777n);

  it("Linearity of STFT: stft(alpha * s1 + beta * s2) == alpha * stft(s1) + beta * stft(s2)", () => {
    for (let iter = 0; iter < 10; iter++) {
      const n = 512;
      const nFft = 128;
      const hop = 64;
      const s1 = rng.array(n);
      const s2 = rng.array(n);
      const alpha = rng.range(-2.0, 2.0);
      const beta = rng.range(-2.0, 2.0);

      const comb = new Float64Array(n);
      for (let i = 0; i < n; i++) comb[i] = alpha * s1[i] + beta * s2[i];

      const specComb = stft(comb, nFft, hop, WindowType.HANN);
      const spec1 = stft(s1, nFft, hop, WindowType.HANN);
      const spec2 = stft(s2, nFft, hop, WindowType.HANN);

      assert.equal(specComb.length, spec1.length);
      for (let i = 0; i < specComb.length; i++) {
        const expected = alpha * spec1[i] + beta * spec2[i];
        const diff = Math.abs(specComb[i] - expected);
        assert.ok(
          diff < 1e-10,
          `linearity mismatch at ${i}: got ${specComb[i]} vs ${expected} (diff: ${diff})`
        );
      }
    }
  });

  it("COLA Roundtrip Reconstruction across random smooth signals", () => {
    for (let iter = 0; iter < 5; iter++) {
      const n = 1024;
      const nFft = 128;
      const hop = 32; // 75% overlap
      // Create smooth signal by lowpass accumulation
      const noise = rng.array(n);
      const signal = new Float64Array(n);
      let acc = 0;
      for (let i = 0; i < n; i++) {
        acc = 0.9 * acc + 0.1 * noise[i];
        signal[i] = acc;
      }

      const spec = stft(signal, nFft, hop, WindowType.HANN);
      const recon = istft(spec, nFft, hop, WindowType.HANN, n);

      // Verify interior samples
      const start = nFft;
      const end = n - nFft;
      for (let i = start; i < end; i++) {
        const diff = Math.abs(recon[i] - signal[i]);
        assert.ok(
          diff < 1e-4,
          `roundtrip mismatch at ${i}: expected ${signal[i]}, got ${recon[i]} (diff: ${diff})`
        );
      }
    }
  });
});
