/**
 * Harmonic profile and ADSR timbre estimation (MUS08, MUS05, VER15).
 */

import {
  AdsrEnvelope,
  HarmonicPartial,
  InstrumentProfile,
  VariationProfile,
} from "@coding-adventures/musical-instruments";

import { buildHannWindow, rfftMagnitudes } from "./dsp_utils.js";
import { rmsEnergy, rmsEnergyFramed } from "./features.js";
import { hpsPitch, yinPitch } from "./pitch.js";
import {
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
 * Extract fundamental, harmonic partial amplitudes, and ADSR envelope from single-note audio.
 *
 * Returns a MUS05 `InstrumentProfile` suitable for additive re-synthesis in `musical-instruments`
 * and portable score playback in `music-machine-web`.
 *
 * @param samples Time-domain audio sample buffer of a single note.
 * @param sampleRateHz Sampling frequency in Hertz.
 * @param fundamentalHz Optional known fundamental frequency (Hz). If `null` or `undefined`, estimated.
 * @param maxPartials Maximum number of harmonic partials to extract (up to 64).
 * @param instrumentId Identifier string for the resulting instrument profile.
 * @param displayName Human-readable display label.
 */
export function extractHarmonicProfile(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
  fundamentalHz?: number | null,
  maxPartials: number = 16,
  instrumentId: string = "extracted_instrument",
  displayName: string = "Extracted Instrument",
): InstrumentProfile {
  const validSr = validatePositiveFloat(
    sampleRateHz,
    "sample_rate_hz",
    MAX_SAMPLE_RATE_HZ,
  );
  const nPartials = validatePositiveInt(
    maxPartials,
    "max_partials",
    MAX_PARTIALS,
  );
  const s = validateSamples(samples, "samples");
  const n = s.length;

  const overallRms = rmsEnergy(s);
  if (overallRms < 1e-4) {
    throw new InvalidInputError(
      "signal is silent, cannot extract harmonic profile",
    );
  }

  // 1. Determine fundamental frequency
  let f0: number;
  if (fundamentalHz !== undefined && fundamentalHz !== null) {
    f0 = validatePositiveFloat(
      fundamentalHz,
      "fundamental_hz",
      validSr / 2.0,
    );
  } else {
    let pitch = yinPitch(s, validSr, 40.0, 2000.0, 0.15);
    if (pitch <= 0.0) {
      pitch = hpsPitch(s, validSr, 5, 40.0, 2000.0, undefined);
    }
    if (pitch <= 0.0) {
      throw new InvalidInputError(
        "failed to detect fundamental pitch in signal",
      );
    }
    f0 = pitch;
  }

  // 2. Extract harmonic partials from sustained portion
  const targetFft = 4096;
  let analysisSlice: Float64Array;
  let fftLen: number;

  if (n > targetFft) {
    const start = Math.floor((n - targetFft) / 2);
    analysisSlice = s.subarray(start, start + targetFft);
    fftLen = targetFft;
  } else {
    const p2 = n <= 1 ? 1 : nextPowerOfTwo(n);
    const fl = Math.min(MAX_FFT_LEN, Math.max(512, p2));
    const padded = new Float64Array(fl);
    padded.set(s, 0);
    analysisSlice = padded;
    fftLen = fl;
  }

  const win = buildHannWindow(fftLen);
  const windowed = new Float64Array(fftLen);
  for (let i = 0; i < fftLen; i++) {
    windowed[i] = analysisSlice[i] * win[i];
  }

  const mags = rfftMagnitudes(windowed);
  const kBins = mags.length;
  const freqStep = validSr / fftLen;

  // Find fundamental amplitude
  const k1 = Math.round(f0 / freqStep);
  const searchW = 4;
  const k1Low = Math.max(0, k1 - searchW);
  const k1High = Math.min(kBins, k1 + searchW + 1);
  let a1 = 0.0;
  for (let k = k1Low; k < k1High; k++) {
    if (mags[k] > a1) {
      a1 = mags[k];
    }
  }
  if (a1 <= 0.0) {
    a1 = 1.0;
  }

  const partials: HarmonicPartial[] = [];
  for (let h = 1; h <= nPartials; h++) {
    const fh = h * f0;
    if (fh >= validSr / 2.0) {
      break;
    }
    const kh = Math.round(fh / freqStep);
    const khLow = Math.max(0, kh - searchW);
    const khHigh = Math.min(kBins, kh + searchW + 1);
    let ah = 0.0;
    for (let k = khLow; k < khHigh; k++) {
      if (mags[k] > ah) {
        ah = mags[k];
      }
    }
    const relAmp = Math.min(1.0, Math.max(0.0, ah / a1));
    const roundedAmp = Math.round(relAmp * 10000.0) / 10000.0;
    partials.push(new HarmonicPartial(h, roundedAmp, 0.0));
  }

  if (partials.length === 0) {
    partials.push(new HarmonicPartial(1.0, 1.0, 0.0));
  }

  // 3. Estimate ADSR Envelope
  const frameLen = 256;
  const hopLen = 128;
  const rmsTrack =
    n >= frameLen
      ? rmsEnergyFramed(s, frameLen, hopLen)
      : [overallRms];

  let maxRms = 0.0;
  let peakIdx = 0;
  for (let idx = 0; idx < rmsTrack.length; idx++) {
    const r = rmsTrack[idx];
    if (r > maxRms) {
      maxRms = r;
      peakIdx = idx;
    }
  }
  if (maxRms <= 0.0) {
    maxRms = 1.0;
  }

  const totalDurSec = n / validSr;
  const attackSec = Math.max(
    0.005,
    Math.min(totalDurSec * 0.4, (peakIdx * hopLen) / validSr),
  );

  // Release detection: when energy drops below 15% of max near end
  let relFrame = Math.max(0, rmsTrack.length - 1);
  while (relFrame > peakIdx && rmsTrack[relFrame] < 0.15 * maxRms) {
    relFrame--;
  }

  const releaseSec = Math.max(
    0.005,
    Math.min(
      totalDurSec * 0.4,
      ((rmsTrack.length - 1 - relFrame) * hopLen) / validSr,
    ),
  );

  // Sustain level: average between peak and release
  const sustainStart = Math.min(
    peakIdx + 1,
    Math.max(0, rmsTrack.length - 1),
  );
  const sustainEnd = Math.max(sustainStart, relFrame);
  let sustainLevel: number;
  if (sustainStart < sustainEnd) {
    let sumVal = 0.0;
    const sliceCount = sustainEnd - sustainStart;
    for (let i = sustainStart; i < sustainEnd; i++) {
      sumVal += rmsTrack[i];
    }
    const avg = sumVal / sliceCount;
    sustainLevel = Math.min(1.0, Math.max(0.05, avg / maxRms));
  } else {
    sustainLevel = 0.7;
  }

  const decaySec = Math.max(
    0.005,
    Math.min(totalDurSec * 0.3, attackSec * 0.5),
  );

  const envelope = new AdsrEnvelope(
    Math.round(Math.min(60.0, attackSec) * 10000.0) / 10000.0,
    Math.round(Math.min(60.0, decaySec) * 10000.0) / 10000.0,
    Math.round(sustainLevel * 1000.0) / 1000.0,
    Math.round(Math.min(60.0, releaseSec) * 10000.0) / 10000.0,
  );

  let peakAmp = 0.0;
  for (let i = 0; i < n; i++) {
    const a = Math.abs(s[i]);
    if (a > peakAmp) {
      peakAmp = a;
    }
  }
  const gain = Math.round(Math.min(1.0, Math.max(0.05, peakAmp)) * 1000.0) / 1000.0;

  return new InstrumentProfile(
    instrumentId,
    displayName,
    "additive",
    gain,
    partials,
    envelope,
    new VariationProfile(),
    null,
    "custom",
  );
}

/**
 * Convenience wrapper for `extractHarmonicProfile` with standard default labels.
 */
export function extractHarmonicProfileSimple(
  samples: ArrayLike<number> | Iterable<number>,
  sampleRateHz: number,
): InstrumentProfile {
  return extractHarmonicProfile(
    samples,
    sampleRateHz,
    null,
    16,
    "extracted_instrument",
    "Extracted Instrument",
  );
}

// Snake-case aliases
export const extract_harmonic_profile = extractHarmonicProfile;
export const extract_harmonic_profile_simple = extractHarmonicProfileSimple;
