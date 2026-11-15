import assert from "node:assert/strict";
import test from "node:test";

import {
  autocorrelationPitch,
  BudgetExceededError,
  detectOnsets,
  extractHarmonicProfile,
  frequencyToNote,
  frequencyToNoteA440,
  hpsPitch,
  InvalidInputError,
  MAX_BPM,
  MAX_FFT_LEN,
  MAX_FRAMES,
  MAX_PARTIALS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  MIN_BPM,
  MIN_SAMPLE_RATE_HZ,
  rmsEnergy,
  rmsEnergyFramed,
  spectralCentroid,
  spectralFlux,
  spectralFluxSequence,
  spectralRolloff,
  transcribeSamples,
  validatePcmSourceBounds,
  validateSamples,
  validateSpectrum,
  yinPitch,
  yinPitchTrack,
  zeroCrossingRate,
  zeroCrossingRateFramed,
} from "../src/index.js";

test("CWE-400: MAX_FFT_LEN bounds enforcement", () => {
  assert.throws(
    () => hpsPitch([1.0], 44100.0, 5, 40.0, 2000.0, MAX_FFT_LEN + 1),
    BudgetExceededError,
  );
  assert.throws(
    () => detectOnsets([1.0], 44100.0, 512, MAX_FFT_LEN + 1, 1.5, 0.08),
    BudgetExceededError,
  );
});

test("CWE-400: MAX_FRAMES bounds enforcement prior to allocation", () => {
  const dummyFrames = new Array(MAX_FRAMES + 1).fill([1.0, 2.0]);
  assert.throws(
    () => spectralFluxSequence(dummyFrames, true),
    BudgetExceededError,
  );

  const sig = new Float64Array(MAX_FRAMES + 2).fill(0.5);
  assert.throws(
    () => zeroCrossingRateFramed(sig, 1, 1),
    BudgetExceededError,
  );
  assert.throws(
    () => rmsEnergyFramed(sig, 1, 1),
    BudgetExceededError,
  );
  assert.throws(
    () => yinPitchTrack(sig, 44100.0, 1, 1, 40.0, 2000.0, 0.15),
    BudgetExceededError,
  );

  const sigOnsets = new Float64Array(MAX_FRAMES + 1026).fill(0.5);
  assert.throws(
    () => detectOnsets(sigOnsets, 44100.0, 1, 1024, 1.5, 0.08),
    BudgetExceededError,
  );
});

test("CWE-400: MAX_PARTIALS bounds enforcement", () => {
  const sig = new Float64Array(500).fill(1.0);
  assert.throws(
    () => extractHarmonicProfile(sig, 44100.0, null, MAX_PARTIALS + 1, "id", "label"),
    BudgetExceededError,
  );
  assert.throws(
    () => hpsPitch(sig, 44100.0, MAX_PARTIALS + 1, 40.0, 2000.0),
    BudgetExceededError,
  );
});

test("CWE-400: MAX_BPM bounds enforcement", () => {
  assert.throws(
    () => transcribeSamples([1.0], 44100.0, { bpm: MAX_BPM + 1.0 }),
    BudgetExceededError,
  );
});

test("CWE-400: MAX_SAMPLE_RATE_HZ bounds enforcement", () => {
  assert.throws(
    () => frequencyToNote(440.0, MAX_SAMPLE_RATE_HZ + 10.0),
    BudgetExceededError,
  );
  assert.throws(
    () => frequencyToNote(MAX_SAMPLE_RATE_HZ / 2.0 + 1.0, 440.0),
    BudgetExceededError,
  );
  assert.throws(
    () => spectralCentroid([1.0, 2.0], MAX_SAMPLE_RATE_HZ + 1.0),
    BudgetExceededError,
  );
  assert.throws(
    () => yinPitch([1.0], MAX_SAMPLE_RATE_HZ + 1.0, 40.0, 2000.0, 0.15),
    BudgetExceededError,
  );
  assert.throws(
    () => hpsPitch([1.0], MAX_SAMPLE_RATE_HZ + 1.0, 5, 40.0, 2000.0),
    BudgetExceededError,
  );
  assert.throws(
    () => autocorrelationPitch([1.0], MAX_SAMPLE_RATE_HZ + 1.0, 40.0, 2000.0),
    BudgetExceededError,
  );
  assert.throws(
    () => detectOnsets([1.0], MAX_SAMPLE_RATE_HZ + 1.0, 512, 1024, 1.5, 0.08),
    BudgetExceededError,
  );
  assert.throws(
    () => extractHarmonicProfile([1.0], MAX_SAMPLE_RATE_HZ + 1.0, null, 16, "id", "label"),
    BudgetExceededError,
  );
  assert.throws(
    () => transcribeSamples([1.0], MAX_SAMPLE_RATE_HZ + 1.0),
    BudgetExceededError,
  );
});

