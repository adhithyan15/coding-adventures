import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  InvalidCoefficientsError,
  iswt,
  Periodic,
  swt,
} from "../src/index.js";

describe("Stationary Wavelet Transform (swt, iswt)", () => {
  it("swt shape preserves signal length", () => {
    const sig = new Float64Array(16);
    for (let i = 0; i < 16; i++) {
      sig[i] = i;
    }
    const coeffs = swt(sig, "haar", 2, Periodic);
    assert.equal(coeffs.length, 2);
    for (const [ca, cd] of coeffs) {
      assert.equal(ca.length, 16);
      assert.equal(cd.length, 16);
    }
  });

  it("swt and iswt Haar roundtrip", () => {
    const sig = [1.0, 2.0, 4.0, 7.0, 11.0, 16.0, 22.0, 29.0];
    const coeffs = swt(sig, "haar", 1, Periodic);
    const rec = iswt(coeffs, "haar", Periodic);
    assert.equal(rec.length, sig.length);
    for (let i = 0; i < sig.length; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-5);
    }
  });

  it("swt and iswt multilevel db2", () => {
    const sig = new Float64Array(16);
    for (let i = 0; i < 16; i++) {
      sig[i] = Math.cos(i * 0.5);
    }
    const coeffs = swt(sig, "db2", 2, Periodic);
    const rec = iswt(coeffs, "db2", Periodic);
    assert.equal(rec.length, sig.length);
    for (let i = 0; i < sig.length; i++) {
      assert.ok(Math.abs(sig[i] - rec[i]) < 1e-4);
    }
  });

  it("iswt throws on mismatched subband lengths", () => {
    assert.throws(
      () => iswt([[[1.0, 2.0], [1.0]]], "haar"),
      InvalidCoefficientsError
    );
  });
});
