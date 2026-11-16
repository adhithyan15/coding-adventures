/**
 * Onset detection via spectral flux and wavelet detail energy, and tempo estimation (MUS08, VER15).
 */

import { stft, WindowType } from "@coding-adventures/dsp-stft";
import { dwt, WaveletBoundary } from "@coding-adventures/dsp-wavelets";

import { computeSpectralFluxStep } from "./dsp_utils.js";
import { rmsEnergy } from "./features.js";
import {
  BudgetExceededError,
  InvalidInputError,
  MAX_BPM,
  MAX_FFT_LEN,
  MAX_FRAMES,
  MAX_ONSETS,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  MIN_BPM,
  validatePositiveFloat,
  validatePositiveInt,
  validateSamples,
} from "./types.js";

/**
 * Detect musical note onset sample indices combining STFT flux and DWT energy.
 */
export function detectOnsetsSamples(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  hopLength: number = 512,
  nFft: number = 1024,
  thresholdFactor: number = 1.5,
  minDistanceSeconds: number = 0.08,
): number[] {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const sig = validateSamples(samples, "samples");
  const n = sig.length;

  const hop = validatePositiveInt(hopLength, "hop_length", MAX_SAMPLES);
  const fftLen = validatePositiveInt(nFft, "n_fft", MAX_FFT_LEN);
  const tFactor = validatePositiveFloat(
    thresholdFactor,
    "threshold_factor",
    10.0,
  );
  const minDistSec = validatePositiveFloat(
    minDistanceSeconds,
    "min_distance_seconds",
    10.0,
  );

  if (rmsEnergy(sig) < 1e-4) {
    return [];
  }

  // If clip is shorter than one FFT window, return single onset at 0 if active
  if (n < fftLen) {
    return [0];
  }

  // Pre-flight frame budget enforcement BEFORE calling stft/allocating
  const expectedFrames = 1 + Math.floor((n - fftLen) / hop);
  if (expectedFrames > MAX_FRAMES) {
    throw new BudgetExceededError(
      `frame count ${expectedFrames} exceeds MAX_FRAMES=${MAX_FRAMES}`,
    );
  }

  // --- 1. Spectral Flux Novelty Curve ---
  const stftMatrix = stft(sig, fftLen, hop, WindowType.HANN);
  const bins = Math.floor(fftLen / 2) + 1;
  const frameFloats = bins * 2;
  const numFrames = Math.floor(stftMatrix.length / frameFloats);

  if (numFrames <= 1) {
    return [0];
  }

  const mags: Float64Array[] = [];
  for (let m = 0; m < numFrames; m++) {
    const offset = m * frameFloats;
    const frameMag = new Float64Array(bins);
    for (let k = 0; k < bins; k++) {
      const re = stftMatrix[offset + 2 * k];
      const im = stftMatrix[offset + 2 * k + 1];
      frameMag[k] = Math.sqrt(re * re + im * im);
    }
    mags.push(frameMag);
  }

  const sf = new Float64Array(numFrames);
  for (let m = 1; m < numFrames; m++) {
    sf[m] = computeSpectralFluxStep(mags[m - 1], mags[m], true);
  }

  let maxSf = 0.0;
  for (let m = 0; m < numFrames; m++) {
    if (sf[m] > maxSf) {
      maxSf = sf[m];
    }
  }
  if (maxSf > 0.0) {
    for (let m = 0; m < numFrames; m++) {
      sf[m] /= maxSf;
    }
  }

  // --- 2. Wavelet Detail Energy Difference Curve ---
  let cd: Float64Array | null = null;
  try {
    const [, detail] = dwt(sig, "haar", WaveletBoundary.SYMMETRIC);
    cd = detail;
  } catch {
    cd = null;
  }

  const weDiff = new Float64Array(numFrames);
  if (cd && cd.length > 0) {
    const dHop = Math.max(1, Math.floor(hop / 2));
    const nCd = cd.length;
    const rawWe = new Float64Array(numFrames);
    for (let m = 0; m < numFrames; m++) {
      const cStart = m * dHop;
      const cEnd = Math.min(nCd, cStart + dHop);
      if (cStart < nCd) {
        let eSum = 0.0;
        for (let idx = cStart; idx < cEnd; idx++) {
          const val = cd[idx];
          eSum += val * val;
        }
        rawWe[m] = eSum;
      }
    }

    for (let m = 1; m < numFrames; m++) {
      const diff = rawWe[m] - rawWe[m - 1];
      if (diff > 0.0) {
        weDiff[m] = diff;
      }
    }

    let maxWe = 0.0;
    for (let m = 0; m < numFrames; m++) {
      if (weDiff[m] > maxWe) {
        maxWe = weDiff[m];
      }
    }
    if (maxWe > 0.0) {
      for (let m = 0; m < numFrames; m++) {
        weDiff[m] /= maxWe;
      }
    }
  }

  // --- 3. Combined Novelty Curve ---
  const novelty = new Float64Array(numFrames);
  for (let m = 0; m < numFrames; m++) {
    novelty[m] = 0.5 * sf[m] + 0.5 * weDiff[m];
  }

  // --- 4. Adaptive Peak Picking ---
  const windowHalf = 3; // 7-frame moving window
  const detectedFrames: number[] = [];

  // Check if frame 0 starts with immediate active energy
  const initialCheckLen = Math.min(n, fftLen);
  if (rmsEnergy(sig.subarray(0, initialCheckLen)) > 0.02) {
    detectedFrames.push(0);
  }

  const minDistFrames = Math.max(
    1,
    Math.round((minDistSec * validSr) / hop),
  );

  for (let m = 1; m < numFrames - 1; m++) {
    const val = novelty[m];
    // Must be local peak
    if (val <= novelty[m - 1] || val < novelty[m + 1]) {
      continue;
    }

    // Local window statistics
    const wStart = Math.max(0, m - windowHalf);
    const wEnd = Math.min(numFrames, m + windowHalf + 1);
    const wLen = wEnd - wStart;
    let sumVal = 0.0;
    for (let i = wStart; i < wEnd; i++) {
      sumVal += novelty[i];
    }
    const meanVal = sumVal / wLen;

    let varSum = 0.0;
    for (let i = wStart; i < wEnd; i++) {
      const diff = novelty[i] - meanVal;
      varSum += diff * diff;
    }
    const stdVal = Math.sqrt(varSum / wLen);

    const threshold = meanVal + tFactor * stdVal;
    const isFarEnough =
      detectedFrames.length === 0 ||
      m - detectedFrames[detectedFrames.length - 1] >= minDistFrames;

    if (val >= threshold && val > 0.15 && isFarEnough) {
      detectedFrames.push(m);
    }
  }

  if (detectedFrames.length > MAX_ONSETS) {
    throw new BudgetExceededError(
      `detected onsets ${detectedFrames.length} exceeds MAX_ONSETS=${MAX_ONSETS}`,
    );
  }

  if (detectedFrames.length === 0) {
    detectedFrames.push(0);
  }

  return detectedFrames.map((f) => f * hop);
}

