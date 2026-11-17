import assert from "node:assert/strict";
import test from "node:test";

import {
  calcNyquist,
  evalSquareDuty,
  fractionalPart,
  INTEGER_TOLERANCE,
  LcgPrng,
  MAX_DURATION_SECONDS,
  MAX_FREQ_HZ,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  nyquistFrequency,
  OscillatorState,
  PulseOscillator,
  renderSamples,
  sampleCountForDuration,
  SampleBuffer,
  SawtoothOscillator,
  SineOscillator,
  SplitMix64,
  SquareOscillator,
  synthesize,
  timeAtSample,
  TriangleOscillator,
  TWO_PI,
  UniformSampler,
  Waveform,
} from "../src/index.js";

const EPSILON = 1e-9;

function assertClose(actual: number, expected: number, eps = EPSILON, msg?: string): void {
  assert.ok(
    Math.abs(actual - expected) <= eps,
    `${msg ?? "assertClose"}: expected ${expected}, got ${actual} (diff ${Math.abs(actual - expected)})`,
  );
}

test("constants match OSC00 specifications", () => {
  assert.equal(MAX_SAMPLE_RATE_HZ, 192_000.0);
  assert.equal(MAX_FREQ_HZ, 96_000.0);
  assert.equal(MAX_DURATION_SECONDS, 60.0);
  assert.equal(MAX_SAMPLES, 16_777_216);
  assert.equal(INTEGER_TOLERANCE, 1e-9);
  assertClose(TWO_PI, 2.0 * Math.PI);
});

test("fractionalPart handles positive, zero, and negative values", () => {
  assertClose(fractionalPart(0.0), 0.0);
  assertClose(fractionalPart(0.25), 0.25);
  assertClose(fractionalPart(1.75), 0.75);
  assertClose(fractionalPart(-0.25), 0.75);
  assertClose(fractionalPart(-1.0), 0.0);
});

test("OSC00 §18.1 SineOscillator cross-language parity vectors", () => {
  // Vector 1: SineOscillator(frequency_hz = 1.0)
  const osc1 = new SineOscillator(1.0);
  assertClose(osc1.valueAt(0.0), 0.0);
  assertClose(osc1.valueAt(0.25), 1.0);
  assertClose(osc1.valueAt(0.5), 0.0);
  assertClose(osc1.valueAt(0.75), -1.0);
  assertClose(osc1.valueAt(1.0), 0.0);

  // Vector 2: SineOscillator(frequency_hz = 1.0, amplitude = 2.0, offset = 3.0)
  const osc2 = new SineOscillator(1.0, 2.0, 0.0, 3.0);
  assertClose(osc2.valueAt(0.0), 3.0);
  assertClose(osc2.valueAt(0.25), 5.0);
  assertClose(osc2.valueAt(0.75), 1.0);

  // Vector 3: SineOscillator(frequency_hz = 1.0, phase_cycles = 0.25)
  const osc3 = new SineOscillator(1.0, 1.0, 0.25, 0.0);
  assertClose(osc3.valueAt(0.0), 1.0);
  assertClose(osc3.valueAt(0.25), 0.0);
});

test("OSC00 §18.2 SquareOscillator cross-language parity vectors", () => {
  const osc = new SquareOscillator(2.0, 0.0, 1.0, 0.5, 0.0);
  assert.equal(osc.valueAt(0.0), 1.0);
  assert.equal(osc.valueAt(0.125), 1.0);
  assert.equal(osc.valueAt(0.25), 0.0);
  assert.equal(osc.valueAt(0.375), 0.0);
  assert.equal(osc.valueAt(0.5), 1.0);
});

test("OSC00 §18.3 UniformSampler cross-language parity vectors", () => {
  const signal = new SineOscillator(1.0);
  const sampler = new UniformSampler(4.0);
  const buffer = sampler.sample(signal, 1.0);

  assert.equal(buffer.sampleCount(), 4);
  assert.equal(buffer.samplePeriodSeconds(), 0.25);
  assert.equal(buffer.durationSeconds(), 1.0);
  assert.equal(sampler.nyquistFrequency(), 2.0);

  assertClose(buffer.timeAt(0), 0.0);
  assertClose(buffer.timeAt(1), 0.25);
  assertClose(buffer.timeAt(2), 0.5);
  assertClose(buffer.timeAt(3), 0.75);

  assertClose(buffer.samples[0], 0.0);
  assertClose(buffer.samples[1], 1.0);
  assertClose(buffer.samples[2], 0.0);
  assertClose(buffer.samples[3], -1.0);
});

