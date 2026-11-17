import { dct, DctNorm, DctType } from "@coding-adventures/dsp-dct";
import { MAX_FFT_LEN, MAX_MELS, MAX_SAMPLES } from "./constants.js";
import { InvalidParamError } from "./errors.js";
import { powerSpectrogram } from "./spectrogram.js";
import { WindowType } from "./types.js";

const EPS = 1e-10;

/**
 * Hz -> mel scale (HTK convention).
 * mel(0) = 0, mel(700) ~ 781.
 */
export function hzToMel(f: number): number {
  return 2595.0 * Math.log10(1.0 + f / 700.0);
}

/**
 * Mel -> Hz (HTK convention). Inverse of hzToMel.
 */
export function melToHz(m: number): number {
  return 700.0 * (Math.pow(10.0, m / 2595.0) - 1.0);
}

/**
 * Build an [nMels, nFft / 2 + 1] triangular mel filterbank.
 */
export function melFilterbank(
  nMels: number,
  nFft: number,
  sampleRate: number
): Float64Array {
  if (
    !Number.isSafeInteger(nMels) ||
    !Number.isSafeInteger(nFft) ||
    nMels <= 0 ||
    nFft <= 0 ||
    !Number.isFinite(sampleRate) ||
    sampleRate <= 0.0 ||
    nMels > MAX_MELS ||
    nFft > MAX_FFT_LEN
  ) {
    return new Float64Array(0);
  }

  const bins = Math.floor(nFft / 2) + 1;
  const totalLen = nMels * bins;
  if (totalLen > MAX_SAMPLES) {
    return new Float64Array(0);
  }

  const fmin = 0.0;
  const fmax = sampleRate * 0.5; // Nyquist
  const melMin = hzToMel(fmin);
  const melMax = hzToMel(fmax);
  const step = (melMax - melMin) / (nMels + 1);
  const binPerHz = nFft / sampleRate;

  const binPts = new Float64Array(nMels + 2);
  for (let i = 0; i < nMels + 2; i++) {
    const mel = melMin + step * i;
    binPts[i] = melToHz(mel) * binPerHz;
  }

  const filterbank = new Float64Array(totalLen);

  for (let m = 0; m < nMels; m++) {
    const left = binPts[m];
    const center = binPts[m + 1];
    const right = binPts[m + 2];
    const rowOffset = m * bins;

    let rowSum = 0.0;
    for (let k = 0; k < bins; k++) {
      let w = 0.0;
      if (k >= left && k <= right) {
        if (k <= center) {
          if (center > left) {
            w = (k - left) / (center - left);
          }
        } else if (right > center) {
          w = (right - k) / (right - center);
        }
      }
      filterbank[rowOffset + k] = w;
      rowSum += w;
    }

    if (rowSum > 0.0) {
      for (let k = 0; k < bins; k++) {
        filterbank[rowOffset + k] /= rowSum;
      }
    }
  }

  return filterbank;
}

/**
 * Mel spectrogram: melFilterbank @ |STFT|^2.
 *
 * Output shape: row-major [numFrames, nMels].
 */
export function melSpectrogram(
  signal: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  nMels: number,
  sampleRate: number,
  window: WindowType | string = WindowType.HANN
): Float64Array {
  if (!Number.isSafeInteger(nMels) || nMels <= 0) {
    throw new InvalidParamError(`nMels must be a positive integer; got ${nMels}`);
  }
  if (nMels > MAX_MELS) {
    throw new InvalidParamError(
      `nMels ${nMels} exceeds MAX_MELS ${MAX_MELS}`
    );
  }
  if (!Number.isFinite(sampleRate) || sampleRate <= 0.0) {
    throw new InvalidParamError(
      `sampleRate must be positive and finite; got ${sampleRate}`
    );
  }

  const bins = Math.floor(nFft / 2) + 1;
  const power = powerSpectrogram(signal, nFft, hopLength, window);
  const numFrames = power.length / bins;

  const fb = melFilterbank(nMels, nFft, sampleRate);
  if (fb.length !== nMels * bins) {
    throw new InvalidParamError("mel filterbank generation failed");
  }

  const outLen = numFrames * nMels;
  if (outLen > MAX_SAMPLES) {
    throw new InvalidParamError("mel spectrogram output length exceeds MAX_SAMPLES");
  }

  const out = new Float64Array(outLen);
  for (let t = 0; t < numFrames; t++) {
    const pRowOffset = t * bins;
    const outRowOffset = t * nMels;
    for (let m = 0; m < nMels; m++) {
      const fbRowOffset = m * bins;
      let acc = 0.0;
      for (let k = 0; k < bins; k++) {
        acc += fb[fbRowOffset + k] * power[pRowOffset + k];
      }
      out[outRowOffset + m] = acc;
    }
  }

  return out;
}

/**
 * MFCC — Mel-Frequency Cepstral Coefficients.
 *
 * mfcc[t, c] = DCT-II_ortho( log(mel_spectrogram[t, :] + eps) )[c]
 * for c in [0, nMfcc).
 *
 * Output layout: row-major [numFrames, nMfcc].
 */
export function mfcc(
  signal: ArrayLike<number>,
  nFft: number,
  hopLength: number,
  nMels: number,
  nMfcc: number,
  sampleRate: number,
  window: WindowType | string = WindowType.HANN
): Float64Array {
  if (!Number.isSafeInteger(nMfcc) || nMfcc <= 0) {
    throw new InvalidParamError(`nMfcc must be a positive integer; got ${nMfcc}`);
  }
  if (nMfcc > nMels) {
    throw new InvalidParamError(
      `nMfcc ${nMfcc} > nMels ${nMels} (cannot keep more DCT coefficients than DCT inputs)`
    );
  }

  const mel = melSpectrogram(signal, nFft, hopLength, nMels, sampleRate, window);
  const numFrames = mel.length / nMels;

  const totalMfccLen = numFrames * nMfcc;
  if (totalMfccLen > MAX_SAMPLES) {
    throw new InvalidParamError("MFCC output length exceeds MAX_SAMPLES");
  }

  const out = new Float64Array(totalMfccLen);
  const rowBuf = new Float64Array(nMels);

  for (let t = 0; t < numFrames; t++) {
    const inOffset = t * nMels;
    for (let m = 0; m < nMels; m++) {
      rowBuf[m] = Math.log(mel[inOffset + m] + EPS);
    }
    const coeffs = dct(rowBuf, DctType.II, DctNorm.ORTHO);
    const outOffset = t * nMfcc;
    for (let c = 0; c < nMfcc; c++) {
      out[outOffset + c] = coeffs[c];
    }
  }

  return out;
}
