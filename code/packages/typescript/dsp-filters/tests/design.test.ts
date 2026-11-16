import assert from "node:assert/strict";
import { describe, it } from "node:test";
import {
  butterworthBandpass,
  butterworthHighpass,
  butterworthLowpass,
  chebyshev1Lowpass,
  designBandPass,
  designHighPass,
  designLowPass,
  firwin,
  InvalidCoefficientError,
} from "../src/index.js";

describe("Classical filter design (FIR and IIR)", () => {
  it("designs lowpass FIR filter with unit DC gain and symmetry", () => {
    const numTaps = 31;
    const h = designLowPass(0.2, numTaps, "hamming");
    assert.equal(h.length, numTaps);

    // Unit DC gain: sum of coefficients == 1.0
    let sum = 0.0;
    for (let i = 0; i < numTaps; i++) sum += h[i];
    assert.ok(Math.abs(sum - 1.0) < 1e-12);

    // Linear-phase symmetry: h[i] == h[numTaps - 1 - i]
    for (let i = 0; i < numTaps; i++) {
      assert.ok(Math.abs(h[i] - h[numTaps - 1 - i]) < 1e-12);
    }
  });

  it("designs lowpass FIR with various window types", () => {
    for (const win of ["hann", "blackman", "rectangular", "kaiser"]) {
      const h = designLowPass(0.15, 17, win);
      assert.equal(h.length, 17);
      let sum = 0.0;
      for (let i = 0; i < h.length; i++) sum += h[i];
      assert.ok(Math.abs(sum - 1.0) < 1e-12);
    }
  });

  it("designs highpass FIR with zero DC gain", () => {
    const numTaps = 31;
    const h = designHighPass(0.2, numTaps, "hamming");
    assert.equal(h.length, numTaps);

    let sum = 0.0;
    for (let i = 0; i < numTaps; i++) sum += h[i];
    assert.ok(Math.abs(sum) < 1e-12);
  });

  it("designs bandpass FIR with peak gain at center frequency", () => {
    const numTaps = 33;
    const f1 = 0.15;
    const f2 = 0.35;
    const h = designBandPass(f1, f2, numTaps);
    assert.equal(h.length, numTaps);

    // Frequency response at center freq f0 = 0.25
    const f0 = (f1 + f2) * 0.5;
    const centre = (numTaps - 1) * 0.5;
    let gainAtF0 = 0.0;
    for (let k = 0; k < numTaps; k++) {
      gainAtF0 += h[k] * Math.cos(2.0 * Math.PI * f0 * (k - centre));
    }
    assert.ok(Math.abs(Math.abs(gainAtF0) - 1.0) < 1e-12);

    // DC gain should be nearly zero
    let dcGain = 0.0;
    for (let k = 0; k < numTaps; k++) dcGain += h[k];
    assert.ok(Math.abs(dcGain) < 0.05);
  });

  it("designs FIR filters via firwin interface", () => {
    const lp = firwin(17, 0.2);
    assert.equal(lp.length, 17);

    const hp = firwin(17, 0.2, { passZero: false });
    assert.equal(hp.length, 17);

    const bp = firwin(17, [0.1, 0.3], { passZero: false });
    assert.equal(bp.length, 17);

    const bs = firwin(17, [0.1, 0.3], { passZero: true });
    assert.equal(bs.length, 17);

    assert.throws(() => firwin(17, "invalid" as unknown as number), InvalidCoefficientError);
  });

  it("designs Butterworth lowpass filters (order 1 and 2)", () => {
    const b1 = butterworthLowpass(1, 0.2);
    assert.equal(b1.b.length, 2);
    assert.equal(b1.a.length, 2);
    // DC gain: (b0 + b1) / (a0 + a1) ≈ 1.0
    const dcGain1 = (b1.b[0] + b1.b[1]) / (b1.a[0] + b1.a[1]);
    assert.ok(Math.abs(dcGain1 - 1.0) < 1e-12);

    const b2 = butterworthLowpass(2, 0.2);
    assert.equal(b2.b.length, 3);
    assert.equal(b2.a.length, 3);
    const dcGain2 = (b2.b[0] + b2.b[1] + b2.b[2]) / (b2.a[0] + b2.a[1] + b2.a[2]);
    assert.ok(Math.abs(dcGain2 - 1.0) < 1e-12);

    assert.throws(() => butterworthLowpass(3, 0.2), InvalidCoefficientError);
    assert.throws(() => butterworthLowpass(1, 0.6), InvalidCoefficientError);
  });

  it("designs Butterworth highpass filters (order 1 and 2)", () => {
    const b1 = butterworthHighpass(1, 0.2);
    assert.equal(b1.b.length, 2);
    assert.equal(b1.a.length, 2);
    const dcGain1 = (b1.b[0] + b1.b[1]) / (b1.a[0] + b1.a[1]);
    assert.ok(Math.abs(dcGain1) < 1e-12);

    const b2 = butterworthHighpass(2, 0.2);
    assert.equal(b2.b.length, 3);
    assert.equal(b2.a.length, 3);
    const dcGain2 = (b2.b[0] + b2.b[1] + b2.b[2]) / (b2.a[0] + b2.a[1] + b2.a[2]);
    assert.ok(Math.abs(dcGain2) < 1e-12);

    assert.throws(() => butterworthHighpass(3, 0.2), InvalidCoefficientError);
    assert.throws(() => butterworthHighpass(1, -0.1), InvalidCoefficientError);
  });

  it("designs Butterworth bandpass filters (order 1 and 2)", () => {
    const bp1 = butterworthBandpass(1, 0.1, 0.3);
    assert.equal(bp1.b.length, 3);
    assert.equal(bp1.a.length, 3);

    const bp2 = butterworthBandpass(2, 0.1, 0.3);
    assert.equal(bp2.b.length, 5);
    assert.equal(bp2.a.length, 5);

    assert.throws(() => butterworthBandpass(3, 0.1, 0.3), InvalidCoefficientError);
    assert.throws(() => butterworthBandpass(1, 0.4, 0.2), InvalidCoefficientError);
    assert.throws(() => butterworthBandpass(1, NaN, 0.3), InvalidCoefficientError);
  });

  it("designs Chebyshev Type I lowpass filters (order 1 and 2)", () => {
    const c1 = chebyshev1Lowpass(1, 1.0, 0.2);
    assert.equal(c1.b.length, 2);
    assert.equal(c1.a.length, 2);
    const dcGain1 = (c1.b[0] + c1.b[1]) / (c1.a[0] + c1.a[1]);
    assert.ok(Math.abs(dcGain1 - 1.0) < 1e-12);

    const c2 = chebyshev1Lowpass(2, 1.0, 0.2);
    assert.equal(c2.b.length, 3);
    assert.equal(c2.a.length, 3);
    const dcGain2 = (c2.b[0] + c2.b[1] + c2.b[2]) / (c2.a[0] + c2.a[1] + c2.a[2]);
    assert.ok(Math.abs(dcGain2 - 1.0) < 0.15);

    assert.throws(() => chebyshev1Lowpass(3, 1.0, 0.2), InvalidCoefficientError);
    assert.throws(() => chebyshev1Lowpass(1, -1.0, 0.2), InvalidCoefficientError);
    assert.throws(() => chebyshev1Lowpass(1, 1.0, 0.6), InvalidCoefficientError);
  });

  it("rejects invalid FIR design parameters", () => {
    assert.throws(() => designLowPass(0.2, 10), InvalidCoefficientError); // even numTaps
    assert.throws(() => designLowPass(0.6, 11), InvalidCoefficientError); // cutoff > 0.5
    assert.throws(() => designBandPass(0.3, 0.2, 11), InvalidCoefficientError); // low >= high
    assert.throws(() => designBandPass(0.1, 0.6, 11), InvalidCoefficientError); // high > 0.5
  });
});
