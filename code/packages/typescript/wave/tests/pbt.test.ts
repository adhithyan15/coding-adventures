import assert from "node:assert/strict";
import test from "node:test";
import { CoverageBitmap, fuzzTarget, SplitMix64 } from "@coding-adventures/test-bootstrap";
import { PI } from "trig/dist/trig.js";
import { Wave } from "../src/wave.js";

function unitFloat(rng: SplitMix64): number {
  return Number(rng.nextU64() >> 11n) * (1.0 / Number(1n << 53n));
}

test("PBT (SplitMix64): Wave evaluations stay finite and amplitude-bounded", () => {
  const rng = new SplitMix64(0x57a1_e001_cafe_beefn);
  for (let i = 0; i < 200; i += 1) {
    const amp = unitFloat(rng) * 1000.0;
    const freq = 0.01 + unitFloat(rng) * 20_000.0;
    const phase = (unitFloat(rng) - 0.5) * 4.0 * PI;
    const wave = new Wave(amp, freq, phase);

    for (let j = 0; j < 8; j += 1) {
      const t = (unitFloat(rng) - 0.5) * 100.0;
      const y = wave.evaluate(t);
      assert.equal(Number.isFinite(y), true);
      assert.ok(Math.abs(y) <= amp + 1e-12);
    }
  }
});

test("PBT (SplitMix64): periodicity and half-period inversion", () => {
  const rng = new SplitMix64(0x1234_5678_abcd_ef01n);
  for (let i = 0; i < 150; i += 1) {
    const amp = 0.1 + unitFloat(rng) * 100.0;
    const freq = 0.5 + unitFloat(rng) * 500.0;
    const phase = (unitFloat(rng) - 0.5) * 2.0 * PI;
    const wave = new Wave(amp, freq, phase);
    const period = wave.period();
    const t = (unitFloat(rng) - 0.5) * 5.0;

    const y0 = wave.evaluate(t);
    const yHalf = wave.evaluate(t + 0.5 * period);
    const yFull = wave.evaluate(t + period);
    const tol = 1e-9 * Math.max(1.0, amp);
    assert.ok(Math.abs(y0 - yFull) <= tol);
    assert.ok(Math.abs(y0 + yHalf) <= tol);
    assert.ok(Math.abs(wave.angularFrequency() * period - 2.0 * PI) <= 1e-12);
  }
});

test("Fuzz (fuzzTarget): raw 64-bit float triples fail closed or remain amplitude-bounded", () => {
  const encodeTriple = (a: number, f: number, p: number): Uint8Array => {
    const buf = new Uint8Array(24);
    const view = new DataView(buf.buffer);
    view.setFloat64(0, a, false);
    view.setFloat64(8, f, false);
    view.setFloat64(16, p, false);
    return buf;
  };
  const seeds = [
    encodeTriple(1.0, 440.0, 0.0),
    encodeTriple(0.0, 1.0, PI / 2),
    encodeTriple(Number.NaN, 1.0, 0.0),
    encodeTriple(1.0, 0.0, 0.0),
    encodeTriple(Number.MAX_VALUE, Number.MIN_VALUE, Number.MAX_VALUE),
  ];

  const report = fuzzTarget(
    seeds,
    { maxIterations: 256, seed: 0x0001_0002_0003_0004n, maxInputLen: 24 },
    (input: Uint8Array, cov: CoverageBitmap) => {
      if (input.length < 24) {
        cov.recordEdge(1);
        return { status: "pass" };
      }
      const view = new DataView(input.buffer, input.byteOffset, input.byteLength);
      const a = view.getFloat64(0, false);
      const f = view.getFloat64(8, false);
      const p = view.getFloat64(16, false);
      try {
        const w = new Wave(a, f, p);
        const y = w.evaluate(0.25);
        if (!Number.isFinite(y) || Math.abs(y) > w.amplitude) {
          return { status: "crash", kind: "invariant-violation", frames: ["Wave.evaluate"] };
        }
        cov.recordEdge(2);
      } catch (err) {
        if (!(err instanceof RangeError || err instanceof TypeError)) {
          return { status: "crash", kind: "unexpected-error", frames: ["Wave"] };
        }
        cov.recordEdge(3);
      }
      return { status: "pass" };
    }
  );
  assert.equal(report.clean, true);
});