test("PulseOscillator evaluates high and low states according to duty cycle", () => {
  const pulse = new PulseOscillator(10.0, 0.25, 2.0, 0.0, 1.0);
  // pos in [0, 0.25) => offset + amplitude = 1 + 2 = 3.0
  assertClose(pulse.valueAt(0.0), 3.0);
  assertClose(pulse.valueAt(0.01), 3.0);
  // pos in [0.25, 1.0) => offset - amplitude = 1 - 2 = -1.0
  assertClose(pulse.valueAt(0.03), -1.0);
});

test("SawtoothOscillator generates linear bipolar ramp", () => {
  const saw = new SawtoothOscillator(1.0, 1.0, 0.0, 0.0);
  assertClose(saw.valueAt(0.0), -1.0);
  assertClose(saw.valueAt(0.25), -0.5);
  assertClose(saw.valueAt(0.5), 0.0);
  assertClose(saw.valueAt(0.75), 0.5);
  // Near end of cycle approaches +1.0
  assertClose(saw.valueAt(0.999), 0.998);
});

test("TriangleOscillator generates symmetric linear triangle matching sine phase", () => {
  const tri = new TriangleOscillator(1.0, 1.0, 0.0, 0.0);
  assertClose(tri.valueAt(0.0), 0.0);
  assertClose(tri.valueAt(0.25), 1.0);
  assertClose(tri.valueAt(0.5), 0.0);
  assertClose(tri.valueAt(0.75), -1.0);
  assertClose(tri.valueAt(1.0), 0.0);
});

test("UniformSampler generator streaming matches sampleCount buffer", () => {
  const signal = new TriangleOscillator(440.0);
  const sampler = new UniformSampler(44_100.0);
  const count = 100;

  const buf = sampler.sampleCount(signal, count, 0.5);
  const streamed = Array.from(sampler.samples(signal, count, 0.5));

  assert.equal(buf.sampleCount(), count);
  assert.equal(streamed.length, count);
  for (let i = 0; i < count; i += 1) {
    assertClose(buf.samples[i], streamed[i]);
  }
});

test("SampleBuffer properties and copy immutability", () => {
  const rawArray = [0.1, -0.2, 0.3];
  const buf = new SampleBuffer(rawArray, 1000.0, 2.0);

  assert.equal(buf.sampleCount(), 3);
  assert.equal(buf.samplePeriodSeconds(), 0.001);
  assertClose(buf.durationSeconds(), 0.003);
  assertClose(buf.timeAt(0), 2.0);
  assertClose(buf.timeAt(2), 2.002);

  // Original array mutation does not affect internal buffer
  rawArray[0] = 999.0;
  assertClose(buf.samples[0], 0.1);
});

test("OscillatorState step and phase accumulation for sine", () => {
  const osc = new OscillatorState({
    waveform: Waveform.Sine,
    frequencyHz: 1.0,
    sampleRateHz: 4.0,
  });

  assertClose(osc.step(), 0.0);
  assertClose(osc.step(), 1.0);
  assertClose(osc.step(), 0.0);
  assertClose(osc.step(), -1.0);
  assertClose(osc.step(), 0.0);

  osc.reset(0.25);
  assertClose(osc.phase, 0.25);
  assertClose(osc.step(), 1.0);
});

test("OscillatorState renderSamples across all waveforms", () => {
  const waveforms: Array<"sine" | "square" | "triangle" | "sawtooth" | "pulse"> = [
    "sine",
    "square",
    "triangle",
    "sawtooth",
    "pulse",
  ];

  for (const wave of waveforms) {
    const osc = new OscillatorState({ waveform: wave, frequencyHz: 100, sampleRateHz: 1000 });
    const samples = osc.renderSamples(50);
    assert.equal(samples.length, 50);
    for (let i = 0; i < samples.length; i += 1) {
      assert.ok(Number.isFinite(samples[i]), `${wave} sample ${i} is not finite`);
      assert.ok(samples[i] >= -1.0 - EPSILON && samples[i] <= 1.0 + EPSILON);
    }
  }
});

