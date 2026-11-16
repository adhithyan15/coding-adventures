import { MAX_FFT_LEN } from "./constants.js";
import { Direction } from "./direction.js";
import { InvalidInputError } from "./errors.js";
import { fftScalar, ifftScalar, validateBuffer } from "./radix2.js";

/**
 * Build chirp sequence chirp[k] = exp(sign * i * pi * k^2 / N).
 * Reduces k^2 modulo 2N to maintain maximum numerical precision.
 * Returns interleaved Float64Array of length 2 * N.
 */
export function buildChirp(n: number, sign: number): Float64Array {
  if (!Number.isFinite(n) || !Number.isInteger(n) || n < 1) {
    throw new InvalidInputError("chirp length must be a positive integer");
  }
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `chirp length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }
  if (!Number.isFinite(sign)) {
    throw new InvalidInputError("chirp sign must be finite");
  }
  const twoN = 2 * n;
  const out = new Float64Array(twoN);
  for (let k = 0; k < n; k++) {
    const kSq = (k * k) % twoN;
    const theta = (sign * Math.PI * kSq) / n;
    out[2 * k] = Math.cos(theta);
    out[2 * k + 1] = Math.sin(theta);
  }
  return out;
}

/**
 * Compute forward or inverse FFT of arbitrary length N >= 1 via Bluestein's algorithm.
 * signal is interleaved [re_0, im_0, ...] of length 2 * N.
 * Returns an interleaved Float64Array of length 2 * N.
 */
export function bluesteinScalar(
  signal: Float64Array | ArrayLike<number>,
  direction: Direction
): Float64Array {
  const n = validateBuffer(signal, "signal");

  // N = 1 case: identity
  if (n === 1) {
    const out = new Float64Array(2);
    out[0] = Number(signal[0]);
    out[1] = Number(signal[1]);
    return out;
  }

  // Step 0: convolution length M = next_pow2(2N - 1)
  const convLen = 2 * n - 1;
  let m = 1;
  while (m < convLen) {
    m <<= 1;
  }
  if (m > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `Bluestein convolution length ${m} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }

  // Step 1: build chirp
  const sign = direction === Direction.FORWARD ? -1.0 : 1.0;
  const chirp = buildChirp(n, sign);

  // Step 2: a[k] = signal[k] * chirp[k], padded to length M with zeros
  const a = new Float64Array(2 * m);
  for (let k = 0; k < n; k++) {
    const xr = Number(signal[2 * k]);
    const xi = Number(signal[2 * k + 1]);
    const cr = chirp[2 * k];
    const ci = chirp[2 * k + 1];
    a[2 * k] = xr * cr - xi * ci;
    a[2 * k + 1] = xr * ci + xi * cr;
  }

  // Step 3: build bilateral wrapped chirp b' of length M
  // b'[k] = conj(chirp[k]) for k = 0..n-1
  // b'[m - k] = conj(chirp[k]) for k = 1..n-1
  const b = new Float64Array(2 * m);
  for (let k = 0; k < n; k++) {
    b[2 * k] = chirp[2 * k];
    b[2 * k + 1] = -chirp[2 * k + 1];
  }
  for (let k = 1; k < n; k++) {
    const idx = m - k;
    b[2 * idx] = chirp[2 * k];
    b[2 * idx + 1] = -chirp[2 * k + 1];
  }

  // Step 4: convolve via radix-2 FFT
  const aSpec = fftScalar(a);
  const bSpec = fftScalar(b);

  const cSpec = new Float64Array(2 * m);
  for (let k = 0; k < m; k++) {
    const ar = aSpec[2 * k];
    const ai = aSpec[2 * k + 1];
    const br = bSpec[2 * k];
    const bi = bSpec[2 * k + 1];
    cSpec[2 * k] = ar * br - ai * bi;
    cSpec[2 * k + 1] = ar * bi + ai * br;
  }

  const conv = ifftScalar(cSpec);

  // Step 5: X[k] = chirp[k] * conv[k], with 1/N scaling if inverse
  const invN = direction === Direction.INVERSE ? 1.0 / n : 1.0;
  const out = new Float64Array(2 * n);
  for (let k = 0; k < n; k++) {
    const cr = conv[2 * k];
    const ci = conv[2 * k + 1];
    const wr = chirp[2 * k];
    const wi = chirp[2 * k + 1];
    const re = wr * cr - wi * ci;
    const im = wr * ci + wi * cr;
    out[2 * k] = re * invN;
    out[2 * k + 1] = im * invN;
  }

  return out;
}
