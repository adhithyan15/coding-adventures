/**
 * Shared numerical routines, DSP helpers, framing validation, and CWE-190 guards (VER15).
 */

import { rfftScalar } from "@coding-adventures/dsp-fft";

import {
  BudgetExceededError,
  InvalidInputError,
  MAX_FFT_LEN,
  MAX_FRAMES,
  MAX_SAMPLE_RATE_HZ,
  MAX_SAMPLES,
  MIN_SAMPLE_RATE_HZ,
  validatePositiveFloat,
  validatePositiveInt,
  validateSamples,
} from "./types.js";

/**
 * Build a symmetric Hann analysis window of length `n`.
 */
export function buildHannWindow(n: number): Float64Array {
  if (n <= 0) {
    return new Float64Array(0);
  }
  if (n === 1) {
    return new Float64Array([1.0]);
  }
  const out = new Float64Array(n);
  const m = n - 1;
  const twoPi = 2.0 * Math.PI;
  for (let i = 0; i < n; i++) {
    out[i] = 0.5 * (1.0 - Math.cos((twoPi * i) / m));
  }
  return out;
}

/**
 * Refine a discrete peak position `peakX` using 3-point parabolic interpolation.
 */
export function parabolicInterpolation(
  y0: number,
  y1: number,
  y2: number,
  peakX: number,
): number {
  const denom = 2.0 * (y0 - 2.0 * y1 + y2);
  if (Math.abs(denom) > 1e-12) {
    const delta = (y0 - y2) / denom;
    if (Math.abs(delta) < 1.0) {
      return peakX + delta;
    }
  }
  return peakX;
}

/**
 * Compute the real FFT magnitude spectrum of windowed time-domain audio samples.
 * Returns Float64Array of length Math.floor(fftLen / 2) + 1.
 */
export function rfftMagnitudes(windowed: ArrayLike<number>): Float64Array {
  const fftLen = windowed.length;
  if (fftLen === 0) {
    throw new InvalidInputError("windowed buffer must be non-empty");
  }
  if (fftLen > MAX_FFT_LEN) {
    throw new BudgetExceededError(
      `windowed buffer length ${fftLen} exceeds MAX_FFT_LEN=${MAX_FFT_LEN}`,
    );
  }
  const spec = rfftScalar(windowed);
  const kBins = Math.floor(fftLen / 2) + 1;
  const mags = new Float64Array(kBins);
  for (let k = 0; k < kBins; k++) {
    const re = spec[2 * k];
    const im = spec[2 * k + 1];
    mags[k] = Math.sqrt(re * re + im * im);
  }
  return mags;
}

/**
 * Compute single-step half-wave rectified or squared spectral flux between two frames.
 */
export function computeSpectralFluxStep(
  prev: ArrayLike<number>,
  curr: ArrayLike<number>,
  rectified: boolean,
): number {
  const nBins = Math.min(prev.length, curr.length);
  let flux = 0.0;
  for (let k = 0; k < nBins; k++) {
    const diff = curr[k] - prev[k];
    if (rectified) {
      if (diff > 0.0) {
        flux += diff;
      }
    } else {
      flux += diff * diff;
    }
  }
  return flux;
}

/**
 * Validate framing parameters and compute total frame count with CWE-400 / CWE-190 enforcement.
 * Returns `number` (numFrames) if `signalLen >= frameLength`, or `null` if `signalLen < frameLength`.
 */
export function validateFraming(
  signalLen: number,
  frameLength: number,
  hopLength: number,
): number | null {
  const fLen = validatePositiveInt(frameLength, "frameLength", MAX_SAMPLES);
  const hLen = validatePositiveInt(hopLength, "hopLength", MAX_SAMPLES);

  if (signalLen < fLen) {
    return null;
  }

  const diff = signalLen - fLen;
  const numFrames = 1 + Math.floor(diff / hLen);

  if (numFrames > MAX_FRAMES) {
    throw new BudgetExceededError(
      `numFrames ${numFrames} exceeds MAX_FRAMES=${MAX_FRAMES}`,
    );
  }

  return numFrames;
}

/**
 * Safely calculate `[start, end]` slice boundaries for frame `m` with safe integer and bounds checks.
 */
export function checkedFrameBounds(
  m: number,
  hopLength: number,
  frameLength: number,
  signalLen: number,
): [number, number] {
  const start = m * hopLength;
  if (!Number.isSafeInteger(start)) {
    throw new BudgetExceededError("frame start offset overflowed safe integer");
  }
  const end = start + frameLength;
  if (!Number.isSafeInteger(end)) {
    throw new BudgetExceededError("frame end offset overflowed safe integer");
  }
  if (end > signalLen) {
    throw new BudgetExceededError(
      `frame end ${end} exceeds signal length ${signalLen}`,
    );
  }
  return [start, end];
}

/**
 * Common validation helper for audio sample buffer and sample rate.
 */
export function validateAudioInputs(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
): { validSamples: Float64Array; validSr: number } {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const validSamples = validateSamples(samples, "samples");
  return { validSamples, validSr };
}

/**
 * Standardized validation helper for PCM sources.
 */
export function validatePcmSourceBounds(
  len: number,
  sampleRateHz: number,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  if (validSr < MIN_SAMPLE_RATE_HZ) {
    throw new InvalidInputError(
      `sample_rate_hz ${validSr} below minimum ${MIN_SAMPLE_RATE_HZ}`,
    );
  }
  validatePositiveInt(len, "pcmSourceLength", MAX_SAMPLES);
  return validSr;
}
