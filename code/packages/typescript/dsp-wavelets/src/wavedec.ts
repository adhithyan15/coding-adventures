import { parseWaveletBoundary } from "./boundary.js";
import { MAX_SAMPLES, MAX_WAVELET_LEVELS } from "./constants.js";
import { filterAndDownsample, synthesizeOneLevel } from "./dwt.js";
import {
  EmptySignalError,
  InvalidCoefficientsError,
  InvalidParamError,
  safeFormatValue,
  SignalTooShortError,
  validateFloats,
  validateSequence,
} from "./errors.js";
import { WaveletBoundary } from "./types.js";
import {
  parseWavelet,
  requireOrthogonalWavelet,
  Wavelet,
} from "./wavelet.js";

const requireOrthogonalWavedec = (w: Wavelet) =>
  requireOrthogonalWavelet(w, "wavedec");

/**
 * Compute multi-level 1-D wavelet decomposition.
 *
 * Returns [cA_J, cD_J, cD_{J-1}, ..., cD_1].
 */
export function wavedec(
  signal: ArrayLike<number>,
  wavelet: Wavelet | string = "haar",
  boundary: WaveletBoundary | string = WaveletBoundary.SYMMETRIC,
  level?: number
): Float64Array[] {
  const sigLen = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError, InvalidParamError);
  validateFloats(signal, "signal");

  const w = parseWavelet(wavelet);
  requireOrthogonalWavedec(w);
  const b = parseWaveletBoundary(boundary);

  let levels: number;
  if (level === undefined) {
    // Default level = floor(log2(signal.length))
    const maxLevel = Math.max(1, 32 - Math.clz32(sigLen) - 1);
    levels = Math.min(maxLevel, MAX_WAVELET_LEVELS);
  } else {
    if (!Number.isSafeInteger(level) || level <= 0) {
      throw new InvalidParamError(`level must be > 0; got ${safeFormatValue(level)}`);
    }
    if (level > MAX_WAVELET_LEVELS) {
      throw new InvalidParamError(`level ${level} exceeds MAX_WAVELET_LEVELS`);
    }
    levels = level;
  }

  const minRequiredLen = Math.pow(2, levels - 1);
  if (sigLen < minRequiredLen) {
    throw new SignalTooShortError(
      `signal length ${sigLen} too short for ${levels} levels`
    );
  }

  let current: Float64Array = new Float64Array(signal.length);
  for (let i = 0; i < signal.length; i++) {
    current[i] = Number(signal[i]);
  }

  const details: Float64Array[] = [];

  for (let j = 0; j < levels; j++) {
    const [ca, cd] = filterAndDownsample(current, w.decLo, w.decHi, b);
    details.push(cd);
    current = ca;
  }

  const result: Float64Array[] = [current];
  for (let i = details.length - 1; i >= 0; i--) {
    result.push(details[i]);
  }
  return result;
}

/**
 * Reconstruct 1-D signal from [cA_J, cD_J, ..., cD_1] coefficients.
 */
export function waverec(
  coeffs: ArrayLike<ArrayLike<number>>,
  wavelet: Wavelet | string = "haar",
  boundary: WaveletBoundary | string = WaveletBoundary.SYMMETRIC
): Float64Array {
  const coeffsLen = validateSequence(
    coeffs,
    "coeffs",
    Number.MAX_SAFE_INTEGER,
    null,
    InvalidCoefficientsError
  );
  if (coeffsLen < 2) {
    throw new InvalidCoefficientsError(
      "coeffs list must contain at least [cA, cD] bands"
    );
  }
  const levels = coeffsLen - 1;
  if (levels > MAX_WAVELET_LEVELS) {
    throw new InvalidParamError(
      `number of levels (${levels}) exceeds MAX_WAVELET_LEVELS ${MAX_WAVELET_LEVELS}`
    );
  }

  for (let idx = 0; idx < coeffsLen; idx++) {
    const band = coeffs[idx];
    validateSequence(band, `band_${idx}`, MAX_SAMPLES, EmptySignalError, InvalidParamError);
    validateFloats(band, `band_${idx}`, InvalidCoefficientsError);
  }

  const w = parseWavelet(wavelet);
  requireOrthogonalWavedec(w);
  const b = parseWaveletBoundary(boundary);

  let current: Float64Array = new Float64Array(coeffs[0].length);
  for (let i = 0; i < coeffs[0].length; i++) {
    current[i] = Number(coeffs[0][i]);
  }

  // Successively synthesize each detail level
  for (let j = 1; j < coeffs.length; j++) {
    const cd = coeffs[j];
    const targetLen = 2 * current.length;
    if (targetLen > MAX_SAMPLES) {
      throw new InvalidParamError("target_len exceeds MAX_SAMPLES");
    }
    current = synthesizeOneLevel(current, cd, w.recLo, w.recHi, targetLen, b);
  }

  return current;
}
