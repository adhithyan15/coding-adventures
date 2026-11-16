import assert from "node:assert/strict";
import test from "node:test";
import { PCMFormat, PCMBuffer } from "@coding-adventures/pcm-audio";

import {
  AnalysisError,
  autocorrelationPitch,
  BudgetExceededError,
  buildHannWindow,
  checkedFrameBounds,
  detectOnsets,
  detectOnsetsSamples,
  estimateTempo,
  extractHarmonicProfile,
  extractHarmonicProfileSimple,
  frequencyToNote,
  hpsPitch,
  InvalidInputError,
  MAX_FFT_LEN,
  MAX_FRAMES,
  MAX_SAMPLES,
  parabolicInterpolation,
  rfftMagnitudes,
  rmsEnergy,
  spectralCentroid,
  spectralFlux,
  spectralRolloff,
  transcribePcmBuffer,
  transcribePcmToPortableScore,
  transcribeSamples,
  validateAudioInputs,
  validateFiniteFloat,
  validateFraming,
  validateNonNegativeInt,
  validatePcmSourceBounds,
  validatePositiveFloat,
  validatePositiveInt,
  validateSamples,
  validateSpectrum,
  yinPitch,
  yinPitchTrack,
  zeroCrossingRate,
  zeroCrossingRateFramed,
} from "../src/index.js";

test("Coverage: dsp_utils edge cases", () => {
  // Hann window edge cases
  assert.equal(buildHannWindow(0).length, 0);
  assert.equal(buildHannWindow(-1).length, 0);
  assert.deepEqual(buildHannWindow(1), new Float64Array([1.0]));
  assert.equal(buildHannWindow(4).length, 4);

  // Parabolic interpolation edge cases
  // Flat/linear curve where denom === 0
  assert.equal(parabolicInterpolation(1.0, 1.0, 1.0, 5), 5);
  // Abs delta >= 1.0
  assert.equal(parabolicInterpolation(5.0, 4.0, 3.5, 5), 5);

  // rfftMagnitudes edge cases
  assert.throws(() => rfftMagnitudes([]), InvalidInputError);
  assert.throws(
    () => rfftMagnitudes(new Float64Array(MAX_FFT_LEN + 1)),
    BudgetExceededError,
  );

  // validateFraming edge cases
  // signalLen < frameLength returns null
  assert.equal(validateFraming(50, 100, 20), null);
  // numFrames > MAX_FRAMES
  assert.throws(
    () => validateFraming(MAX_FRAMES + 100, 10, 1),
    BudgetExceededError,
  );

  // checkedFrameBounds edge cases
  // end > signalLen
  assert.throws(
    () => checkedFrameBounds(5, 20, 50, 100),
    BudgetExceededError,
  );
  // Safe integer bounds
  assert.throws(
    () => checkedFrameBounds(Number.MAX_SAFE_INTEGER, 2, 10, 100),
    BudgetExceededError,
  );
  assert.throws(
    () => checkedFrameBounds(0, 1, Number.MAX_SAFE_INTEGER + 10, 100),
    BudgetExceededError,
  );

  // validateAudioInputs
  const audio = validateAudioInputs([0.1, 0.2], 44100.0);
  assert.equal(audio.validSr, 44100.0);
  assert.equal(audio.validSamples.length, 2);

  // validatePcmSourceBounds
  assert.equal(validatePcmSourceBounds(100, 44100.0), 44100.0);
  assert.throws(() => validatePcmSourceBounds(0, 44100.0), InvalidInputError);
  assert.throws(
    () => validatePcmSourceBounds(MAX_SAMPLES + 1, 44100.0),
    BudgetExceededError,
  );
  assert.throws(() => validatePcmSourceBounds(10, 500.0), InvalidInputError);
});

