import assert from "node:assert/strict";
import test from "node:test";
import {
  checkProperty,
  CoverageBitmap,
  fuzzTarget,
  Gen,
  SplitMix64,
  type FuzzOutcome,
} from "@coding-adventures/test-bootstrap";

import {
  calcNyquist,
  MAX_SAMPLE_RATE_HZ,
  nyquistFrequency,
  OscillatorState,
  PulseOscillator,
  SampleBuffer,
  sampleCountForDuration,
  SawtoothOscillator,
  SineOscillator,
  SquareOscillator,
  timeAtSample,
  TriangleOscillator,
  type WaveformType,
} from "../src/index.js";

const EPSILON = 1e-7;

function nextSignedUnit(rng: SplitMix64): number {
  const raw53 = Number(rng.nextU64() & 0x1f_ffff_ffff_ffffn);
  return (raw53 / 9007199254740991.0) * 2.0 - 1.0;
}

function nextUnit(rng: SplitMix64): number {
  const raw53 = Number(rng.nextU64() & 0x1f_ffff_ffff_ffffn);
  return raw53 / 9007199254740991.0;
}

test("PBT: Nyquist frequency property: nyquist(rate) * 2 == rate", () => {
  const rateGen = Gen.integer(1, Math.trunc(MAX_SAMPLE_RATE_HZ)).map((v) => Number(v));
  assert.doesNotThrow(() => {
    const verdict = checkProperty(
      0x1337n,
      { numTests: 100 },
      (rng) => rateGen.sample(rng),
      (rate) => Math.abs(nyquistFrequency(rate) * 2.0 - rate) <= 1e-9,
    );
    assert.equal(verdict.status, "pass");
  });

  const prng = new SplitMix64(0x1337n);
  for (let trial = 0; trial < 100; trial += 1) {
    const rate = 1.0 + nextUnit(prng) * (MAX_SAMPLE_RATE_HZ - 1.0);
    const nyq = nyquistFrequency(rate);
    assert.ok(
      Math.abs(nyq * 2.0 - rate) <= 1e-9,
      `Nyquist relation failed for rate ${rate}`,
    );
  }
});

test("PBT: Waveform amplitude bounding invariant: values lie within [offset - amp, offset + amp]", () => {
  const prng = new SplitMix64(0xcafe_baben);
  const waveforms: WaveformType[] = ["sine", "square", "triangle", "sawtooth", "pulse", "noise"];

  for (const wave of waveforms) {
    for (let trial = 0; trial < 25; trial += 1) {
      const freq = 1.0 + nextUnit(prng) * 1000.0;
      const amp = 0.1 + nextUnit(prng) * 5.0;
      const offset = nextSignedUnit(prng) * 10.0;

      const osc = new OscillatorState({
        waveform: wave,
        frequencyHz: freq,
        sampleRateHz: 44_100.0,
        amplitude: amp,
        offset,
      });

      const samples = osc.renderSamples(64);
      const minExpected = offset - amp - EPSILON;
      const maxExpected = offset + amp + EPSILON;

      for (let i = 0; i < samples.length; i += 1) {
        assert.ok(
          samples[i] >= minExpected && samples[i] <= maxExpected,
          `${wave} sample ${i} (${samples[i]}) outside bounds [${minExpected}, ${maxExpected}]`,
        );
      }
    }
  }
});

test("PBT: Continuous signal waveform periodicity: x(t + 1/f) == x(t)", () => {
  const prng = new SplitMix64(0x9999n);

  for (let trial = 0; trial < 30; trial += 1) {
    const freq = 5.0 + nextUnit(prng) * 200.0;
    const period = 1.0 / freq;

    const sine = new SineOscillator(freq);
    const tri = new TriangleOscillator(freq);
    const saw = new SawtoothOscillator(freq);
    const sq = new SquareOscillator(freq);
    const pulse = new PulseOscillator(freq, 0.3);

    const t = (nextSignedUnit(prng) + 1.0) * 0.5;

    assert.ok(
      Math.abs(sine.valueAt(t + period) - sine.valueAt(t)) <= 1e-6,
      `Sine periodicity failed at f=${freq}, t=${t}`,
    );
    assert.ok(
      Math.abs(tri.valueAt(t + period) - tri.valueAt(t)) <= 1e-6,
      `Triangle periodicity failed at f=${freq}, t=${t}`,
    );
    assert.ok(
      Math.abs(saw.valueAt(t + period) - saw.valueAt(t)) <= 1e-6,
      `Sawtooth periodicity failed at f=${freq}, t=${t}`,
    );
    assert.equal(
      sq.valueAt(t + period),
      sq.valueAt(t),
      `Square periodicity failed at f=${freq}, t=${t}`,
    );
    assert.equal(
      pulse.valueAt(t + period),
      pulse.valueAt(t),
      `Pulse periodicity failed at f=${freq}, t=${t}`,
    );
  }
});

