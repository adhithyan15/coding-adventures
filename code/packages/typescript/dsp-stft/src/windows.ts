import { MAX_FFT_LEN } from "./constants.js";
import { InvalidParamError, safeFormatValue } from "./errors.js";
import { parseWindowType, WindowType } from "./types.js";

/**
 * Generate a Hann (Hanning) window of the given length.
 */
export function hannWindow(length: number): Float64Array {
  return buildWindow(WindowType.HANN, length);
}

/**
 * Generate a Hamming window of the given length.
 */
export function hammingWindow(length: number): Float64Array {
  return buildWindow(WindowType.HAMMING, length);
}

/**
 * Generate a Blackman window of the given length.
 */
export function blackmanWindow(length: number): Float64Array {
  return buildWindow(WindowType.BLACKMAN, length);
}

/**
 * Generate a rectangular (boxcar) window of the given length.
 */
export function rectangularWindow(length: number): Float64Array {
  return buildWindow(WindowType.RECTANGULAR, length);
}

/**
 * Build an analysis window of length `nFft` per the `WindowType`.
 * Symmetric convention (span [0, nFft - 1]).
 */
export function buildWindow(
  window: WindowType | string,
  nFft: number
): Float64Array {
  if (!Number.isSafeInteger(nFft) || nFft <= 0) {
    throw new InvalidParamError(`window length must be a positive integer; got ${safeFormatValue(nFft)}`);
  }
  if (nFft > MAX_FFT_LEN) {
    throw new InvalidParamError(
      `window length ${nFft} exceeds MAX_FFT_LEN ${MAX_FFT_LEN}`
    );
  }

  const wType = parseWindowType(window);
  const w = new Float64Array(nFft);

  if (nFft === 1) {
    w[0] = 1.0;
    return w;
  }

  const m = nFft - 1;
  const twoPiOverM = (2.0 * Math.PI) / m;
  const fourPiOverM = (4.0 * Math.PI) / m;

  for (let n = 0; n < nFft; n++) {
    switch (wType) {
      case WindowType.HANN:
        w[n] = 0.5 * (1.0 - Math.cos(twoPiOverM * n));
        break;
      case WindowType.HAMMING:
        w[n] = 0.54 - 0.46 * Math.cos(twoPiOverM * n);
        break;
      case WindowType.BLACKMAN:
        w[n] =
          0.42 -
          0.5 * Math.cos(twoPiOverM * n) +
          0.08 * Math.cos(fourPiOverM * n);
        break;
      case WindowType.RECTANGULAR:
        w[n] = 1.0;
        break;
    }
  }

  return w;
}
