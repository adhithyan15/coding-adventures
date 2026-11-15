/**
 * Spectral and time-domain audio feature extraction (MUS08, VER15).
 */

import {
  checkedFrameBounds,
  computeSpectralFluxStep,
  validateFraming,
} from "./dsp_utils.js";
import {
  BudgetExceededError,
  InvalidInputError,
  MAX_FRAMES,
  MAX_SAMPLE_RATE_HZ,
  validatePositiveFloat,
  validateSamples,
  validateSpectrum,
} from "./types.js";

/**
 * Compute the spectral centroid (center of mass / auditory brightness) of a spectrum.
 *
 * @param spectrum Magnitude or power spectrum bins from 0 Hz to Nyquist.
 * @param sampleRateHz Sampling frequency in Hertz (up to 192 kHz).
 * @returns Spectral centroid frequency in Hertz.
 */
export function spectralCentroid(
  spectrum: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const spec = validateSpectrum(spectrum, "spectrum");
  const kBins = spec.length;
  if (kBins <= 1) {
    return 0.0;
  }

  const nyquist = validSr / 2.0;
  const freqStep = nyquist / (kBins - 1);

  let weightedSum = 0.0;
  let totalMag = 0.0;

  // Iterative center of mass computation (CWE-674 zero recursion)
  for (let k = 0; k < kBins; k++) {
    const mag = Math.abs(spec[k]);
    const freq = k * freqStep;
    weightedSum += freq * mag;
    totalMag += mag;
  }

  if (totalMag <= 0.0) {
    return 0.0;
  }
  return weightedSum / totalMag;
}

/**
 * Compute spectral flux between two consecutive spectral frames.
 *
 * @param spectrumPrev Previous frame spectrum.
 * @param spectrumCurr Current frame spectrum.
 * @param rectified If true, only positive energy increases are summed (half-wave rectified).
 */
export function spectralFlux(
  spectrumPrev: ArrayLike<number> | Iterable<number>,
  spectrumCurr: ArrayLike<number> | Iterable<number>,
  rectified: boolean = true,
): number {
  const prev = validateSpectrum(spectrumPrev, "spectrum_prev");
  const curr = validateSpectrum(spectrumCurr, "spectrum_curr");
  return computeSpectralFluxStep(prev, curr, rectified);
}

/**
 * Compute spectral flux across a sequence of successive frames.
 */
export function spectralFluxSequence(
  frames: ReadonlyArray<ArrayLike<number> | Iterable<number>>,
  rectified: boolean = true,
): number[] {
  const nFrames = frames.length;
  if (nFrames === 0) {
    throw new InvalidInputError("frame count must be >= 1");
  }
  if (nFrames > MAX_FRAMES) {
    throw new BudgetExceededError(
      `frame count ${nFrames} exceeds MAX_FRAMES=${MAX_FRAMES}`,
    );
  }
  if (nFrames === 1) {
    validateSpectrum(frames[0], "frames[0]");
    return [];
  }

  const fluxes: number[] = [];
  let prev = validateSpectrum(frames[0], "frames[0]");

  for (let m = 1; m < nFrames; m++) {
    const curr = validateSpectrum(frames[m], `frames[${m}]`);
    const flux = computeSpectralFluxStep(prev, curr, rectified);
    fluxes.push(flux);
    prev = curr;
  }

  return fluxes;
}

/**
 * Compute spectral rolloff frequency below which `rolloffPercent` energy lies.
 *
 * @param spectrum Magnitude or power spectrum bins from 0 Hz to Nyquist.
 * @param sampleRateHz Sampling frequency in Hertz.
 * @param rolloffPercent Fraction of total energy in (0.0, 1.0] (default 0.85).
 */
export function spectralRolloff(
  spectrum: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  rolloffPercent: number = 0.85,
): number {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const validPct = validatePositiveFloat(
    rolloffPercent,
    "rolloff_percent",
    1.0,
  );
  const spec = validateSpectrum(spectrum, "spectrum");
  const kBins = spec.length;
  if (kBins <= 1) {
    return 0.0;
  }

  let totalEnergy = 0.0;
  for (let i = 0; i < kBins; i++) {
    totalEnergy += Math.abs(spec[i]);
  }

  if (totalEnergy <= 0.0) {
    return 0.0;
  }

  const target = totalEnergy * validPct;
  const nyquist = validSr / 2.0;
  const freqStep = nyquist / (kBins - 1);

  let accum = 0.0;
  for (let k = 0; k < kBins; k++) {
    accum += Math.abs(spec[k]);
    if (accum >= target) {
      return k * freqStep;
    }
  }

  return nyquist;
}

