import { MAX_FFT_LEN, MAX_SAMPLES } from "./constants.js";
import { Direction } from "./direction.js";
import {
  InvalidInputError,
  NotPowerOfTwoError,
  readFiniteNumber,
  safeFormatValue,
  validateArrayLikeLength,
} from "./errors.js";

/**
 * Validate buffer finiteness, bounds, and even length pre-flight.
 * Returns the number of complex elements n = data.length / 2.
 */
export function validateBuffer(
  data: Float64Array | ArrayLike<number>,
  name: string = "buffer"
): number {
  const rawLen = validateArrayLikeLength(data, name);
  if (rawLen === 0) {
    throw new InvalidInputError(`${name} length must be at least 1`);
  }
  if (rawLen > MAX_SAMPLES) {
    throw new InvalidInputError(
      `${name} length ${rawLen} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
    );
  }
  if (rawLen % 2 !== 0) {
    throw new InvalidInputError(
      `interleaved ${name} must have even length; got ${rawLen}`
    );
  }

  const n = rawLen / 2;
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `${name} FFT length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }

  for (let i = 0; i < rawLen; i++) {
    readFiniteNumber(data, i);
  }

  return n;
}

export function requirePow2Length(n: number): void {
  if (typeof n !== "number" || !Number.isSafeInteger(n) || n < 1) {
    throw new InvalidInputError(
      `FFT length must be a positive integer; got ${safeFormatValue(n)}`
    );
  }
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `FFT length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }
  if ((n & (n - 1)) !== 0) {
    throw new NotPowerOfTwoError(n);
  }
}

/**
 * In-place bit-reversal permutation on interleaved [re_0, im_0, ...] buffer.
 * Strictly iterative algorithm (CWE-674).
 */
export function bitReversalPermute(buf: Float64Array, n: number): void {
  const bits = 31 - Math.clz32(n);
  for (let i = 0; i < n; i++) {
    let r = 0;
    let x = i;
    for (let b = 0; b < bits; b++) {
      r = (r << 1) | (x & 1);
      x >>= 1;
    }
    if (r > i) {
      const idxI = 2 * i;
      const idxR = 2 * r;
      const tmpRe = buf[idxI];
      const tmpIm = buf[idxI + 1];
      buf[idxI] = buf[idxR];
      buf[idxI + 1] = buf[idxR + 1];
      buf[idxR] = tmpRe;
      buf[idxR + 1] = tmpIm;
    }
  }
}

/**
 * In-place iterative butterfly stages on bit-reversed buffer.
 * Strictly iterative execution with zero recursion (CWE-674).
 */
export function butterflies(
  buf: Float64Array,
  n: number,
  dir: Direction
): void {
  const sign = dir === Direction.FORWARD ? -1.0 : 1.0;
  let full = 2;
  while (full <= n) {
    const half = full >> 1;
    const angleStep = (sign * 2.0 * Math.PI) / full;
    for (let blockStart = 0; blockStart < n; blockStart += full) {
      for (let j = 0; j < half; j++) {
        const theta = j * angleStep;
        const wRe = Math.cos(theta);
        const wIm = Math.sin(theta);

        const xIdx = 2 * (blockStart + j);
        const yIdx = 2 * (blockStart + j + half);

        const xRe = buf[xIdx];
        const xIm = buf[xIdx + 1];
        const yRe = buf[yIdx];
        const yIm = buf[yIdx + 1];

        // Complex multiplication: t = w * y
        const tRe = wRe * yRe - wIm * yIm;
        const tIm = wRe * yIm + wIm * yRe;

        buf[xIdx] = xRe + tRe;
        buf[xIdx + 1] = xIm + tIm;
        buf[yIdx] = xRe - tRe;
        buf[yIdx + 1] = xIm - tIm;
      }
    }
    full <<= 1;
  }
}

/**
 * Forward FFT on interleaved [re, im, ...] data.
 * Length must be 2 * N where N is a power of two.
 * Returns a new interleaved Float64Array.
 */
export function fftScalar(
  signal: Float64Array | ArrayLike<number>
): Float64Array {
  const n = validateBuffer(signal, "signal");
  requirePow2Length(n);

  const buf = new Float64Array(signal.length);
  for (let i = 0; i < signal.length; i++) {
    buf[i] = Number(signal[i]);
  }

  bitReversalPermute(buf, n);
  butterflies(buf, n, Direction.FORWARD);
  return buf;
}

/**
 * Inverse FFT on interleaved [re, im, ...] data.
 * Length must be 2 * N where N is a power of two.
 * Applies backward normalization (division by N).
 */
export function ifftScalar(
  spectrum: Float64Array | ArrayLike<number>
): Float64Array {
  const n = validateBuffer(spectrum, "spectrum");
  requirePow2Length(n);

  const buf = new Float64Array(spectrum.length);
  for (let i = 0; i < spectrum.length; i++) {
    buf[i] = Number(spectrum[i]);
  }

  bitReversalPermute(buf, n);
  butterflies(buf, n, Direction.INVERSE);

  const invN = 1.0 / n;
  for (let i = 0; i < buf.length; i++) {
    buf[i] *= invN;
  }

  return buf;
}
