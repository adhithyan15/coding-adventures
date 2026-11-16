import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { cwt, EmptySignalError, InvalidParamError } from "../src/index.js";

describe("Continuous Wavelet Transform (cwt)", () => {
  it("cwt shape and values", () => {
    const sig = new Float64Array(32).fill(1.0);
    const scales = [1.0, 2.0, 4.0];
    const out = cwt(sig, scales, "morlet");
    assert.equal(out.length, scales.length * sig.length);
    for (let i = 0; i < out.length; i++) {
      assert.ok(Number.isFinite(out[i]));
      assert.ok(out[i] >= 0.0);
    }
  });

  it("cwt Ricker impulse response", () => {
    const n = 31;
    const sig = new Float64Array(n);
    const mid = Math.floor(n / 2);
    sig[mid] = 10.0;
    const scales = [2.0];
    const out = cwt(sig, scales, "ricker");
    assert.equal(out.length, n);

    const centerVal = out[mid];
    assert.ok(centerVal > 0.0);
    assert.ok(centerVal > out[0]);
    assert.ok(centerVal > out[n - 1]);
  });

  it("cwt Morlet sine response", () => {
    const omega = 1.0;
    const n = 64;
    const sig = new Float64Array(n);
    for (let t = 0; t < n; t++) {
      sig[t] = Math.sin(omega * t);
    }
    const scales = [2.0, 6.0, 15.0];
    const out = cwt(sig, scales, "morlet");
    assert.equal(out.length, 3 * n);

    const mid = 32;
    const valS2 = out[0 * n + mid];
    const valS6 = out[1 * n + mid];
    const valS15 = out[2 * n + mid];

    assert.ok(valS6 > valS15);
    assert.ok(valS6 > valS2);
  });

  it("cwt invalid inputs", () => {
    assert.throws(() => cwt([], [1.0]), EmptySignalError);
    assert.throws(() => cwt([1.0, 2.0], []), InvalidParamError);
    assert.throws(() => cwt([1.0, 2.0], [0.0]), InvalidParamError);
    assert.throws(() => cwt([1.0, 2.0], [-1.0]), InvalidParamError);
    assert.throws(() => cwt([1.0, 2.0], [Infinity]), InvalidParamError);
    assert.throws(() => cwt([1.0, 2.0], [1.0], "haar"), InvalidParamError);
  });
});
