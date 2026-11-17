import assert from "node:assert/strict";
import test from "node:test";

import {
  MAX_FREQ_HZ,
  MAX_SAMPLE_RATE_HZ,
  nyquistFrequency,
  OscillatorState,
  PulseOscillator,
  sampleCountForDuration,
  SawtoothOscillator,
  SineOscillator,
  SplitMix64,
  SquareOscillator,
  TriangleOscillator,
  type WaveformType,
} from "../src/index.js";

const EPSILON = 1e-7;

test("PBT: Nyquist frequency property: nyquist(rate) * 2 == rate", () => {
  const prng = new SplitMix64(0x1337n);
  for (let trial = 0; trial < 100; trial += 1) {
    // Generate valid rate in [1.0, MAX_SAMPLE_RATE_HZ]
    const u = (prng.nextFloat() + 1.0) / 2.0; // [0.0, 1.0]
    const rate = 1.0 + u * (MAX_SAMPLE_RATE_HZ - 1.0);
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
      const u1 = (prng.nextFloat() + 1.0) / 2.0;
      const u2 = (prng.nextFloat() + 1.0) / 2.0;
      const u3 = prng.nextFloat();

      const freq = 1.0 + u1 * 1000.0;
      const amp = 0.1 + u2 * 5.0;
      const offset = u3 * 10.0;

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
    const u = (prng.nextFloat() + 1.0) / 2.0;
    const freq = 5.0 + u * 200.0;
    const period = 1.0 / freq;

    const sine = new SineOscillator(freq);
    const tri = new TriangleOscillator(freq);
    const saw = new SawtoothOscillator(freq);
    const sq = new SquareOscillator(freq);
    const pulse = new PulseOscillator(freq, 0.3);

    // Random time point
    const t = (prng.nextFloat() + 1.0) * 0.5;

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
