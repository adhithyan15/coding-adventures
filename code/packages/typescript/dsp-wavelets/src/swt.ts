import { parseWaveletBoundary, sampleWithBoundary } from "./boundary.js";
import { MAX_SAMPLES, MAX_WAVELET_LEVELS } from "./constants.js";
import {
  EmptySignalError,
  InvalidCoefficientsError,
  InvalidParamError,
  safeFormatValue,
  validateFloats,
  validateSequence,
} from "./errors.js";
import { WaveletBoundary } from "./types.js";
import {
  FRAC_1_SQRT_2,
  parseWavelet,
  requireOrthogonalWavelet,
  Wavelet,
} from "./wavelet.js";

const requireOrthogonalSwt = (w: Wavelet) =>
  requireOrthogonalWavelet(w, "SWT");

/**
 * Compute stationary (undecimated) wavelet transform.
 *
 * Returns a list of [cA, cD] tuples from level J down to 1.
 * Each array has the same length as the input signal.
 */
export function swt(
  signal: ArrayLike<number>,
  wavelet: Wavelet | string = "haar",
  level = 1,
  boundary: WaveletBoundary | string = WaveletBoundary.PERIODIC
): Array<[Float64Array, Float64Array]> {
  const n = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError, InvalidParamError);
  if (!Number.isSafeInteger(level) || level <= 0) {
    throw new InvalidParamError(`level must be > 0; got ${safeFormatValue(level)}`);
  }
  if (level > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(`level ${level} exceeds MAX_WAVELET_LEVELS`);
  }
  validateFloats(signal, "signal");
  const w = parseWavelet(wavelet);
  requireOrthogonalSwt(w);
  const b = parseWaveletBoundary(boundary);

  const hScaled = new Float64Array(w.decLo.length);
  const gScaled = new Float64Array(w.decHi.length);
  for (let i = 0; i < w.decLo.length; i++) {
    hScaled[i] = w.decLo[i] * FRAC_1_SQRT_2;
  }
  for (let i = 0; i < w.decHi.length; i++) {
    gScaled[i] = w.decHi[i] * FRAC_1_SQRT_2;
  }
  const filterLen = hScaled.length;

  let current = new Float64Array(signal.length);
  for (let i = 0; i < signal.length; i++) {
    current[i] = Number(signal[i]);
  }

  const coeffs: Array<[Float64Array, Float64Array]> = [];

  for (let j = 1; j <= level; j++) {
    const step = 1 << (j - 1);
    const ca = new Float64Array(n);
    const cd = new Float64Array(n);
    for (let idx = 0; idx < n; idx++) {
      let accH = 0.0;
      let accG = 0.0;
      for (let i = 0; i < filterLen; i++) {
        const srcIdx = idx - i * step;
        const s = sampleWithBoundary(current, srcIdx, b);
        accH += hScaled[i] * s;
        accG += gScaled[i] * s;
      }
      ca[idx] = accH;
      cd[idx] = accG;
    }
    coeffs.push([ca, cd]);
    current = ca;
  }

  // Return from coarsest (level) to finest (1)
  return coeffs.reverse();
}

/**
 * Inverse stationary wavelet transform.
 */
export function iswt(
  coeffs: ArrayLike<[ArrayLike<number>, ArrayLike<number>]>,
  wavelet: Wavelet | string = "haar",
  boundary: WaveletBoundary | string = WaveletBoundary.PERIODIC
): Float64Array {
  const level = validateSequence(
    coeffs,
    "coeffs",
    Number.MAX_SAFE_INTEGER,
    EmptySignalError,
    InvalidCoefficientsError
  );
  if (level > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(`level ${level} exceeds MAX_WAVELET_LEVELS`);
  }

  const w = parseWavelet(wavelet);
  requireOrthogonalSwt(w);
  const b = parseWaveletBoundary(boundary);

  const firstPair: unknown = coeffs[0];
  if (!Array.isArray(firstPair) || firstPair.length !== 2) {
    throw new InvalidCoefficientsError("each SWT entry must be a [cA, cD] pair");
  }
  const caTop = firstPair[0];
  const n = validateSequence(caTop, "cA_0", MAX_SAMPLES, EmptySignalError, InvalidParamError);

  // Validate all subbands shapes and finite floats BEFORE allocating rec
  for (let jIdx = 0; jIdx < level; jIdx++) {
    const pair: unknown = coeffs[jIdx];
    if (!Array.isArray(pair) || pair.length !== 2) {
      throw new InvalidCoefficientsError("each SWT entry must be a [cA, cD] pair");
    }
    const [caJ, cdJ] = pair;
    const caLen = validateSequence(caJ, `cA_${jIdx}`, MAX_SAMPLES, EmptySignalError, InvalidParamError);
    const cdLen = validateSequence(cdJ, `cD_${jIdx}`, MAX_SAMPLES, EmptySignalError, InvalidParamError);
    if (caLen !== n || cdLen !== n) {
      throw new InvalidCoefficientsError("all subbands must have length N");
    }
    validateFloats(caJ, `cA_${jIdx}`, InvalidCoefficientsError);
    validateFloats(cdJ, `cD_${jIdx}`, InvalidCoefficientsError);
  }

  const hScaled = new Float64Array(w.recLo.length);
  const gScaled = new Float64Array(w.recHi.length);
  for (let i = 0; i < w.recLo.length; i++) {
    hScaled[i] = w.recLo[i] * FRAC_1_SQRT_2;
  }
  for (let i = 0; i < w.recHi.length; i++) {
    gScaled[i] = w.recHi[i] * FRAC_1_SQRT_2;
  }
  const filterLenH = hScaled.length;
  const filterLenG = gScaled.length;

  let currentCa = new Float64Array(caTop.length);
  for (let i = 0; i < caTop.length; i++) {
    currentCa[i] = Number(caTop[i]);
  }

  for (let jIdx = 0; jIdx < coeffs.length; jIdx++) {
    const [, cdJ] = coeffs[jIdx];
    const j = level - jIdx; // Level from J down to 1
    const step = 1 << (j - 1);

    const rec = new Float64Array(n);
    for (let idx = 0; idx < n; idx++) {
      let acc = 0.0;
      for (let i = 0; i < filterLenH; i++) {
        const srcIdx = idx + i * step;
        const caVal = sampleWithBoundary(currentCa, srcIdx, b);
        acc += hScaled[i] * caVal;
      }
      for (let i = 0; i < filterLenG; i++) {
        const srcIdx = idx + i * step;
        const cdVal = sampleWithBoundary(cdJ, srcIdx, b);
        acc += gScaled[i] * cdVal;
      }
      rec[idx] = acc;
    }
    currentCa = rec;
  }

  return currentCa;
}
