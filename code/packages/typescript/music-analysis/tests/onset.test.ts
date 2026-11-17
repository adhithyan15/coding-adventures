import assert from "node:assert/strict";
import test from "node:test";

import {
  detectOnsets,
  detectOnsetsSamples,
  detectOnsetsSimple,
  detect_onsets,
  detect_onsets_samples,
  detect_onsets_simple,
  estimateTempo,
  estimate_tempo,
  InvalidInputError,
} from "../src/index.js";

function makeToneBurst(freq: number, durationSec: number, sampleRateHz: number): Float64Array {
  const n = Math.floor(sampleRateHz * durationSec);
  const phaseInc = (2.0 * Math.PI * freq) / sampleRateHz;
  const out = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const t = i / sampleRateHz;
    const env = Math.exp(-15.0 * t);
    out[i] = env * Math.sin(i * phaseInc);
  }
  return out;
}

test("detectOnsets: single burst near 0.0s", () => {
  const sr = 44100.0;
  const burst = makeToneBurst(440.0, 0.2, sr);
  const onsets = detectOnsetsSimple(burst, sr);
  assert.ok(onsets.length > 0);
  assert.ok(Math.abs(onsets[0] - 0.0) < 0.03);
});

test("detectOnsets: multiple burst events separated by silence", () => {
  const sr = 44100.0;
  const silence = new Float64Array(Math.floor(0.15 * sr));
  const burst1 = makeToneBurst(440.0, 0.1, sr);
  const burst2 = makeToneBurst(880.0, 0.1, sr);
  const burst3 = makeToneBurst(550.0, 0.1, sr);

  // 3 bursts separated by silence
  const totalLen = burst1.length + silence.length + burst2.length + silence.length + burst3.length;
  const sig = new Float64Array(totalLen);
  let offset = 0;
  sig.set(burst1, offset); offset += burst1.length;
  sig.set(silence, offset); offset += silence.length;
  sig.set(burst2, offset); offset += burst2.length;
  sig.set(silence, offset); offset += silence.length;
  sig.set(burst3, offset);

  const onsets = detectOnsets(sig, sr, 512, 1024, 1.5, 0.1);
  assert.equal(onsets.length, 3);
  assert.ok(Math.abs(onsets[0] - 0.0) < 0.03);
  assert.ok(Math.abs(onsets[1] - 0.25) < 0.05);
  assert.ok(Math.abs(onsets[2] - 0.50) < 0.05);
});

test("detectOnsetsSamples: returns sample indices", () => {
  const sr = 44100.0;
  const burst = makeToneBurst(440.0, 0.2, sr);
  const sampleOnsets = detectOnsetsSamples(burst, sr, 512, 1024, 1.5, 0.08);
  assert.ok(sampleOnsets.length > 0);
  assert.equal(sampleOnsets[0], 0);
});

test("detectOnsets: silence returns empty array", () => {
  const sr = 44100.0;
  const sig = new Float64Array(4096);
  const onsets = detectOnsetsSimple(sig, sr);
  assert.equal(onsets.length, 0);
});

test("detectOnsets: short signal returns [0.0]", () => {
  const sr = 44100.0;
  const shortSig = [0.5, 0.8, -0.4, 0.2];
  const onsets = detectOnsetsSimple(shortSig, sr);
  assert.deepEqual(onsets, [0.0]);
});

test("estimateTempo: estimates tempo from periodic events", () => {
  const sr = 44100.0;
  // 120 BPM = 2 beats per second -> 0.5s interval
  const totalSec = 2.0;
  const sig = new Float64Array(Math.floor(sr * totalSec));
  const burst = makeToneBurst(440.0, 0.05, sr);

  // Place bursts at 0.0, 0.5, 1.0, 1.5 s
  for (const t of [0.0, 0.5, 1.0, 1.5]) {
    const idx = Math.floor(t * sr);
    sig.set(burst, idx);
  }

  const bpm = estimateTempo(sig, sr, 60.0, 200.0);
  assert.ok(Math.abs(bpm - 120.0) < 5.0, `estimated bpm: ${bpm}`);

  // Silence falls back to default 120
  const silenceBpm = estimateTempo(new Float64Array(4000), sr);
  assert.equal(silenceBpm, 120.0);
});

test("onset: aliases match camelCase", () => {
  const sr = 44100.0;
  const burst = makeToneBurst(440.0, 0.1, sr);
  assert.deepEqual(detect_onsets(burst, sr), detectOnsets(burst, sr));
  assert.deepEqual(detect_onsets_samples(burst, sr), detectOnsetsSamples(burst, sr));
  assert.deepEqual(detect_onsets_simple(burst, sr), detectOnsetsSimple(burst, sr));
  assert.equal(estimate_tempo(burst, sr), estimateTempo(burst, sr));
});

test("onset: invalid inputs", () => {
  assert.throws(() => detectOnsetsSimple([], 44100.0), InvalidInputError);
  assert.throws(() => detectOnsetsSimple([1.0], -44100.0), InvalidInputError);
  assert.throws(() => estimateTempo([1.0], 44100.0, 150.0, 100.0), InvalidInputError);
});
