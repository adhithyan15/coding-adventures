import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  InvalidParamError,
  parseWavelet,
  parseWaveletBoundary,
  Periodic,
  Reflect,
  Replicate,
  sampleWithBoundary,
  Symmetric,
  Wavelet,
  WaveletBoundary,
  WaveletFamily,
  Zero,
} from "../src/index.js";

describe("Wavelet definitions, families, and boundary modes", () => {
  it("verifies Haar wavelet properties", () => {
    const w = Wavelet.haar();
    assert.equal(w.name, "haar");
    assert.equal(w.family, WaveletFamily.HAAR);
    assert.equal(w.filterLength, 2);

    let sumSqLo = 0;
    for (let i = 0; i < w.decLo.length; i++) {
      sumSqLo += w.decLo[i] * w.decLo[i];
    }
    assert.ok(Math.abs(sumSqLo - 1.0) < 1e-6);

    let sumSqHi = 0;
    for (let i = 0; i < w.decHi.length; i++) {
      sumSqHi += w.decHi[i] * w.decHi[i];
    }
    assert.ok(Math.abs(sumSqHi - 1.0) < 1e-6);
  });

  it("verifies Daubechies wavelets", () => {
    const orders: Array<[number, number]> = [
      [1, 2],
      [2, 4],
      [4, 8],
      [6, 12],
      [8, 16],
    ];

    for (const [order, expectedLen] of orders) {
      const w = Wavelet.db(order);
      assert.equal(w.filterLength, expectedLen);

      let energyLo = 0;
      for (let i = 0; i < w.decLo.length; i++) {
        energyLo += w.decLo[i] * w.decLo[i];
      }
      assert.ok(Math.abs(energyLo - 1.0) < 1e-4);

      let energyHi = 0;
      for (let i = 0; i < w.decHi.length; i++) {
        energyHi += w.decHi[i] * w.decHi[i];
      }
      assert.ok(Math.abs(energyHi - 1.0) < 1e-4);

      let dot = 0;
      for (let i = 0; i < w.decLo.length; i++) {
        dot += w.decLo[i] * w.decHi[i];
      }
      assert.ok(Math.abs(dot) < 1e-6);
    }

    assert.throws(() => Wavelet.db(3), InvalidParamError);
  });

  it("verifies Symlets", () => {
    const w2 = Wavelet.sym(2);
    assert.equal(w2.filterLength, 4);

    const w4 = Wavelet.sym(4);
    assert.equal(w4.name, "sym4");
    assert.equal(w4.family, WaveletFamily.SYMLETS);
    assert.equal(w4.filterLength, 8);

    assert.throws(() => Wavelet.sym(5), InvalidParamError);
  });

  it("verifies Coiflets", () => {
    const w = Wavelet.coif(1);
    assert.equal(w.name, "coif1");
    assert.equal(w.family, WaveletFamily.COIFLETS);
    assert.equal(w.filterLength, 6);

    assert.throws(() => Wavelet.coif(2), InvalidParamError);
  });

  it("verifies Biorthogonal wavelets", () => {
    const w = Wavelet.bior(2, 2);
    assert.equal(w.name, "bior2.2");
    assert.equal(w.family, WaveletFamily.BIORTHOGONAL);
    assert.equal(w.decLo.length, 5);
    assert.equal(w.recLo.length, 3);

    const wAlias = Wavelet.bior(5, 3);
    assert.equal(wAlias.name, "bior2.2");

    assert.throws(() => Wavelet.bior(9, 7), InvalidParamError);
  });

  it("verifies continuous wavelets", () => {
    const wm = Wavelet.morlet();
    assert.equal(wm.family, WaveletFamily.MORLET);

    const wr = Wavelet.ricker();
    assert.equal(wr.family, WaveletFamily.RICKER);

    const wmh = Wavelet.mexicanHat();
    assert.equal(wmh.family, WaveletFamily.RICKER);
  });

  it("verifies parseWavelet", () => {
    assert.equal(parseWavelet("haar").family, WaveletFamily.HAAR);
    assert.equal(parseWavelet("HAAR").family, WaveletFamily.HAAR);
    assert.equal(parseWavelet("db4").filterLength, 8);
    assert.equal(parseWavelet("DB2").filterLength, 4);
    assert.equal(parseWavelet("sym4").name, "sym4");
    assert.equal(parseWavelet("coif1").name, "coif1");
    assert.equal(parseWavelet("bior2.2").name, "bior2.2");
    assert.equal(parseWavelet("bior5.3").name, "bior2.2");
    assert.equal(parseWavelet("morlet").name, "morlet");
    assert.equal(parseWavelet("ricker").name, "ricker");
    assert.equal(parseWavelet("mexican_hat").name, "ricker");

    const w = Wavelet.haar();
    assert.equal(parseWavelet(w), w);

    assert.throws(() => parseWavelet("unknown_wavelet"), InvalidParamError);
  });

  it("verifies boundary sampling modes", () => {
    const sig = [10.0, 20.0, 30.0];

    // Interior
    assert.equal(sampleWithBoundary(sig, 0, Zero), 10.0);
    assert.equal(sampleWithBoundary(sig, 1, Zero), 20.0);
    assert.equal(sampleWithBoundary(sig, 2, Zero), 30.0);

    // Zero
    assert.equal(sampleWithBoundary(sig, -1, Zero), 0.0);
    assert.equal(sampleWithBoundary(sig, 3, Zero), 0.0);

    // Replicate
    assert.equal(sampleWithBoundary(sig, -2, Replicate), 10.0);
    assert.equal(sampleWithBoundary(sig, 5, Replicate), 30.0);

    // Periodic
    assert.equal(sampleWithBoundary(sig, 3, Periodic), 10.0);
    assert.equal(sampleWithBoundary(sig, 4, Periodic), 20.0);
    assert.equal(sampleWithBoundary(sig, -1, Periodic), 30.0);

    // Reflect (period = 2*(n-1) = 4)
    // 0->10, 1->20, 2->30, 3->20, 4->10
    assert.equal(sampleWithBoundary(sig, 3, Reflect), 20.0);
    assert.equal(sampleWithBoundary(sig, 4, Reflect), 10.0);
    assert.equal(sampleWithBoundary(sig, -1, Reflect), 20.0);

    // Symmetric (period = 2*n = 6)
    // 0->10, 1->20, 2->30, 3->30, 4->20, 5->10
    assert.equal(sampleWithBoundary(sig, 3, Symmetric), 30.0);
    assert.equal(sampleWithBoundary(sig, 4, Symmetric), 20.0);
    assert.equal(sampleWithBoundary(sig, 5, Symmetric), 10.0);

    // Empty signal
    assert.equal(sampleWithBoundary([], 0, Zero), 0.0);

    // Boundary parsing
    assert.equal(parseWaveletBoundary("periodic"), WaveletBoundary.PERIODIC);
    assert.equal(parseWaveletBoundary(WaveletBoundary.ZERO), WaveletBoundary.ZERO);
    assert.throws(() => parseWaveletBoundary("invalid_mode"), InvalidParamError);
  });
});
