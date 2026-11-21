import assert from "node:assert/strict";
import test from "node:test";

import {
  calcNyquist,
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
  UniformSampler,
} from "../src/index.js";

test("CWE-20: SineOscillator rejects non-finite or invalid parameters", () => {
  assert.throws(() => new SineOscillator(Number.NaN), /finite/);
  assert.throws(() => new SineOscillator(Number.POSITIVE_INFINITY), /finite/);
  assert.throws(() => new SineOscillator(-1.0), /non-negative/);
  assert.throws(() => new SineOscillator(MAX_FREQ_HZ + 1), /exceeds maximum allowable/);
  assert.throws(() => new SineOscillator(440, Number.NaN), /finite/);
  assert.throws(() => new SineOscillator(440, -1.0), /non-negative/);
  assert.throws(() => new SineOscillator(440, 1.0, Number.NaN), /finite/);
  assert.throws(() => new SineOscillator(440, 1.0, 0.0, Number.NaN), /finite/);

  const osc = new SineOscillator(440);
  assert.throws(() => osc.valueAt(Number.NaN), /finite/);
  assert.throws(() => osc.valueAt(Number.POSITIVE_INFINITY), /finite/);
});

test("CWE-20: SquareOscillator rejects invalid duty cycles and levels", () => {
  assert.throws(() => new SquareOscillator(10, 0, 1, 0.0), /0.0 < duty_cycle < 1.0/);
  assert.throws(() => new SquareOscillator(10, 0, 1, 1.0), /0.0 < duty_cycle < 1.0/);
  assert.throws(() => new SquareOscillator(10, 0, 1, -0.1), /0.0 < duty_cycle < 1.0/);
  assert.throws(() => new SquareOscillator(10, 0, 1, 1.5), /0.0 < duty_cycle < 1.0/);
  assert.throws(() => new SquareOscillator(10, Number.NaN), /finite/);
  assert.throws(() => new SquareOscillator(10, 0, Number.NaN), /finite/);

  const osc = new SquareOscillator(10);
  assert.throws(() => osc.valueAt(Number.NaN), /finite/);
});

test("CWE-20: PulseOscillator, SawtoothOscillator, TriangleOscillator reject bad inputs", () => {
  assert.throws(() => new PulseOscillator(10, 0.0), /duty_cycle/);
  assert.throws(() => new PulseOscillator(10, 0.5, -1.0), /non-negative/);
  assert.throws(() => new SawtoothOscillator(MAX_FREQ_HZ + 10), /exceeds/);
  assert.throws(() => new TriangleOscillator(10, -5), /non-negative/);
});

test("CWE-400: UniformSampler and sampleCountForDuration resource exhaustion defenses", () => {
  assert.throws(() => new UniformSampler(0.0), /positive/);
  assert.throws(() => new UniformSampler(-10.0), /positive/);
  assert.throws(() => new UniformSampler(Number.NaN), /finite/);
  assert.throws(() => new UniformSampler(MAX_SAMPLE_RATE_HZ + 1.0), /exceeds maximum allowable/);

  assert.throws(() => nyquistFrequency(0), /positive/);
  assert.throws(() => nyquistFrequency(MAX_SAMPLE_RATE_HZ + 100), /exceeds/);

  assert.throws(() => sampleCountForDuration(-1.0, 44_100), /non-negative/);
  assert.throws(() => sampleCountForDuration(MAX_DURATION_SECONDS + 1.0, 44_100), /exceeds/);
  assert.throws(() => sampleCountForDuration(Number.NaN, 44_100), /finite/);

  const sampler = new UniformSampler(44_100);
  const signal = new SineOscillator(440);
  assert.throws(() => sampler.sampleCount(signal, -1), /non-negative/);
  assert.throws(() => sampler.sampleCount(signal, 1.5), /integer/);
  assert.throws(() => sampler.sampleCount(signal, MAX_SAMPLES + 1), /exceeds maximum allowable/);

  // @ts-expect-error test non-signal input
  assert.throws(() => sampler.sample(null, 1.0), /ContinuousSignal/);
  // @ts-expect-error test non-signal input
  assert.throws(() => Array.from(sampler.samples(null, 10)), /ContinuousSignal/);
});