test("Deterministic noise with SplitMix64 and LcgPrng", () => {
  const oscSplit1 = new OscillatorState({ waveform: "noise", noiseType: "splitmix64", seed: 42n });
  const oscSplit2 = new OscillatorState({ waveform: "noise", noiseType: "splitmix64", seed: 42n });
  const buf1 = oscSplit1.renderSamples(64);
  const buf2 = oscSplit2.renderSamples(64);
  assert.deepEqual(Array.from(buf1), Array.from(buf2));

  const oscLcg1 = new OscillatorState({ waveform: "noise", noiseType: "lcg", seed: 12345 });
  const oscLcg2 = new OscillatorState({ waveform: "noise", noiseType: "lcg", seed: 12345 });
  const lcgBuf1 = oscLcg1.renderSamples(64);
  const lcgBuf2 = oscLcg2.renderSamples(64);
  assert.deepEqual(Array.from(lcgBuf1), Array.from(lcgBuf2));

  // SplitMix64 and Lcg are distinct PRNG sequences
  assert.notDeepEqual(Array.from(buf1), Array.from(lcgBuf1));
});

test("SplitMix64 direct PRNG properties", () => {
  const rng1 = new SplitMix64(0n);
  const rng2 = new SplitMix64(0);
  assert.equal(rng1.nextU64(), rng2.nextU64());

  const f = rng1.nextFloat();
  assert.ok(f >= -1.0 && f <= 1.0);
});

test("LcgPrng direct PRNG properties", () => {
  const lcg1 = new LcgPrng(99);
  const lcg2 = new LcgPrng(99);
  assert.equal(lcg1.nextU32(), lcg2.nextU32());
  const f = lcg1.nextFloat();
  assert.ok(f >= -1.0 && f <= 1.0);
});

test("renderSamples and synthesize convenience helpers", () => {
  const rendered = renderSamples({ waveform: "square", frequencyHz: 2.0, sampleRateHz: 8.0 }, 8);
  assert.equal(rendered.length, 8);

  const synthBuf = synthesize(
    { waveform: "sine", frequencyHz: 440.0, sampleRateHz: 44_100.0 },
    0.01,
  );
  assert.equal(synthBuf.sampleCount(), 441);
  assert.equal(synthBuf.sampleRateHz, 44_100.0);
});

test("sampleCountForDuration handles precision rounding boundaries", () => {
  assert.equal(sampleCountForDuration(1.0, 44_100.0), 44_100);
  assert.equal(sampleCountForDuration(0.5, 48_000.0), 24_000);
  assert.equal(sampleCountForDuration(0.01, 48_000.0), 480);
  assert.equal(sampleCountForDuration(0.0, 44_100.0), 0);
});

test("timeAtSample computes exact uniform grid times", () => {
  assertClose(timeAtSample(0, 100.0, 5.0), 5.0);
  assertClose(timeAtSample(10, 100.0, 5.0), 5.1);
});

test("oscillator.warrant operation conformance", () => {
  // Warrant examples:
  // nyquist_44100: calc_nyquist(44100) == Some(22050)
  assert.equal(calcNyquist(44_100), 22_050);
  // nyquist_48000: calc_nyquist(48000) == Some(24000)
  assert.equal(calcNyquist(48_000), 24_000);

  // square_duty_50_below: eval_square_duty(25, 50) == true
  assert.equal(evalSquareDuty(25, 50), true);
  // square_duty_50_above: eval_square_duty(75, 50) == false
  assert.equal(evalSquareDuty(75, 50), false);

  // Invalid values return null/false fail-closed
  assert.equal(calcNyquist(-1), null);
  assert.equal(calcNyquist(0), null);
  assert.equal(calcNyquist(192_001), null);
  assert.equal(calcNyquist(Number.NaN), null);
  assert.equal(evalSquareDuty(Number.NaN, 50), false);
});
