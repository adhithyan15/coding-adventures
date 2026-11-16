import assert from "node:assert/strict";
import { describe, it } from "node:test";
import { Biquad, sosfilt } from "../src/index.js";

describe("Biquad filters and SOS cascade", () => {
  it("processes DC through lowpass biquad with unit gain", () => {
    const bq = Biquad.lowpass(0.1, 1.0 / Math.SQRT2);
    // DC gain: (b0 + b1 + b2) / (1 + a1 + a2) ≈ 1.0
    const dcGain = (bq.b0 + b1(bq) + bq.b2) / (1.0 + bq.a1 + bq.a2);
    assert.ok(Math.abs(dcGain - 1.0) < 1e-12);

    const input = [1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0, 1.0];
    const out = bq.process(input);
    assert.equal(out.length, 8);
    // Output should settle towards 1.0
    assert.ok(Math.abs(out[7] - 1.0) < 0.1);
  });

  it("processes highpass biquad with zero DC gain", () => {
    const bq = Biquad.highpass(0.1);
    const dcGain = (bq.b0 + b1(bq) + bq.b2) / (1.0 + bq.a1 + bq.a2);
    assert.ok(Math.abs(dcGain) < 1e-12);
  });

  it("processes bandpass biquad with zero DC gain", () => {
    const bq = Biquad.bandpass(0.25, 2.0);
    const dcGain = (bq.b0 + b1(bq) + bq.b2) / (1.0 + bq.a1 + bq.a2);
    assert.ok(Math.abs(dcGain) < 1e-12);
  });

  it("processes notch, allpass, and peak biquads", () => {
    const notch = Biquad.notch(0.25, 2.0);
    assert.ok(notch instanceof Biquad);

    const allpass = Biquad.allpass(0.2, 1.0);
    assert.ok(allpass instanceof Biquad);

    const peak = Biquad.peak(0.2, 6.0, 1.5);
    assert.ok(peak instanceof Biquad);
  });

  it("resets internal state", () => {
    const bq = Biquad.lowpass(0.1);
    bq.processSample(5.0);
    bq.reset();
    const freshOut = bq.processSample(1.0);
    const brandNew = Biquad.lowpass(0.1);
    assert.equal(freshOut, brandNew.processSample(1.0));
  });

  it("cascades multiple sections via sosfilt", () => {
    // 2 cascaded identical biquads
    const sec1 = [0.1, 0.2, 0.1, 1.0, -0.5, 0.2];
    const sec2 = [0.1, 0.2, 0.1, 1.0, -0.5, 0.2];
    const signal = [1, 0, 0, 0, 0];
    const out = sosfilt([sec1, sec2], signal);
    assert.equal(out.length, 5);
    assert.ok(Math.abs(out[0] - 0.01) < 1e-12);
  });
});

function b1(bq: Biquad): number {
  return bq.b1;
}
