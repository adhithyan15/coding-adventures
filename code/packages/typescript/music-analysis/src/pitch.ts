/**
 * Iterative pitch detection: YIN, Harmonic Product Spectrum (HPS), and Autocorrelation (MUS08, VER15).
 */

import {
  buildHannWindow,
  checkedFrameBounds,
  parabolicInterpolation,
  rfftMagnitudes,
  validateFraming,
} from "./dsp_utils.js";
import { rmsEnergy } from "./features.js";
import {
  BudgetExceededError,
  InvalidInputError,
  MAX_FFT_LEN,
  MAX_PARTIALS,
  MAX_SAMPLE_RATE_HZ,
  validatePositiveFloat,
  validatePositiveInt,
  validateSamples,
} from "./types.js";

function nextPowerOfTwo(n: number): number {
  if (n <= 1) return 1;
  let p = 1;
  while (p < n && p < 1073741824) {
    p <<= 1;
  }
  return p;
}

/**
 * Estimate fundamental pitch using strictly iterative YIN algorithm.
 *
 * @param samples Time-domain audio sample sequence.
 * @param sampleRateHz Sampling frequency in Hertz.
 * @param fMin Minimum expected fundamental frequency in Hertz (default 40.0 Hz).
 * @param fMax Maximum expected fundamental frequency in Hertz (default 2000.0 Hz).
 * @param threshold YIN absolute threshold for cumulative normalized difference (default 0.15).
 * @returns Estimated fundamental frequency in Hertz, or 0.0 if unpitched/silent.
 */
export function yinPitch(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  fMin: number = 40.0,
  fMax: number = 2000.0,
  threshold: number = 0.15,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const validFmin = validatePositiveFloat(fMin, "f_min", validSr / 2.0);
  const validFmax = validatePositiveFloat(fMax, "f_max", validSr / 2.0);
  if (validFmin >= validFmax) {
    throw new InvalidInputError(
      `f_min (${validFmin}) must be < f_max (${validFmax})`,
    );
  }
  const validThresh = validatePositiveFloat(threshold, "threshold", 1.0);
  const s = validateSamples(samples, "samples");
  const n = s.length;

  // Silence check
  if (rmsEnergy(s) < 1e-4) {
    return 0.0;
  }

  const tauMin = Math.max(2, Math.floor(validSr / validFmax));
  const tauMax = Math.min(Math.floor(n / 2), Math.ceil(validSr / validFmin));

  if (tauMax <= tauMin) {
    return 0.0;
  }

  const wSize = n - tauMax;

  // Step 1: Difference function d(tau)
  const d = new Float64Array(tauMax + 1);
  for (let tau = 1; tau <= tauMax; tau++) {
    let diffSum = 0.0;
    for (let j = 0; j < wSize; j++) {
      const diff = s[j] - s[j + tau];
      diffSum += diff * diff;
    }
    d[tau] = diffSum;
  }

  // Step 2: Cumulative mean normalized difference d'(tau)
  const dPrime = new Float64Array(tauMax + 1);
  dPrime[0] = 1.0;
  let runningSum = 0.0;
  for (let tau = 1; tau <= tauMax; tau++) {
    runningSum += d[tau];
    if (runningSum > 0.0) {
      dPrime[tau] = d[tau] / (runningSum / tau);
    } else {
      dPrime[tau] = 1.0;
    }
  }

  // Step 3: Absolute threshold search
  let bestTau: number | null = null;
  for (let tau = tauMin; tau <= tauMax; tau++) {
    if (dPrime[tau] < validThresh) {
      let t = tau;
      while (t < tauMax && dPrime[t + 1] < dPrime[t]) {
        t += 1;
      }
      bestTau = t;
      break;
    }
  }

  let chosenTau: number;
  if (bestTau !== null) {
    chosenTau = bestTau;
  } else {
    let minVal = Infinity;
    let minTau = tauMin;
    for (let tau = tauMin; tau <= tauMax; tau++) {
      const val = dPrime[tau];
      if (val < minVal) {
        minVal = val;
        minTau = tau;
      }
    }
    if (minVal > 0.85) {
      return 0.0;
    }
    chosenTau = minTau;
  }

  // Step 4: Parabolic interpolation
  const refinedTau =
    tauMin < chosenTau && chosenTau < tauMax
      ? parabolicInterpolation(
          dPrime[chosenTau - 1],
          dPrime[chosenTau],
          dPrime[chosenTau + 1],
          chosenTau,
        )
      : chosenTau;

  if (refinedTau <= 0.0) {
    return 0.0;
  }

  const pitch = validSr / refinedTau;
  if (validFmin <= pitch && pitch <= validFmax) {
    return pitch;
  }
  return 0.0;
}