test("PBT: Monotonicity of sample count with respect to duration", () => {
  const sampleRate = 48_000;
  let previousCount = 0;
  for (let d = 0; d <= 10; d += 1) {
    const duration = d * 0.1;
    const count = sampleCountForDuration(duration, sampleRate);
    assert.ok(count >= previousCount, `Monotonicity failed at duration ${duration}`);
    previousCount = count;
  }
});

test("PBT: Deterministic PRNG seed replay produces identical buffer", () => {
  const prng = new SplitMix64(0x5555n);
  for (let trial = 0; trial < 10; trial += 1) {
    const seed = prng.nextU64();
    const oscA = new OscillatorState({ waveform: "noise", seed });
    const oscB = new OscillatorState({ waveform: "noise", seed });

    const bufA = oscA.renderSamples(128);
    const bufB = oscB.renderSamples(128);

    assert.deepEqual(Array.from(bufA), Array.from(bufB));
  }
});

test("PBT: CWE-20/190/400/1321 fuzzing of oscillator and SampleBuffer fails closed cleanly", () => {
  const report = fuzzTarget(
    [new Uint8Array([0, 1, 2, 3]), new Uint8Array([255, 128, 64, 0])],
    { maxIterations: 150, seed: 0x05c0_11a7n, maxInputLen: 32 },
    (input: Uint8Array, cov: CoverageBitmap): FuzzOutcome => {
      const b0 = input[0] ?? 0;
      const b1 = input[1] ?? 1;
      const waveforms = ["sine", "square", "triangle", "sawtooth", "pulse", "noise", "bad"];
      const wave = waveforms[b0 % waveforms.length];

      const numProbe =
        b1 === 255
          ? 1e100
          : b1 === 254
            ? Number.NaN
            : b1 === 253
              ? -1
              : b1;

      try {
        cov.recordEdge(1);
        const nyq = calcNyquist(numProbe);
        assert.ok(nyq === null || (Number.isSafeInteger(nyq) && nyq >= 0));

        const osc = new OscillatorState({
          waveform: wave as WaveformType,
          frequencyHz: numProbe,
          sampleRateHz: Math.max(1, b1 * 100),
          seed: BigInt(b0),
        });
        cov.recordEdge(2);
        const rendered = osc.renderSamples(Math.min(16, input.length));
        const buf = new SampleBuffer(rendered, 44_100);
        if (buf.sampleCount() > 0) {
          assert.ok(Number.isFinite(timeAtSample(0, 44_100)));
        }
        cov.recordEdge(3);
      } catch (err) {
        if (err instanceof TypeError || err instanceof RangeError) {
          cov.recordEdge(4);
          return { status: "pass" };
        }
        return {
          status: "crash",
          kind: err instanceof Error ? err.name : "UnknownError",
          frames: ["fuzzOscillator"],
        };
      }

      // Hostile Symbol / Object.create(null) / __proto__ checks must fail closed cleanly
      const hostileValues: unknown[] = [
        Symbol("hostile"),
        Object.create(null),
        JSON.parse('{"__proto__":{"polluted":true}}'),
      ];
      for (const hostile of hostileValues) {
        try {
          new SineOscillator(hostile as number);
          return { status: "crash", kind: "AcceptedHostile", frames: ["SineOscillator"] };
        } catch (err) {
          if (!(err instanceof TypeError || err instanceof RangeError)) {
            return { status: "crash", kind: "UnexpectedError", frames: ["SineOscillator"] };
          }
        }
      }
      assert.equal(({} as { polluted?: boolean }).polluted, undefined);
      cov.recordEdge(5);
      return { status: "pass" };
    },
  );

  assert.equal(report.clean, true);
  assert.equal(report.crashes.length, 0);
});