test("Coverage: types validation helpers", () => {
  // validateFiniteFloat
  assert.equal(validateFiniteFloat(3.14, "pi"), 3.14);
  assert.throws(() => validateFiniteFloat("abc", "str"), InvalidInputError);
  assert.throws(() => validateFiniteFloat(NaN, "nan"), InvalidInputError);

  // validatePositiveFloat
  assert.throws(() => validatePositiveFloat(-1.0, "neg"), InvalidInputError);
  assert.throws(() => validatePositiveFloat(0.0, "zero"), InvalidInputError);
  assert.throws(() => validatePositiveFloat(5.0, "five", NaN), InvalidInputError);
  assert.throws(() => validatePositiveFloat(10.0, "ten", 5.0), BudgetExceededError);

  // validatePositiveInt
  assert.throws(() => validatePositiveInt(0, "zero"), InvalidInputError);
  assert.throws(() => validatePositiveInt(-3, "neg"), InvalidInputError);
  assert.throws(() => validatePositiveInt(1.5, "float"), InvalidInputError);
  assert.throws(() => validatePositiveInt(10, "ten", 5), BudgetExceededError);

  // validateNonNegativeInt
  assert.equal(validateNonNegativeInt(0, "zero"), 0);
  assert.equal(validateNonNegativeInt(5, "five", 10), 5);
  assert.throws(() => validateNonNegativeInt(-1, "neg"), InvalidInputError);
  assert.throws(() => validateNonNegativeInt(1.5, "float"), InvalidInputError);
  assert.throws(() => validateNonNegativeInt(11, "eleven", 10), BudgetExceededError);

  // validateSamples null/undefined / oversized
  assert.throws(() => validateSamples(null, "samples"), InvalidInputError);
  assert.throws(() => validateSamples(undefined, "samples"), InvalidInputError);
  assert.throws(
    () => validateSamples({ length: MAX_SAMPLES + 1 } as any, "samples"),
    BudgetExceededError,
  );

  // validateSpectrum null/undefined / oversized
  assert.throws(() => validateSpectrum(null, "spectrum"), InvalidInputError);
  assert.throws(() => validateSpectrum(undefined, "spectrum"), InvalidInputError);
  assert.throws(
    () => validateSpectrum(new Float64Array(MAX_FFT_LEN + 1), "spectrum"),
    BudgetExceededError,
  );

  // AnalysisError constructor methods
  const err1 = AnalysisError.InvalidInput("bad");
  assert.equal(err1.name, "InvalidInputError");
  assert.ok(err1 instanceof InvalidInputError);

  const err2 = AnalysisError.BudgetExceeded("too big");
  assert.equal(err2.name, "BudgetExceededError");
  assert.ok(err2 instanceof BudgetExceededError);
});

test("Coverage: pitch algorithms edge cases", () => {
  const sr = 44100.0;

  // YIN: tauMax <= tauMin
  const veryShort = [0.1, -0.1, 0.1];
  assert.equal(yinPitch(veryShort, sr, 100.0, 200.0), 0.0);

  // YIN: unpitched white noise where minVal > 0.85
  const noise = new Float64Array(1024);
  let seed = 12345;
  for (let i = 0; i < 1024; i++) {
    seed = (seed * 1103515245 + 12345) & 0x7fffffff;
    noise[i] = (seed / 0x7fffffff) * 2.0 - 1.0;
  }
  const noisePitch = yinPitch(noise, sr, 40.0, 2000.0, 0.01);
  // May be 0.0 or valid pitch; should not throw
  assert.ok(noisePitch >= 0.0);

  // HPS: kMax <= kMin
  const sig = new Float64Array(512).fill(0.5);
  assert.equal(hpsPitch(sig, sr, 64, 600.0, 2000.0, 256), 0.0);

  // Autocorrelation: tauMax <= tauMin
  assert.equal(autocorrelationPitch(veryShort, sr, 100.0, 200.0), 0.0);

  // Autocorrelation: silence / near-zero energy
  const quiet = new Float64Array(1024).fill(1e-6);
  assert.equal(autocorrelationPitch(quiet, sr), 0.0);

  // Autocorrelation: zero energy
  assert.equal(autocorrelationPitch(new Float64Array(1000), sr), 0.0);

  // Autocorrelation: linear ramp without local peaks
  const ramp = new Float64Array(2000);
  for (let i = 0; i < 2000; i++) ramp[i] = i / 2000;
  assert.ok(autocorrelationPitch(ramp, sr) >= 0.0);

  // Autocorrelation: unpitched noise returning 0.0
  const noiseAuto = autocorrelationPitch(noise, sr, 40.0, 2000.0);
  assert.ok(noiseAuto >= 0.0);

  // YIN tracking when signal length < frameLength
  const shortPitchTrack = yinPitchTrack(new Float64Array(100).fill(0.5), sr, 512, 256);
  assert.equal(shortPitchTrack.length, 1);
});

test("Coverage: onset and tempo edge cases", () => {
  const sr = 44100.0;

  // Signal shorter than n_fft in detectOnsetsSamples
  const shortSig = new Float64Array(200).fill(0.5);
  assert.deepEqual(detectOnsetsSamples(shortSig, sr, 512, 1024), [0]);

  // estimateTempo: minBpm < MIN_BPM
  assert.throws(
    () => estimateTempo([0.5, 0.5], sr, 0.5, 200.0),
    InvalidInputError,
  );

  // estimateTempo: minBpm >= maxBpm
  assert.throws(
    () => estimateTempo([0.5, 0.5], sr, 200.0, 100.0),
    InvalidInputError,
  );

  // estimateTempo: single onset fallback to default
  const singleOnset = new Float64Array(1000);
  singleOnset[10] = 1.0;
  const tempoFallback = estimateTempo(singleOnset, sr, 60.0, 200.0);
  assert.ok(tempoFallback >= 60.0 && tempoFallback <= 200.0);

  // estimateTempo: octave fold up
  const slowSig = new Float64Array(88200 * 2);
  slowSig[10] = 1.0;
  slowSig[88200] = 1.0;
  const slowBpm = estimateTempo(slowSig, sr, 50.0, 200.0);
  assert.ok(slowBpm >= 50.0 && slowBpm <= 200.0);

  // estimateTempo: octave fold down
  const fastSig = new Float64Array(44100);
  for (let i = 0; i < 15; i++) {
    fastSig[i * 2205] = 1.0;
  }
  const fastBpm = estimateTempo(fastSig, sr, 40.0, 100.0);
  assert.ok(fastBpm >= 40.0 && fastBpm <= 100.0);

  // detectOnsetsSamples with very short signal
  assert.deepEqual(detectOnsetsSamples(new Float64Array(5).fill(1.0), sr), [0]);
});

