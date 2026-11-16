import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { ComplexArray } from "@coding-adventures/dsp-complex";
import {
  fft,
  ifft,
  irfft,
  rfft,
} from "../src/index.js";

describe("High-level FFT APIs with ComplexArray", () => {
  it("computes complex FFT and IFFT on ComplexArray", () => {
    const original = ComplexArray.fromRealImag([1, 2, 3, 4], [0, 1, 0, -1]);
    const spectrum = fft(original);
    assert.ok(spectrum instanceof ComplexArray);
    assert.equal(spectrum.length, 4);

    const recovered = ifft(spectrum);
    assert.ok(recovered.equals(original, 1e-11));
  });

  it("computes FFT on real array and interleaved array", () => {
    const realSig = [1, 2, 3, 4];
    const spec = fft(realSig);
    assert.equal(spec.length, 4);

    const interleaved = [1, 0, 2, 1, 3, 0, 4, -1];
    const specComplex = fft(interleaved, { complex: true });
    assert.equal(specComplex.length, 4);
  });

  it("computes high-level rfft and irfft", () => {
    const realSig = [1, 2, 3, 4, 5, 6];
    const half = rfft(realSig);
    assert.ok(half instanceof ComplexArray);
    assert.equal(half.length, 4); // floor(6/2) + 1 = 4

    const recovered = irfft(half, 6);
    assert.equal(recovered.length, 6);
    for (let i = 0; i < 6; i++) {
      assert.ok(Math.abs(recovered[i] - realSig[i]) < 1e-11);
    }
  });

  it("PBT: verifies round-trip error budget on random vectors", () => {
    // Deterministic pseudo-random generation (LCG)
    let state = 123456789;
    const nextFloat = () => {
      state = (1103515245 * state + 12345) & 0x7fffffff;
      return (state / 0x7fffffff) * 2.0 - 1.0;
    };

    const lengths = [4, 7, 8, 15, 16, 31, 32, 64, 100];
    for (const len of lengths) {
      const real = new Float64Array(len);
      const imag = new Float64Array(len);
      for (let i = 0; i < len; i++) {
        real[i] = nextFloat();
        imag[i] = nextFloat();
      }
      const original = ComplexArray.fromRealImag(real, imag);
      const spectrum = fft(original);
      const recovered = ifft(spectrum);

      let maxDiff = 0.0;
      for (let i = 0; i < len; i++) {
        const orig = original.get(i);
        const rec = recovered.get(i);
        maxDiff = Math.max(
          maxDiff,
          Math.abs(orig.re - rec.re),
          Math.abs(orig.im - rec.im)
        );
      }
      assert.ok(
        maxDiff < 1e-10,
        `PBT failed for length ${len}: max error ${maxDiff} exceeds 1e-10`
      );
    }
  });
});