/**
 * Convenience wrapper for `yinPitch` with standard defaults (40 Hz to 2000 Hz, threshold 0.15).
 */
export function yinPitchSimple(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
): number {
  return yinPitch(samples, sampleRateHz, 40.0, 2000.0, 0.15);
}

/**
 * Track fundamental pitch over time using windowed YIN analysis.
 */
export function yinPitchTrack(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  frameLength: number,
  hopLength: number,
  fMin: number = 40.0,
  fMax: number = 2000.0,
  threshold: number = 0.15,
): number[] {
  const s = validateSamples(samples, "samples");
  const maybeFrames = validateFraming(s.length, frameLength, hopLength);
  if (maybeFrames === null) {
    const p = yinPitch(s, sampleRateHz, fMin, fMax, threshold);
    return [p];
  }
  const numFrames = maybeFrames;

  const pitches: number[] = [];
  for (let m = 0; m < numFrames; m++) {
    const [start, end] = checkedFrameBounds(
      m,
      hopLength,
      frameLength,
      s.length,
    );
    const frame = s.subarray(start, end);
    const p = yinPitch(frame, sampleRateHz, fMin, fMax, threshold);
    pitches.push(p);
  }

  return pitches;
}

/**
 * Estimate fundamental pitch via Harmonic Product Spectrum (HPS).
 *
 * @param samples Time-domain audio sample sequence.
 * @param sampleRateHz Sampling frequency in Hertz.
 * @param numHarmonics Number of harmonic spectra to compress (default 5, up to 64).
 * @param fMin Minimum frequency search bound.
 * @param fMax Maximum frequency search bound.
 * @param nFft Optional FFT length (defaults to power of two >= len(samples), capped at 8192).
 */
export function hpsPitch(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  numHarmonics: number = 5,
  fMin: number = 40.0,
  fMax: number = 2000.0,
  nFft?: number,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const nHarmonics = validatePositiveInt(
    numHarmonics,
    "num_harmonics",
    MAX_PARTIALS,
  );
  const validFmin = validatePositiveFloat(fMin, "f_min", validSr / 2.0);
  const validFmax = validatePositiveFloat(fMax, "f_max", validSr / 2.0);
  if (validFmin >= validFmax) {
    throw new InvalidInputError("f_min must be < f_max");
  }
  const s = validateSamples(samples, "samples");
  const n = s.length;

  if (rmsEnergy(s) < 1e-4) {
    return 0.0;
  }

  let fftLen: number;
  if (nFft === undefined || nFft === null) {
    const p2 = n <= 1 ? 1 : nextPowerOfTwo(n);
    fftLen = Math.min(8192, Math.max(1024, p2));
  } else {
    fftLen = validatePositiveInt(nFft, "n_fft", MAX_FFT_LEN);
  }

  const takeLen = Math.min(n, fftLen);
  const win = buildHannWindow(takeLen);
  const windowed = new Float64Array(fftLen);
  for (let i = 0; i < takeLen; i++) {
    windowed[i] = s[i] * win[i];
  }

  const mags = rfftMagnitudes(windowed);
  const kBins = mags.length;

  const freqStep = validSr / fftLen;
  const kMin = Math.max(1, Math.floor(validFmin / freqStep));
  const kMax = Math.min(
    Math.floor(kBins / nHarmonics),
    Math.ceil(validFmax / freqStep),
  );

  if (kMax <= kMin) {
    return 0.0;
  }

  let bestK = kMin;
  let maxHps = -Infinity;
  const hpsScores = new Float64Array(kMax + 2);

  for (let k = kMin; k <= kMax; k++) {
    let logProd = 0.0;
    for (let h = 1; h <= nHarmonics; h++) {
      const kh = k * h;
      if (!Number.isSafeInteger(kh)) {
        throw new BudgetExceededError("HPS harmonic bin index overflow");
      }
      if (kh < kBins) {
        logProd += Math.log(mags[kh] + 1e-9);
      }
    }
    hpsScores[k] = logProd;
    if (logProd > maxHps) {
      maxHps = logProd;
      bestK = k;
    }
  }

  // Parabolic interpolation
  const refinedK =
    kMin < bestK && bestK < kMax
      ? parabolicInterpolation(
          hpsScores[bestK - 1],
          maxHps,
          hpsScores[bestK + 1],
          bestK,
        )
      : bestK;

  const pitch = refinedK * freqStep;
  if (validFmin <= pitch && pitch <= validFmax) {
    return pitch;
  }
  return 0.0;
}

