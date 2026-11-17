import assert from "node:assert/strict";
import test from "node:test";
import {
  parsePortableScore,
  renderPortableScoreToPcm,
  PORTABLE_FORMAT_VERSION,
} from "@coding-adventures/music-machine-web";
import { PCMBuffer, PCMFormat } from "@coding-adventures/pcm-audio";

import {
  transcribePcmBuffer,
  transcribePcmToPortableScore,
  transcribeSamples,
  transcribe_pcm_buffer,
  transcribe_pcm_to_portable_score,
  transcribe_samples,
  InvalidInputError,
  BudgetExceededError,
} from "../src/index.js";

const TRIAD_SCORE_TEXT = `format: music-machine-score/v2
title: Round Trip Triad
ppq: 480
sample_rate: 44100
tempo 0 120
meter 0 4/4

instrument lead profile=sine gain=0.8
track melody instrument=lead

event melody 0 480 note C4 velocity=0.8
event melody 480 480 note E4 velocity=0.8
event melody 960 480 note G4 velocity=0.8
`;

test("transcribePcmToPortableScore: portable score round trip", () => {
  const originalScore = parsePortableScore(TRIAD_SCORE_TEXT);
  const rendered = renderPortableScoreToPcm(originalScore, 0);

  const opts = {
    bpm: 120.0,
    ppq: 480,
    quantizeGridBeats: 0.25,
    title: "Transcribed Triad",
    instrumentId: "sine",
    trackId: "melody",
  };

  const transcribed = transcribePcmBuffer(rendered.pcmBuffer, opts);

  assert.equal(transcribed.formatVersion, PORTABLE_FORMAT_VERSION);
  assert.equal(transcribed.title, "Transcribed Triad");
  assert.equal(transcribed.ppq, 480);
  assert.equal(transcribed.sampleRateHz, 44100);

  // Verify notes and ticks match
  assert.equal(transcribed.events.length, 3);

  const e0 = transcribed.events[0];
  assert.deepEqual(e0.notes, ["C4"]);
  assert.equal(e0.startTick, 0);
  assert.equal(e0.durationTick, 480);

  const e1 = transcribed.events[1];
  assert.deepEqual(e1.notes, ["E4"]);
  assert.equal(e1.startTick, 480);
  assert.equal(e1.durationTick, 480);

  const e2 = transcribed.events[2];
  assert.deepEqual(e2.notes, ["G4"]);
  assert.equal(e2.startTick, 960);
  assert.equal(e2.durationTick, 480);
});

test("transcribeSamples: raw float sequence", () => {
  const sr = 44100.0;
  const bpm = 120.0;
  // Create 0.5s of A4 (440 Hz)
  const n = Math.floor(0.5 * sr);
  const phaseInc = (2.0 * Math.PI * 440.0) / sr;
  const sig = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    sig[i] = 0.8 * Math.sin(i * phaseInc);
  }

  const score = transcribeSamples(sig, sr, { bpm });
  assert.equal(score.events.length, 1);
  const event = score.events[0];
  assert.deepEqual(event.notes, ["A4"]);
  assert.equal(event.startTick, 0);
});

test("transcribeSamples: pcm16 integer sequence", () => {
  const sr = 44100.0;
  const bpm = 120.0;
  // Create 0.5s of C4 (261.63 Hz) in 16-bit PCM integer range
  const n = Math.floor(0.5 * sr);
  const phaseInc = (2.0 * Math.PI * 261.63) / sr;
  const sig = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    sig[i] = 24000.0 * Math.sin(i * phaseInc);
  }

  const score = transcribeSamples(sig, sr, { bpm });
  assert.equal(score.events.length, 1);
  assert.deepEqual(score.events[0].notes, ["C4"]);
});

test("transcribeSamples: silence yields no events", () => {
  const sr = 44100.0;
  const sig = new Float64Array(8000);
  const score = transcribeSamples(sig, sr);
  assert.equal(score.events.length, 0);
});

test("transcribePcmBuffer: PCMBuffer metadata inferred automatically", () => {
  const sr = 22050.0;
  const format = new PCMFormat(sr);
  const n = Math.floor(0.5 * sr);
  const phaseInc = (2.0 * Math.PI * 440.0) / sr;
  const intSamples: number[] = [];
  for (let i = 0; i < n; i++) {
    intSamples.push(Math.round(20000.0 * Math.sin(i * phaseInc)));
  }
  const buf = new PCMBuffer(intSamples, format);

  const score = transcribePcmBuffer(buf);
  assert.equal(score.sampleRateHz, 22050);
  assert.equal(score.events.length, 1);
  assert.deepEqual(score.events[0].notes, ["A4"]);
});

test("transcription: aliases match camelCase", () => {
  const sr = 44100.0;
  const sig = new Float64Array(1000);
  assert.equal(
    transcribe_samples(sig, sr).events.length,
    transcribeSamples(sig, sr).events.length,
  );
  assert.equal(
    transcribe_pcm_to_portable_score(sig, sr).events.length,
    transcribePcmToPortableScore(sig, sr).events.length,
  );
});

test("transcribe: invalid and out-of-bounds inputs", () => {
  assert.throws(
    () => transcribePcmToPortableScore([1.0], undefined),
    InvalidInputError,
  );
  assert.throws(
    () => transcribeSamples([1.0], 500.0),
    InvalidInputError,
  );
  assert.throws(
    () => transcribeSamples([1.0], 44100.0, { bpm: 0.0 }),
    InvalidInputError,
  );
  assert.throws(
    () => transcribeSamples([1.0], 44100.0, { bpm: 1001.0 }),
    BudgetExceededError,
  );
  assert.throws(
    () => transcribeSamples([NaN], 44100.0),
    InvalidInputError,
  );
});
