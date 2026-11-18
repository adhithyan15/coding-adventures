import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { fir, firFft, fir_fft } from "../src/index.js";

describe("FIR filtering (direct and FFT convolution)", () => {
  it("convolves one-sample operands and recognizes the one-tap identity", () => {
    assert.deepEqual(Array.from(fir([2.0], [2.0])), [4.0]);
    assert.deepEqual(Array.from(fir([1.0, -2.0, 3.0], [1.0])), [1.0, -2.0, 3.0]);
  });

  it("convolves impulse with kernel: delta * h == h", () => {
    const delta = [1.0, 0.0, 0.0, 0.0];
    const kernel = [0.25, 0.5, 0.25];
    const out = fir(delta, kernel);
    // output length = 4 + 3 - 1 = 6
    assert.equal(out.length, 6);
    assert.equal(out[0], 0.25);
    assert.equal(out[1], 0.5);
    assert.equal(out[2], 0.25);
    assert.equal(out[3], 0.0);
  });

  it("produces identical results between fir and firFft", () => {
    const signal = [1.0, -2.0, 3.5, 0.5, -1.0, 2.0, 4.0, -3.0];
    const kernel = [0.1, 0.2, 0.4, 0.2, 0.1];

    const outDirect = fir(signal, kernel);
    const outFft = firFft(signal, kernel);
    const outFftAlias = fir_fft(signal, kernel);

    assert.equal(outDirect.length, outFft.length);
    assert.equal(outFft.length, outFftAlias.length);

    for (let i = 0; i < outDirect.length; i++) {
      assert.ok(
        Math.abs(outDirect[i] - outFft[i]) < 1e-11,
        `mismatch at ${i}: direct ${outDirect[i]} vs fft ${outFft[i]}`
      );
    }
  });

  it("handles long signals with firFft", () => {
    const n = 128;
    const signal = new Float64Array(n);
    for (let i = 0; i < n; i++) signal[i] = Math.sin((2 * Math.PI * i) / 16);
    const kernel = [0.2, 0.6, 0.2];

    const outDirect = fir(signal, kernel);
    const outFft = firFft(signal, kernel);

    for (let i = 0; i < outDirect.length; i++) {
      assert.ok(Math.abs(outDirect[i] - outFft[i]) < 1e-10);
    }
  });
});