test("CWE-20 / CWE-400: SampleBuffer rejects non-arrays, non-finite samples, and out-of-bounds indices", () => {
  // @ts-expect-error test invalid sample buffer construction
  assert.throws(() => new SampleBuffer(null, 44_100), /Float64Array/);
  assert.throws(() => new SampleBuffer([0.0, Number.NaN, 1.0], 44_100), /finite/);
  assert.throws(() => new SampleBuffer([0.0, Number.POSITIVE_INFINITY], 44_100), /finite/);

  // Non-integer, NaN, or out-of-bounds length
  // @ts-expect-error test non-integer length
  assert.throws(() => new SampleBuffer({ length: Number.NaN }, 44_100), /integer/);
  // @ts-expect-error test fractional length
  assert.throws(() => new SampleBuffer({ length: 1.5 }, 44_100), /integer/);
  // @ts-expect-error test negative length
  assert.throws(() => new SampleBuffer({ length: -1 }, 44_100), /non-negative/);
  // @ts-expect-error test oversized length
  assert.throws(() => new SampleBuffer({ length: MAX_SAMPLES + 1 }, 44_100), /exceeds maximum allowable/);

  const buf = new SampleBuffer([1.0, 2.0], 44_100);
  assert.throws(() => buf.timeAt(-1), /out of bounds/);
  assert.throws(() => buf.timeAt(2), /out of bounds/);
  assert.throws(() => buf.timeAt(1.5), /integer/);
  assert.throws(() => buf.timeAt(Number.NaN), /integer/);
});

test("CWE-20: timeAtSample rejects invalid indices or sample rates", () => {
  assert.throws(() => timeAtSample(-1, 44_100), /non-negative/);
  assert.throws(() => timeAtSample(1.5, 44_100), /integer/);
  assert.throws(() => timeAtSample(MAX_SAMPLES + 1, 44_100), /MAX_SAMPLES/);
  assert.throws(() => timeAtSample(0, 0), /positive/);
  assert.throws(() => timeAtSample(0, 44_100, Number.NaN), /finite/);
});

test("CWE-20: OscillatorState rejects unknown waveforms and invalid parameters", () => {
  // @ts-expect-error invalid waveform
  assert.throws(() => new OscillatorState({ waveform: "hypersine" }), /invalid waveform/);
  // @ts-expect-error invalid noiseType
  assert.throws(() => new OscillatorState({ waveform: "noise", noiseType: "invalid-noise" }), RangeError);
  // @ts-expect-error invalid noiseType
  assert.throws(() => new OscillatorState({ waveform: "noise", noiseType: "mersenne" }), RangeError);
  assert.throws(() => new OscillatorState({ frequencyHz: -10 }), /non-negative/);
  assert.throws(() => new OscillatorState({ frequencyHz: MAX_FREQ_HZ + 10 }), /exceeds/);
  assert.throws(() => new OscillatorState({ sampleRateHz: 0 }), /positive/);
  assert.throws(() => new OscillatorState({ amplitude: -1 }), /non-negative/);
  assert.throws(() => new OscillatorState({ dutyCycle: 1.0 }), /duty_cycle/);
  assert.throws(() => new OscillatorState({ phaseCycles: Number.NaN }), /finite/);

  const osc = new OscillatorState();
  assert.throws(() => osc.reset(Number.NaN), /finite/);
  assert.throws(() => osc.renderSamples(-1), /non-negative/);
  assert.throws(() => osc.renderSamples(1.5), /integer/);
  assert.throws(() => osc.renderSamples(MAX_SAMPLES + 1), /exceeds/);

  assert.throws(() => renderSamples({}, -5), /non-negative/);
  assert.throws(() => synthesize({}, MAX_DURATION_SECONDS + 10), /exceeds/);
});