test("CWE-20: Fail-closed NaN rejection across all functions and options", () => {
  const nan = NaN;
  assert.throws(() => frequencyToNoteA440(nan), InvalidInputError);
  assert.throws(() => frequencyToNote(440.0, nan), InvalidInputError);
  assert.throws(() => spectralCentroid([nan, 1.0], 44100.0), InvalidInputError);
  assert.throws(() => spectralFlux([nan], [1.0], true), InvalidInputError);
  assert.throws(() => spectralRolloff([nan, 1.0], 44100.0, 0.85), InvalidInputError);
  assert.throws(() => spectralRolloff([1.0, 1.0], 44100.0, nan), InvalidInputError);
  assert.throws(() => zeroCrossingRate([1.0, nan, 2.0]), InvalidInputError);
  assert.throws(() => rmsEnergy([1.0, nan, 2.0]), InvalidInputError);
  assert.throws(() => yinPitch([nan], 44100.0, 40.0, 2000.0, 0.15), InvalidInputError);
  assert.throws(() => yinPitch([1.0], 44100.0, nan, 2000.0, 0.15), InvalidInputError);
  assert.throws(() => yinPitch([1.0], 44100.0, 40.0, nan, 0.15), InvalidInputError);
  assert.throws(() => yinPitch([1.0], 44100.0, 40.0, 2000.0, nan), InvalidInputError);
  assert.throws(() => hpsPitch([nan], 44100.0, 5, 40.0, 2000.0), InvalidInputError);
  assert.throws(() => hpsPitch([1.0], 44100.0, nan as unknown as number, 40.0, 2000.0), InvalidInputError);
  assert.throws(() => autocorrelationPitch([nan], 44100.0), InvalidInputError);
  assert.throws(() => detectOnsets([nan], 44100.0, 512, 1024, 1.5, 0.08), InvalidInputError);
  assert.throws(() => detectOnsets([1.0], 44100.0, nan as unknown as number, 1024, 1.5, 0.08), InvalidInputError);
  assert.throws(() => detectOnsets([1.0], 44100.0, 512, nan as unknown as number, 1.5, 0.08), InvalidInputError);
  assert.throws(() => detectOnsets([1.0], 44100.0, 512, 1024, nan, 0.08), InvalidInputError);
  assert.throws(() => detectOnsets([1.0], 44100.0, 512, 1024, 1.5, nan), InvalidInputError);
  assert.throws(() => extractHarmonicProfile([nan], 44100.0), InvalidInputError);
  assert.throws(() => extractHarmonicProfile([1.0], 44100.0, nan), InvalidInputError);
  assert.throws(() => extractHarmonicProfile([1.0], 44100.0, null, nan as unknown as number), InvalidInputError);
  assert.throws(() => transcribeSamples([nan], 44100.0), InvalidInputError);
  assert.throws(() => transcribeSamples([1.0], 44100.0, { bpm: nan }), InvalidInputError);
  assert.throws(() => transcribeSamples([1.0], 44100.0, { ppq: nan as unknown as number }), InvalidInputError);
  assert.throws(() => transcribeSamples([1.0], 44100.0, { quantizeGridBeats: nan }), InvalidInputError);
});

test("CWE-20: Fail-closed Infinity rejection across all functions", () => {
  for (const bad of [Infinity, -Infinity]) {
    assert.throws(() => frequencyToNoteA440(bad), InvalidInputError);
    assert.throws(() => spectralCentroid([1.0, bad], 44100.0), InvalidInputError);
    assert.throws(() => zeroCrossingRate([1.0, bad]), InvalidInputError);
    assert.throws(() => rmsEnergy([1.0, bad]), InvalidInputError);
    assert.throws(() => yinPitch([1.0, bad], 44100.0, 40.0, 2000.0, 0.15), InvalidInputError);
    assert.throws(() => hpsPitch([1.0, bad], 44100.0, 5, 40.0, 2000.0), InvalidInputError);
    assert.throws(() => autocorrelationPitch([1.0, bad], 44100.0), InvalidInputError);
  }
});

