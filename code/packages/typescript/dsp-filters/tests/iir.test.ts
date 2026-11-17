import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  EmptyKernelError,
  EmptySignalError,
  filtfilt,
  iir,
  InvalidCoefficientError,
  lfilter,
} from "../src/index.js";

describe("IIR filtering (lfilter, iir, filtfilt)", () => {
  it("computes 1st order IIR impulse response via lfilter", () => {
    // y[n] = x[n] + 0.5 * y[n-1]
    const b = [1.0];
    const a = [1.0, -0.5];
    const x = [1.0, 0.0, 0.0, 0.0];
    const y = lfilter(b, a, x);

    assert.equal(y.length, 4);
    assert.ok(Math.abs(y[0] - 1.0) < 1e-12);
    assert.ok(Math.abs(y[1] - 0.5) < 1e-12);
    assert.ok(Math.abs(y[2] - 0.25) < 1e-12);
    assert.ok(Math.abs(y[3] - 0.125) < 1e-12);
  });

  it("normalises by a[0] when a[0] != 1.0", () => {
    const b = [2.0];
    const a = [2.0, -1.0]; // equivalent to b=[1.0], a=[1.0, -0.5]
    const x = [1.0, 0.0, 0.0, 0.0];
    const y = lfilter(b, a, x);

    assert.ok(Math.abs(y[0] - 1.0) < 1e-12);
    assert.ok(Math.abs(y[1] - 0.5) < 1e-12);
  });

  it("acts as FIR filter when a = [1.0]", () => {
    const b = [1.0, 2.0, 1.0];
    const a = [1.0];
    const x = [1.0, 1.0, 1.0, 1.0];
    const y = lfilter(b, a, x);

    assert.equal(y.length, 4);
    assert.ok(Math.abs(y[0] - 1.0) < 1e-12); // 1*1
    assert.ok(Math.abs(y[1] - 3.0) < 1e-12); // 1*1 + 2*1
    assert.ok(Math.abs(y[2] - 4.0) < 1e-12); // 1*1 + 2*1 + 1*1
    assert.ok(Math.abs(y[3] - 4.0) < 1e-12);
  });

  it("matches iir alias with lfilter", () => {
    const b = [0.5, 0.5];
    const a = [1.0, -0.2];
    const x = [1.0, 2.0, 3.0, 4.0];

    const yLfilter = lfilter(b, a, x);
    const yIir = iir(x, b, a);

    assert.equal(yLfilter.length, yIir.length);
    for (let i = 0; i < yLfilter.length; i++) {
      assert.equal(yLfilter[i], yIir[i]);
    }
  });

  it("performs zero-phase forward-backward filtfilt with padding", () => {
    // Symmetrical impulse in long signal to exercise padded reflection branch
    const n = 64;
    const x = new Float64Array(n);
    x[32] = 1.0;

    const b = [0.25, 0.5, 0.25];
    const a = [1.0];

    const y = filtfilt(b, a, x);
    assert.equal(y.length, n);

    // Peak should remain exactly at center index 32 (zero phase distortion)
    let maxIdx = 0;
    let maxVal = -Infinity;
    for (let i = 0; i < n; i++) {
      if (y[i] > maxVal) {
        maxVal = y[i];
        maxIdx = i;
      }
    }
    assert.equal(maxIdx, 32);
    // Symmetry check: y[32 - k] === y[32 + k]
    for (let k = 1; k < 10; k++) {
      assert.ok(Math.abs(y[32 - k] - y[32 + k]) < 1e-12);
    }
  });

  it("performs zero-phase filtfilt with short signal (unpadded branch)", () => {
    const x = [0.0, 1.0, 2.0, 1.0, 0.0];
    const b = [0.2, 0.6, 0.2];
    const a = [1.0];

    const y = filtfilt(b, a, x);
    assert.equal(y.length, x.length);
    assert.ok(Number.isFinite(y[0]));
  });

  it("rejects invalid inputs in lfilter and filtfilt", () => {
    assert.throws(() => lfilter([], [1], [1]), EmptyKernelError);
    assert.throws(() => lfilter([1], [], [1]), EmptyKernelError);
    assert.throws(() => lfilter([1], [1], []), EmptySignalError);
    assert.throws(() => lfilter([1], [0], [1]), InvalidCoefficientError);
    assert.throws(() => filtfilt([1], [1], []), EmptySignalError);
  });
});