/**
 * Detect musical note onset timestamps (in seconds) combining STFT flux and DWT energy.
 *
 * @param samples Time-domain audio sample buffer.
 * @param sampleRateHz Audio sampling frequency in Hertz.
 * @param hopLength Step size between analysis windows (default 512).
 * @param nFft Window size for Short-Time Fourier Transform (default 1024).
 * @param thresholdFactor Multiplier for adaptive standard-deviation threshold (default 1.5).
 * @param minDistanceSeconds Refractory lockout duration between consecutive onsets (default 0.08 s).
 * @returns List of detected onset timestamps in seconds.
 */
export function detectOnsets(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  hopLength: number = 512,
  nFft: number = 1024,
  thresholdFactor: number = 1.5,
  minDistanceSeconds: number = 0.08,
): number[] {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const sampleOnsets = detectOnsetsSamples(
    samples,
    validSr,
    hopLength,
    nFft,
    thresholdFactor,
    minDistanceSeconds,
  );
  return sampleOnsets.map((s) => s / validSr);
}

/**
 * Convenience wrapper for `detectOnsets` using standard defaults.
 */
export function detectOnsetsSimple(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
): number[] {
  return detectOnsets(samples, sampleRateHz, 512, 1024, 1.5, 0.08);
}

/**
 * Estimate musical tempo in Beats Per Minute (BPM) from audio sample onsets.
 *
 * @param samples Time-domain audio sample buffer.
 * @param sampleRateHz Audio sampling frequency in Hertz.
 * @param minBpm Minimum expected tempo (default 40.0 BPM).
 * @param maxBpm Maximum expected tempo (default 240.0 BPM).
 * @returns Estimated tempo in BPM, or default 120.0 if not enough events.
 */
export function estimateTempo(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  minBpm: number = 40.0,
  maxBpm: number = 240.0,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const validMinBpm = validatePositiveFloat(minBpm, "min_bpm", MAX_BPM);
  const validMaxBpm = validatePositiveFloat(maxBpm, "max_bpm", MAX_BPM);
  if (validMinBpm < MIN_BPM) {
    throw new InvalidInputError(`min_bpm ${validMinBpm} below minimum ${MIN_BPM}`);
  }
  if (validMinBpm >= validMaxBpm) {
    throw new InvalidInputError("min_bpm must be < max_bpm");
  }

  const onsets = detectOnsets(samples, validSr);
  if (onsets.length < 2) {
    return Math.min(validMaxBpm, Math.max(validMinBpm, 120.0));
  }

  // Inter-onset intervals (IOIs) in seconds
  const candidates: number[] = [];
  for (let i = 0; i < onsets.length - 1; i++) {
    const ioi = onsets[i + 1] - onsets[i];
    if (ioi >= 0.05 && ioi <= 4.0) {
      let bpm = 60.0 / ioi;
      // Map candidate into [validMinBpm, validMaxBpm] by octave folding
      while (bpm < validMinBpm && bpm > 0) {
        bpm *= 2.0;
      }
      while (bpm > validMaxBpm) {
        bpm /= 2.0;
      }
      if (bpm >= validMinBpm && bpm <= validMaxBpm) {
        candidates.push(bpm);
      }
    }
  }

  if (candidates.length === 0) {
    return Math.min(validMaxBpm, Math.max(validMinBpm, 120.0));
  }

  candidates.sort((a, b) => a - b);
  const medianBpm = candidates[Math.floor(candidates.length / 2)];
  return Math.round(medianBpm * 100.0) / 100.0;
}

// Snake-case aliases
export const detect_onsets = detectOnsets;
export const detect_onsets_samples = detectOnsetsSamples;
export const detect_onsets_simple = detectOnsetsSimple;
export const estimate_tempo = estimateTempo;