/**
 * Compute zero-crossing rate of a signal.
 */
export function zeroCrossingRate(
  samples: ArrayLike<number> | Iterable<number>,
): number {
  const s = validateSamples(samples, "samples");
  const n = s.length;
  if (n <= 1) {
    return 0.0;
  }

  let crossings = 0;
  for (let i = 1; i < n; i++) {
    const sCurr = s[i];
    const sPrev = s[i - 1];
    if ((sCurr >= 0.0 && sPrev < 0.0) || (sCurr < 0.0 && sPrev >= 0.0)) {
      crossings++;
    }
  }

  return crossings / (n - 1);
}

/**
 * Compute zero-crossing rate across sliding frames.
 */
export function zeroCrossingRateFramed(
  samples: ArrayLike<number> | Iterable<number>,
  frameLength: number,
  hopLength: number,
): number[] {
  const s = validateSamples(samples, "samples");
  const maybeFrames = validateFraming(s.length, frameLength, hopLength);
  if (maybeFrames === null) {
    const val = zeroCrossingRate(s);
    return [val];
  }
  const numFrames = maybeFrames;

  // CWE-20 guard: frameLength <= 1 must produce 0.0 to prevent 0 / 0 = NaN
  if (frameLength <= 1) {
    return new Array(numFrames).fill(0.0);
  }

  const denom = frameLength - 1;
  const out: number[] = [];
  for (let m = 0; m < numFrames; m++) {
    const [start, end] = checkedFrameBounds(
      m,
      hopLength,
      frameLength,
      s.length,
    );
    let crossings = 0;
    for (let i = start + 1; i < end; i++) {
      const sCurr = s[i];
      const sPrev = s[i - 1];
      if ((sCurr >= 0.0 && sPrev < 0.0) || (sCurr < 0.0 && sPrev >= 0.0)) {
        crossings++;
      }
    }
    out.push(crossings / denom);
  }

  return out;
}

/**
 * Compute root-mean-square (RMS) energy of a signal.
 */
export function rmsEnergy(
  samples: ArrayLike<number> | Iterable<number>,
): number {
  const s = validateSamples(samples, "samples");
  const n = s.length;
  let sumSq = 0.0;
  for (let i = 0; i < n; i++) {
    const v = s[i];
    sumSq += v * v;
  }
  return Math.sqrt(sumSq / n);
}

/**
 * Compute root-mean-square (RMS) energy across sliding frames.
 */
export function rmsEnergyFramed(
  samples: ArrayLike<number> | Iterable<number>,
  frameLength: number,
  hopLength: number,
): number[] {
  const s = validateSamples(samples, "samples");
  const maybeFrames = validateFraming(s.length, frameLength, hopLength);
  if (maybeFrames === null) {
    const val = rmsEnergy(s);
    return [val];
  }
  const numFrames = maybeFrames;

  const denom = frameLength;
  const out: number[] = [];
  for (let m = 0; m < numFrames; m++) {
    const [start, end] = checkedFrameBounds(
      m,
      hopLength,
      frameLength,
      s.length,
    );
    let sumSq = 0.0;
    for (let i = start; i < end; i++) {
      const v = s[i];
      sumSq += v * v;
    }
    out.push(Math.sqrt(sumSq / denom));
  }

  return out;
}

// Snake-case aliases
export const spectral_centroid = spectralCentroid;
export const spectral_flux = spectralFlux;
export const spectral_flux_sequence = spectralFluxSequence;
export const spectral_rolloff = spectralRolloff;
export const zero_crossing_rate = zeroCrossingRate;
export const zero_crossing_rate_framed = zeroCrossingRateFramed;
export const rms_energy = rmsEnergy;
export const rms_energy_framed = rmsEnergyFramed;
