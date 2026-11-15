import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  dwt,
  dwt1d,
  forwardLevelLengths,
  idwt,
  idwt1d,
  InvalidParamError,
  Periodic,
  Reflect,
  Wavelet,
  Zero,
} from "../src/index.js";

describe("1-D Discrete Wavelet Transform (dwt, idwt, dwt1d, idwt1d)", () => {
  it("forwardLevelLengths returns expected dimensions", () => {
    const lengths = forwardLevelLengths(16, 3);
    assert.deepEqual(Array.from(lengths), [16, 8, 4, 2]);

    const oddLens = forwardLevelLengths(15, 3);
    assert.deepEqual(Array.from(oddLens), [15, 8, 4, 2]);
  });

  it("dwt and idwt Haar roundtrip", () => {
    const sig = [1.0, 2.0, 3.0, 4.0, 5.0, 6.0, 7.0, 8.0];
    const [ca, cd] = dwt(sig, "haar", Periodic);
    assert.equal(ca.length, 4);
    assert.equal(cd.length, 4);

    const rec = idwt(ca, cd, "haar", Periodic, sig.length);
    assert.equal(rec.length, sig.length);
    for (let i = 0; i < sig.length; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-5);
    }
  });

  it("dwt and idwt db4 roundtrip", () => {
    const sig = new Float64Array(32);
    for (let i = 0; i < 32; i++) {
      sig[i] = Math.sin((2.0 * Math.PI * i) / 32.0);
    }
    const [ca, cd] = dwt(sig, "db4", Periodic);
    const rec = idwt(ca, cd, "db4", Periodic, 32);
    for (let i = 0; i < 32; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-4);
    }
  });

  it("dwt odd length", () => {
    const sig = [1.0, 3.0, -2.0, 5.0, 4.0];
    const [ca, cd] = dwt(sig, "haar", Reflect);
    assert.equal(ca.length, 3);
    assert.equal(cd.length, 3);

    const rec = idwt(ca, cd, "haar", Reflect, 5);
    assert.equal(rec.length, 5);
    for (let i = 0; i < 5; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-5);
    }
  });

  it("dwt1d multilevel", () => {
    const sig = new Float64Array(16);
    for (let i = 0; i < 16; i++) {
      sig[i] = i;
    }
    const coeffs = dwt1d(sig, "haar", 2, Periodic);
    assert.equal(coeffs.length, 16);

    const rec = idwt1d(coeffs, "haar", 2, Periodic, 16);
    assert.equal(rec.length, 16);
    for (let i = 0; i < 16; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-5);
    }
  });

  it("dwt1d sym4", () => {
    const sig = new Float64Array(32);
    for (let i = 0; i < 32; i++) {
      sig[i] = Math.cos(i * 0.3);
    }
    const coeffs = dwt1d(sig, "sym4", 2, Periodic);
    const rec = idwt1d(coeffs, "sym4", 2, Periodic, 32);
    assert.equal(rec.length, 32);
    for (let i = 0; i < 32; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-4);
    }
  });

  it("dwt rejects biorthogonal wavelets", () => {
    const w = Wavelet.bior(2, 2);
    const sig = [1.0, 4.0, 2.0, 8.0, 5.0, 7.0, 3.0, 6.0];
    assert.throws(() => dwt(sig, w, Zero), InvalidParamError);
    assert.throws(() => idwt([1.0, 2.0], [3.0, 4.0], w, Zero), InvalidParamError);
  });
});