test("Domain parameter validation and fail-closed security", () => {
  // hopLength = 0
  assert.throws(() => zeroCrossingRateFramed([0.5, 0.5], 2, 0), InvalidInputError);
  assert.throws(() => rmsEnergyFramed([0.5, 0.5], 2, 0), InvalidInputError);

  // frameLength = 0
  assert.throws(() => zeroCrossingRateFramed([0.5, 0.5], 0, 1), InvalidInputError);
  assert.throws(() => rmsEnergyFramed([0.5, 0.5], 0, 1), InvalidInputError);

  // hpsPitch numHarmonics = 0
  assert.throws(() => hpsPitch([0.5, 0.5], 44100.0, 0, 40.0, 2000.0), InvalidInputError);

  // ppq out of range
  assert.throws(
    () => transcribeSamples([0.5], 44100.0, { ppq: 0 }),
    InvalidInputError,
  );
  assert.throws(
    () => transcribeSamples([0.5], 44100.0, { ppq: 9601 }),
    BudgetExceededError,
  );

  // quantizeGridBeats <= 0 or > 16
  assert.throws(
    () => transcribeSamples([0.5], 44100.0, { quantizeGridBeats: 0.0 }),
    InvalidInputError,
  );
  assert.throws(
    () => transcribeSamples([0.5], 44100.0, { quantizeGridBeats: 17.0 }),
    BudgetExceededError,
  );

  // bpm below MIN_BPM
  assert.throws(
    () => transcribeSamples([0.5], 44100.0, { bpm: MIN_BPM - 0.1 }),
    InvalidInputError,
  );

  // sampleRate below MIN_SAMPLE_RATE_HZ
  assert.throws(
    () => transcribeSamples([0.5], MIN_SAMPLE_RATE_HZ - 1.0),
    InvalidInputError,
  );
});

test("Hostile zeroCrossingRateFramed frameLength=1 with extreme float values never produces NaN", () => {
  const hostileVals = [0.0, -0.0, 1e-300, -1e-300, 1e300, -1e300, 0.5, -0.5];
  const res = zeroCrossingRateFramed(hostileVals, 1, 1);
  assert.equal(res.length, hostileVals.length);
  for (let i = 0; i < res.length; i++) {
    assert.ok(!Number.isNaN(res[i]), `res[${i}] must not be NaN`);
    assert.equal(res[i], 0.0, `res[${i}] must be 0.0`);
  }
});

test("Hostile CWE-20: validatePcmSourceBounds strictly rejects NaN, negative, non-integer, and zero length", () => {
  assert.throws(() => validatePcmSourceBounds(NaN, 44100.0), InvalidInputError);
  assert.throws(() => validatePcmSourceBounds(-1, 44100.0), InvalidInputError);
  assert.throws(() => validatePcmSourceBounds(1.5, 44100.0), InvalidInputError);
  assert.throws(() => validatePcmSourceBounds(0, 44100.0), InvalidInputError);
  assert.throws(() => validatePcmSourceBounds(MAX_SAMPLES + 1, 44100.0), BudgetExceededError);
});

test("Hostile CWE-400: validateSamples and validateSpectrum reject oversized ArrayLike before allocation", () => {
  // ArrayLike with length > MAX
  assert.throws(
    () => validateSamples({ length: MAX_SAMPLES + 1 } as any, "samples"),
    BudgetExceededError,
  );
  assert.throws(
    () => validateSpectrum({ length: MAX_FFT_LEN + 1 } as any, "spectrum"),
    BudgetExceededError,
  );

  // ArrayLike with invalid length
  assert.throws(() => validateSamples({ length: NaN } as any, "samples"), InvalidInputError);
  assert.throws(() => validateSamples({ length: -5 } as any, "samples"), InvalidInputError);
  assert.throws(() => validateSamples({ length: 3.14 } as any, "samples"), InvalidInputError);
  assert.throws(() => validateSpectrum({ length: NaN } as any, "spectrum"), InvalidInputError);
  assert.throws(() => validateSpectrum({ length: -1 } as any, "spectrum"), InvalidInputError);
});

test("Hostile CWE-400: validateSamples and validateSpectrum abort oversized arbitrary Iterables early", () => {
  function* makeOversizedIterable(count: number) {
    for (let i = 0; i < count; i++) {
      yield 0.25;
    }
  }

  // Oversized iterable without .length
  assert.throws(
    () => validateSamples(makeOversizedIterable(MAX_SAMPLES + 1), "samples"),
    BudgetExceededError,
  );
  assert.throws(
    () => validateSpectrum(makeOversizedIterable(MAX_FFT_LEN + 1), "spectrum"),
    BudgetExceededError,
  );

  // Valid small iterable without .length succeeds
  const smallSamples = validateSamples(makeOversizedIterable(4), "samples");
  assert.equal(smallSamples.length, 4);
  assert.equal(smallSamples[0], 0.25);

  const smallSpectrum = validateSpectrum(makeOversizedIterable(8), "spectrum");
  assert.equal(smallSpectrum.length, 8);
  assert.equal(smallSpectrum[0], 0.25);
});
