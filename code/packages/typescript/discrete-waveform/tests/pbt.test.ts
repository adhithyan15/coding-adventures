import assert from "node:assert/strict";
import test from "node:test";
import { ConstantWaveform, SineWaveform } from "@coding-adventures/analog-waveform";
import { CoverageBitmap, fuzzTarget, SplitMix64 } from "@coding-adventures/test-bootstrap";
import { MAX_SAMPLES, SampledWaveform } from "../src/index.js";

function unitFloat(rng: SplitMix64): number {
  return Number(rng.nextU64() >> 11n) * (1.0 / Number(1n << 53n));
}

test("PBT (SplitMix64): fromAnalog and zero-order hold invariants", () => {
  const rng = new SplitMix64(0x4321_8765_abcd_ef01n);
  for (let trial = 0; trial < 100; trial += 1) {
    const count = 1 + rng.bounded(64);
    const rate = 1 + unitFloat(rng) * 1000;
    const amp = unitFloat(rng) * 20;
    const freq = 0.5 + unitFloat(rng) * 50;
    const analog = new SineWaveform(amp, freq);

    const sw = SampledWaveform.fromAnalog(analog, rate, count);
    assert.equal(sw.samples.length, count);
    assert.ok(Math.abs(sw.samplePeriodSeconds() * sw.sampleRateHz - 1) <= 1e-12);
    assert.ok(Math.abs(sw.durationSeconds() - count / rate) <= 1e-12);

    const idx = rng.bounded(count);
    assert.equal(sw.sampleAtIndex(idx), sw.samples[idx]);
    assert.equal(sw.heldValueAt(-1), sw.samples[0]);
    assert.equal(sw.heldValueAt(sw.durationSeconds() + 1), sw.samples[count - 1]);

    const c = SampledWaveform.fromAnalog(new ConstantWaveform(7), rate, count);
    const t = unitFloat(rng) * (count / rate);
    assert.equal(c.heldValueAt(t), 7);
  }
});

test("Fuzz (fuzzTarget): hostile counts, rates, and times fail closed or return finite samples", () => {
  const encodeCase = (count: number, rate: number, t: number): Uint8Array => {
    const buf = new Uint8Array(24);
    const view = new DataView(buf.buffer);
    view.setInt32(0, count, false);
    view.setFloat64(8, rate, false);
    view.setFloat64(16, t, false);
    return buf;
  };
  const seeds = [
    encodeCase(4, 10, 0.15),
    encodeCase(0, 10, 0),
    encodeCase(MAX_SAMPLES + 1, 10, 0),
    encodeCase(4, Number.NaN, 0),
    encodeCase(4, 10, Number.NaN),
  ];

  const report = fuzzTarget(
    seeds,
    { maxIterations: 256, seed: 0x1111_2222_3333_4444n, maxInputLen: 24 },
    (input: Uint8Array, cov: CoverageBitmap) => {
      if (input.length < 24) {
        cov.recordEdge(1);
        return { status: "pass" };
      }
      const view = new DataView(input.buffer, input.byteOffset, input.byteLength);
      const rawCount = view.getInt32(0, false);
      const count = Math.abs(rawCount) % 16 === rawCount ? rawCount : (rawCount % 64);
      const rate = view.getFloat64(8, false);
      const t = view.getFloat64(16, false);
      try {
        const sw = SampledWaveform.fromAnalog(new SineWaveform(1, 2), rate, count);
        const held = sw.heldValueAt(t);
        if (!Number.isFinite(held)) {
          return { status: "crash", kind: "non-finite-held", frames: ["heldValueAt"] };
        }
        cov.recordEdge(2);
      } catch (err) {
        if (!(err instanceof RangeError || err instanceof TypeError)) {
          return { status: "crash", kind: "unexpected-error", frames: ["SampledWaveform"] };
        }
        cov.recordEdge(3);
      }
      return { status: "pass" };
    }
  );
  assert.equal(report.clean, true);
});
