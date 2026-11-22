import assert from "node:assert/strict";
import test from "node:test";
import { CoverageBitmap, fuzzTarget, SplitMix64 } from "@coding-adventures/test-bootstrap";
import { ConstantWaveform, SineWaveform } from "../src/index.js";

function unitFloat(rng: SplitMix64): number {
  return Number(rng.nextU64() >> 11n) * (1.0 / Number(1n << 53n));
}

test("PBT (SplitMix64): ConstantWaveform is invariant across time", () => {
  const rng = new SplitMix64(0x2026_0001_aaaa_bbbbn);
  for (let i = 0; i < 100; i += 1) {
    const level = (unitFloat(rng) - 0.5) * 1000;
    const c = new ConstantWaveform(level);
    const t = (unitFloat(rng) - 0.5) * 1e9;
    assert.equal(c.sampleAt(t), c.level);
  }
});

test("PBT (SplitMix64): SineWaveform stays within [offset - amp, offset + amp] and is periodic", () => {
  const rng = new SplitMix64(0x9876_5432_10fe_dcban);
  for (let i = 0; i < 150; i += 1) {
    const amp = unitFloat(rng) * 100;
    const freq = 0.25 + unitFloat(rng) * 500;
    const phase = (unitFloat(rng) - 0.5) * 4 * Math.PI;
    const offset = (unitFloat(rng) - 0.5) * 50;
    const sine = new SineWaveform(amp, freq, phase, offset);
    const period = sine.periodSeconds();
    const t = (unitFloat(rng) - 0.5) * 10;

    const y0 = sine.sampleAt(t);
    const y1 = sine.sampleAt(t + period);
    assert.ok(y0 >= offset - amp - 1e-12 && y0 <= offset + amp + 1e-12);
    assert.ok(Math.abs(y0 - y1) <= 1e-8 * Math.max(1, amp, Math.abs(offset)));
  }
});

test("Fuzz (fuzzTarget): raw binary64 parameters fail closed or evaluate finitely", () => {
  const encodeQuad = (a: number, f: number, p: number, o: number): Uint8Array => {
    const buf = new Uint8Array(32);
    const view = new DataView(buf.buffer);
    view.setFloat64(0, a, false);
    view.setFloat64(8, f, false);
    view.setFloat64(16, p, false);
    view.setFloat64(24, o, false);
    return buf;
  };
  const seeds = [
    encodeQuad(1, 440, 0, 0),
    encodeQuad(0, 1, 0, 5),
    encodeQuad(-1, 1, 0, 0),
    encodeQuad(Number.NaN, 1, 0, 0),
  ];

  const report = fuzzTarget(
    seeds,
    { maxIterations: 256, seed: 0xfeed_face_0000_0001n, maxInputLen: 32 },
    (input: Uint8Array, cov: CoverageBitmap) => {
      if (input.length < 32) {
        cov.recordEdge(1);
        return { status: "pass" };
      }
      const view = new DataView(input.buffer, input.byteOffset, input.byteLength);
      const a = view.getFloat64(0, false);
      const f = view.getFloat64(8, false);
      const p = view.getFloat64(16, false);
      const o = view.getFloat64(24, false);
      try {
        const s = new SineWaveform(a, f, p, o);
        const y = s.sampleAt(0.25);
        if (!Number.isFinite(y)) {
          return { status: "crash", kind: "non-finite-sample", frames: ["SineWaveform.sampleAt"] };
        }
        cov.recordEdge(2);
      } catch (err) {
        if (!(err instanceof RangeError || err instanceof TypeError)) {
          return { status: "crash", kind: "unexpected-error", frames: ["SineWaveform"] };
        }
        cov.recordEdge(3);
      }
      return { status: "pass" };
    }
  );
  assert.equal(report.clean, true);
});