test("Coverage: harmonic extraction edge cases", () => {
  const sr = 44100.0;

  // Signal longer than targetFft (4096)
  const longSig = new Float64Array(5000);
  const phaseInc = (2.0 * Math.PI * 440.0) / sr;
  for (let i = 0; i < 5000; i++) {
    longSig[i] = 0.5 * Math.sin(i * phaseInc);
  }
  const profLong = extractHarmonicProfile(
    longSig,
    sr,
    440.0,
    8,
    "long_synth",
    "Long Synth",
  );
  assert.equal(profLong.id, "long_synth");
  assert.equal(profLong.harmonicProfile.length, 8);

  // High maxPartials hitting Nyquist
  const profNyquist = extractHarmonicProfile(
    longSig,
    sr,
    10000.0,
    10,
    "high_synth",
    "High Synth",
  );
  assert.ok(profNyquist.harmonicProfile.length < 10);

  // extractHarmonicProfileSimple
  const profSimple = extractHarmonicProfileSimple(longSig, sr);
  assert.equal(profSimple.id, "extracted_instrument");

  // Signal shorter than targetFft (exercises nextPowerOfTwo and padding)
  const medSig = new Float64Array(1000);
  for (let i = 0; i < 1000; i++) {
    medSig[i] = 0.5 * Math.sin(i * phaseInc);
  }
  const profMed = extractHarmonicProfile(medSig, sr, 440.0, 8);
  assert.equal(profMed.harmonicProfile.length, 8);

  // Signal shorter than frameLen (256) (exercises rmsTrack = [overallRms] and sustainLevel = 0.7 fallback)
  const tinySig = new Float64Array(200);
  for (let i = 0; i < 200; i++) {
    tinySig[i] = 0.5 * Math.sin(i * phaseInc);
  }
  const profTiny = extractHarmonicProfile(tinySig, sr, 440.0, 4);
  assert.equal(profTiny.harmonicProfile.length, 4);

  // High fundamental frequency exceeding Nyquist
  const profHigh = extractHarmonicProfile(medSig, sr, 22000.0, 4);
  assert.equal(profHigh.harmonicProfile.length, 1);

  // Failure to detect fundamental pitch (unpitched noise with non-silent energy)
  const noiseSignal = new Float64Array(1024);
  for (let i = 0; i < 1024; i++) {
    noiseSignal[i] = i % 2 === 0 ? 0.5 : -0.5;
  }
  assert.throws(
    () => extractHarmonicProfile(noiseSignal, sr, null),
    InvalidInputError,
  );
});

test("Coverage: transcription edge cases", () => {
  const sr = 44100.0;

  // Integer PCM samples normalized
  const intPcm = new Float64Array(2000);
  const phaseInc = (2.0 * Math.PI * 440.0) / sr;
  for (let i = 0; i < 2000; i++) {
    intPcm[i] = 10000.0 * Math.sin(i * phaseInc);
  }
  const scoreInt = transcribeSamples(intPcm, sr);
  assert.equal(scoreInt.events.length, 1);

  // Signal with short segment skipped (< 128 samples)
  const shortSegment = new Float64Array(100).fill(0.5);
  const scoreShort = transcribeSamples(shortSegment, sr);
  assert.equal(scoreShort.events.length, 0);

  // Transcribe with custom trackId and instrumentId
  const customScore = transcribeSamples(intPcm, sr, {
    title: "My Custom Score",
    instrumentId: "flute_naive",
    trackId: "solo",
  });
  assert.equal(customScore.title, "My Custom Score");
  assert.equal(customScore.tracks[0].id, "solo");
  assert.equal(customScore.tracks[0].instrumentId, "flute_naive");

  // Transcribe via PCMBuffer
  const pcmFmt = new PCMFormat(sr);
  const rawPcm = new Array(1000).fill(5000);
  const pcmBuf = new PCMBuffer(rawPcm, pcmFmt);
  const bufScore = transcribePcmBuffer(pcmBuf);
  assert.equal(bufScore.sampleRateHz, 44100);
});
