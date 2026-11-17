import assert from "node:assert/strict";
import test from "node:test";

import {
  autocorrelationPitch,
  autocorrelation_pitch,
  BudgetExceededError,
  hpsPitch,
  hpsPitchSimple,
  hps_pitch,
  hps_pitch_simple,
  InvalidInputError,
  yinPitch,
  yinPitchSimple,
  yinPitchTrack,
  yin_pitch,
  yin_pitch_simple,
  yin_pitch_track,
} from "../src/index.js";

function synthesizeSine(freqHz: number, sampleRateHz: number, durationSec: number): Float64Array {
  const n = Math.floor(sampleRateHz * durationSec);
  const phaseInc = (2.0 * Math.PI * freqHz) / sampleRateHz;
  const out = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    out[i] = Math.sin(i * phaseInc);
  }
  return out;
}

function synthesizeHarmonic(
  f0: number,
  harmonics: number[],
  sampleRateHz: number,
  durationSec: number,
): Float64Array {
  const n = Math.floor(sampleRateHz * durationSec);
  const out = new Float64Array(n);
  for (let hIdx = 0; hIdx < harmonics.length; hIdx++) {
    const h = hIdx + 1;
    const freq = h * f0;
    const amp = harmonics[hIdx];
    if (freq < sampleRateHz / 2.0) {
      const phaseInc = (2.0 * Math.PI * freq) / sampleRateHz;
      for (let i = 0; i < n; i++) {
        out[i] += amp * Math.sin(i * phaseInc);
      }
    }
  }
  return out;
}

test("yinPitch: pure sines across multiple octaves", () => {
  const sr = 44100.0;
  for (const targetFreq of [110.0, 220.0, 440.0, 880.0]) {
    const sig = synthesizeSine(targetFreq, sr, 0.1);
    const est = yinPitch(sig, sr, 50.0, 1200.0, 0.15);
    const relErr = Math.abs(est - targetFreq) / targetFreq;
    assert.ok(
      relErr < 0.01,
      `target ${targetFreq}, got ${est}, relErr ${relErr}`,
    );
  }
});

test("yinPitch: harmonic signal (C4)", () => {
  const sr = 44100.0;
  const targetF0 = 261.63; // C4
  const sig = synthesizeHarmonic(targetF0, [1.0, 0.6, 0.4, 0.2], sr, 0.1);
  const est = yinPitch(sig, sr, 80.0, 600.0, 0.15);
  const relErr = Math.abs(est - targetF0) / targetF0;
  assert.ok(relErr < 0.01);
});

test("yinPitch: silence returns 0.0", () => {
  const sr = 44100.0;
  const sig = new Float64Array(2000);
  assert.equal(yinPitchSimple(sig, sr), 0.0);
});

test("yinPitchTrack: two note sequence tracking", () => {
  const sr = 44100.0;
  const sig1 = synthesizeSine(220.0, sr, 0.1);
  const sig2 = synthesizeSine(440.0, sr, 0.1);
  const combined = new Float64Array(sig1.length + sig2.length);
  combined.set(sig1, 0);
  combined.set(sig2, sig1.length);

  const track = yinPitchTrack(combined, sr, 1024, 256, 40.0, 2000.0, 0.15);
  assert.ok(track.length > 5);

  const earlyPitches = track.slice(0, 4).filter((p) => p > 0.0);
  assert.ok(earlyPitches.some((p) => Math.abs(p - 220.0) / 220.0 < 0.02));

  const latePitches = track.slice(track.length - 4).filter((p) => p > 0.0);
  assert.ok(latePitches.some((p) => Math.abs(p - 440.0) / 440.0 < 0.02));
});

test("hpsPitch: harmonic signal fundamental detection", () => {
  const sr = 44100.0;
  const targetF0 = 440.0;
  const sig = synthesizeHarmonic(targetF0, [1.0, 0.8, 0.6, 0.4, 0.2], sr, 0.15);
  const est = hpsPitch(sig, sr, 5, 100.0, 1000.0, undefined);
  const relErr = Math.abs(est - targetF0) / targetF0;
  assert.ok(
    relErr < 0.02,
    `target ${targetF0}, got ${est}, relErr ${relErr}`,
  );
});

test("hpsPitch: silence and explicit nFft", () => {
  const sr = 44100.0;
  const sigSilence = new Float64Array(2048);
  assert.equal(hpsPitchSimple(sigSilence, sr), 0.0);

  const sig = synthesizeHarmonic(440.0, [1.0, 0.8, 0.6, 0.4], sr, 0.15);
  const est = hpsPitch(sig, sr, 5, 40.0, 2000.0, 2048);
  const relErr = Math.abs(est - 440.0) / 440.0;
  assert.ok(relErr < 0.03);

  // Inverted min/max search bin condition returns 0.0
  assert.equal(hpsPitch(sig, sr, 5, 440.0, 440.1, 512), 0.0);
});

test("autocorrelationPitch: fundamental pitch estimation", () => {
  const sr = 44100.0;
  const targetF0 = 440.0;
  const sig = synthesizeSine(targetF0, sr, 0.1);
  const est = autocorrelationPitch(sig, sr, 100.0, 1000.0);
  const relErr = Math.abs(est - targetF0) / targetF0;
  assert.ok(relErr < 0.02, `autocorr est: ${est}, target: ${targetF0}`);

  // Silence
  assert.equal(autocorrelationPitch(new Float64Array(1000), sr), 0.0);

  // Short signal
  assert.equal(autocorrelationPitch([0.1, -0.1], sr), 0.0);
});

test("pitch: aliases match camelCase", () => {
  const sr = 44100.0;
  const sig = synthesizeSine(440.0, sr, 0.05);
  assert.equal(yin_pitch(sig, sr, 40.0, 2000.0, 0.15), yinPitch(sig, sr, 40.0, 2000.0, 0.15));
  assert.equal(yin_pitch_simple(sig, sr), yinPitchSimple(sig, sr));
  assert.deepEqual(yin_pitch_track(sig, sr, 512, 256), yinPitchTrack(sig, sr, 512, 256));
  assert.equal(hps_pitch(sig, sr), hpsPitch(sig, sr));
  assert.equal(hps_pitch_simple(sig, sr), hpsPitchSimple(sig, sr));
  assert.equal(autocorrelation_pitch(sig, sr), autocorrelationPitch(sig, sr));
});

test("pitch: invalid and out-of-bounds inputs", () => {
  assert.throws(() => yinPitchSimple([], 44100.0), InvalidInputError);
  assert.throws(() => yinPitchSimple([1.0], -44100.0), InvalidInputError);
  assert.throws(() => yinPitch([1.0, 2.0], 44100.0, 1000.0, 500.0, 0.15), InvalidInputError);
  assert.throws(() => hpsPitch([1.0, 2.0], 44100.0, 5, 1000.0, 500.0), InvalidInputError);
  assert.throws(() => hpsPitch([1.0, 2.0], 44100.0, 0, 40.0, 2000.0), InvalidInputError);
  assert.throws(() => hpsPitch([1.0, 2.0], 44100.0, 65, 40.0, 2000.0), BudgetExceededError);
  assert.throws(() => autocorrelationPitch([1.0, 2.0], 44100.0, 1000.0, 500.0), InvalidInputError);
});