test("CWE-20: PRNG engines reject non-finite seeds and invalid types", () => {
  assert.throws(() => new SplitMix64(Number.NaN), /finite/);
  assert.throws(() => new SplitMix64(Number.POSITIVE_INFINITY), /finite/);
  // @ts-expect-error invalid seed type
  assert.throws(() => new SplitMix64("bad-seed"), /number or bigint/);

  assert.throws(() => new LcgPrng(Number.NaN), /finite/);
  assert.throws(() => new LcgPrng(Number.POSITIVE_INFINITY), /finite/);
  // @ts-expect-error invalid seed type
  assert.throws(() => new LcgPrng("bad-seed"), /finite/);
});

test("VER15: rejects > MAX_SAFE_INTEGER (1e100), Symbol, Object.create(null), and DataView cleanly", () => {
  // Number.isSafeInteger checks (rejects 1e100 without falling through to range or truncation)
  assert.throws(() => new SampleBuffer({ length: 1e100 } as unknown as number[], 44_100), TypeError);
  assert.throws(() => timeAtSample(1e100, 44_100), TypeError);
  assert.throws(() => new OscillatorState().renderSamples(1e100), TypeError);
  assert.equal(calcNyquist(1e100), null);
  assert.equal(calcNyquist(Number.MAX_SAFE_INTEGER + 1), null);

  const buf = new SampleBuffer([1.0, 2.0], 44_100);
  assert.throws(() => buf.timeAt(1e100), TypeError);

  // Symbol & Object.create(null) must not crash String() or Number()
  const nullProto = Object.create(null);
  const sym = Symbol("hostile");
  const throwingObj = {
    toString() {
      throw new Error("boom toString");
    },
  };
  for (const hostile of [nullProto, sym, throwingObj]) {
    assert.throws(() => new SineOscillator(hostile as number), TypeError);
    assert.throws(() => new LcgPrng(hostile as number), TypeError);
    assert.throws(() => new OscillatorState({ waveform: hostile as never }), TypeError);
    assert.throws(() => new OscillatorState({ waveform: "noise", noiseType: hostile as never }), RangeError);
    assert.throws(() => new OscillatorState({ waveform: "noise", noiseType: "lcg", seed: hostile as never }), TypeError);
    assert.throws(() => new SampleBuffer([hostile as number], 44_100), TypeError);
  }

  // ArrayBuffer.isView: DataView has no .length; BigInt64Array elements are bigints; oversized typed arrays hit pre-flight cap
  const dv = new DataView(new ArrayBuffer(16));
  assert.throws(() => new SampleBuffer(dv as unknown as Float64Array, 44_100), TypeError);
  const bi = new BigInt64Array([1n, 2n]);
  assert.throws(() => new SampleBuffer(bi as unknown as Float64Array, 44_100), TypeError);
  const oversized = new Uint8Array(MAX_SAMPLES + 1);
  assert.throws(() => new SampleBuffer(oversized as unknown as Float64Array, 44_100), /exceeds maximum allowable/);

  // CWE-1321: prototype pollution attempt via JSON-parsed config must not pollute Object.prototype
  const parsedConfig = JSON.parse('{"waveform":"sine","__proto__":{"pollutedOsc":true}}');
  const osc = new OscillatorState(parsedConfig);
  assert.equal(osc.waveform, "sine");
  assert.equal(({} as { pollutedOsc?: boolean }).pollutedOsc, undefined);

  // Non-object / null config must fail closed cleanly
  assert.throws(() => new OscillatorState(null as never), TypeError);
  assert.throws(() => new OscillatorState("bad-config" as never), TypeError);
  assert.throws(() => synthesize(null as never, 0.1), TypeError);
  assert.throws(() => synthesize("bad-config" as never, 0.1), TypeError);
});