/**
 * Convenience wrapper for `hpsPitch` with standard defaults (5 harmonics, 40 Hz to 2000 Hz).
 */
export function hpsPitchSimple(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
): number {
  return hpsPitch(samples, sampleRateHz, 5, 40.0, 2000.0, undefined);
}

/**
 * Estimate fundamental pitch via normalized autocorrelation with parabolic refinement.
 *
 * @param samples Time-domain audio sample sequence.
 * @param sampleRateHz Sampling frequency in Hertz.
 * @param fMin Minimum frequency search bound (default 40.0 Hz).
 * @param fMax Maximum frequency search bound (default 2000.0 Hz).
 * @returns Estimated fundamental frequency in Hertz, or 0.0 if unpitched/silent.
 */
export function autocorrelationPitch(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  fMin: number = 40.0,
  fMax: number = 2000.0,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const validFmin = validatePositiveFloat(fMin, "f_min", validSr / 2.0);
  const validFmax = validatePositiveFloat(fMax, "f_max", validSr / 2.0);
  if (validFmin >= validFmax) {
    throw new InvalidInputError(
      `f_min (${validFmin}) must be < f_max (${validFmax})`,
    );
  }
  const s = validateSamples(samples, "samples");
  const n = s.length;

  if (rmsEnergy(s) < 1e-4) {
    return 0.0;
  }

  const tauMin = Math.max(2, Math.floor(validSr / validFmax));
  const tauMax = Math.min(Math.floor(n / 2), Math.ceil(validSr / validFmin));

  if (tauMax <= tauMin) {
    return 0.0;
  }

  const wSize = n - tauMax;
  let energy0 = 0.0;
  for (let j = 0; j < wSize; j++) {
    const v = s[j];
    energy0 += v * v;
  }
  if (energy0 <= 0.0) {
    return 0.0;
  }

  const r = new Float64Array(tauMax + 2);
  let maxCorr = -Infinity;

  for (let tau = tauMin; tau <= tauMax; tau++) {
    let cross = 0.0;
    let energyTau = 0.0;
    for (let j = 0; j < wSize; j++) {
      const v0 = s[j];
      const vt = s[j + tau];
      cross += v0 * vt;
      energyTau += vt * vt;
    }
    const denom = Math.sqrt(energy0 * energyTau);
    const norm = denom > 1e-12 ? cross / denom : 0.0;
    r[tau] = norm;
    if (norm > maxCorr) {
      maxCorr = norm;
    }
  }

  if (maxCorr < 0.3) {
    return 0.0;
  }

  // Select the FIRST local peak exceeding 0.7 * maxCorr to avoid subharmonic / octave error
  const peakThresh = Math.max(0.3, 0.7 * maxCorr);
  let bestTau = tauMin;
  let foundPeak = false;

  for (let tau = tauMin + 1; tau < tauMax; tau++) {
    if (r[tau] > r[tau - 1] && r[tau] >= r[tau + 1] && r[tau] >= peakThresh) {
      bestTau = tau;
      foundPeak = true;
      break;
    }
  }

  if (!foundPeak) {
    let peakVal = -Infinity;
    for (let tau = tauMin; tau <= tauMax; tau++) {
      if (r[tau] > peakVal) {
        peakVal = r[tau];
        bestTau = tau;
      }
    }
  }

  const refinedTau =
    tauMin < bestTau && bestTau < tauMax
      ? parabolicInterpolation(r[bestTau - 1], maxCorr, r[bestTau + 1], bestTau)
      : bestTau;

  if (refinedTau <= 0.0) {
    return 0.0;
  }

  const pitch = validSr / refinedTau;
  if (validFmin <= pitch && pitch <= validFmax) {
    return pitch;
  }
  return 0.0;
}

// Snake-case aliases
export const yin_pitch = yinPitch;
export const yin_pitch_simple = yinPitchSimple;
export const yin_pitch_track = yinPitchTrack;
export const hps_pitch = hpsPitch;
export const hps_pitch_simple = hpsPitchSimple;
export const autocorrelation_pitch = autocorrelationPitch;
