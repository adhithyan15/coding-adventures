import { bluesteinScalar } from "./bluestein.js";
import { MAX_FFT_LEN, MAX_SAMPLES } from "./constants.js";
import { Direction } from "./direction.js";
import { InvalidInputError } from "./errors.js";
import { fftScalar, ifftScalar } from "./radix2.js";

/**
 * Compute forward FFT of a real-valued signal, returning half-spectrum.
 * Returns interleaved Float64Array of length 2 * (floor(N / 2) + 1).
 */
export function rfftScalar(
  signal: Float64Array | ArrayLike<number>
): Float64Array {
  const n = signal.length;
  if (n === 0) {
    throw new InvalidInputError("rfft input must be at least 1 sample");
  }
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `rfft input length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }
  if (n > MAX_SAMPLES) {
    throw new InvalidInputError(
      `rfft input length ${n} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
    );
  }

  // Pre-validate finite floats and pack into interleaved complex buffer
  const interleaved = new Float64Array(2 * n);
  for (let i = 0; i < n; i++) {
    const val = Number(signal[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
    }
    interleaved[2 * i] = val;
    interleaved[2 * i + 1] = 0.0;
  }

  const isPow2 = (n & (n - 1)) === 0 && n >= 1;
  const full = isPow2
    ? fftScalar(interleaved)
    : bluesteinScalar(interleaved, Direction.FORWARD);

  const halfBins = (n >> 1) + 1;
  return full.slice(0, 2 * halfBins);
}

/**
 * Inverse FFT of a half-spectrum back to a real-valued signal of length outputLength.
 */
export function irfftScalar(
  halfSpectrum: Float64Array | ArrayLike<number>,
  outputLength: number
): Float64Array {
  const n = outputLength;
  if (!Number.isFinite(n) || !Number.isInteger(n) || n < 1) {
    throw new InvalidInputError("irfft output length must be a positive integer");
  }
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `output length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }
  if (halfSpectrum.length > MAX_SAMPLES) {
    throw new InvalidInputError(
      `half_spectrum length ${halfSpectrum.length} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
    );
  }
  if (halfSpectrum.length % 2 !== 0) {
    throw new InvalidInputError(
      `irfft half-spectrum must have even length; got ${halfSpectrum.length}`
    );
  }

  const halfBins = halfSpectrum.length / 2;
  const expectedHalfBins = (n >> 1) + 1;
  if (halfBins !== expectedHalfBins) {
    throw new InvalidInputError(
      `irfft: half-spectrum has ${halfBins} bins, expected ${expectedHalfBins} for output_length = ${n}`
    );
  }

  // Validate finite floats
  for (let i = 0; i < halfSpectrum.length; i++) {
    const val = Number(halfSpectrum[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
    }
  }

  // Reconstruct full length-N spectrum using X[N - k] = conj(X[k])
  const full = new Float64Array(2 * n);
  for (let i = 0; i < 2 * halfBins; i++) {
    full[i] = Number(halfSpectrum[i]);
  }

  const reflectEnd = Math.floor((n + 1) / 2);
  for (let k = 1; k < reflectEnd; k++) {
    const srcRe = full[2 * k];
    const srcIm = full[2 * k + 1];
    const dst = n - k;
    full[2 * dst] = srcRe;
    full[2 * dst + 1] = -srcIm;
  }

  const isPow2 = (n & (n - 1)) === 0 && n >= 1;
  const inverted = isPow2
    ? ifftScalar(full)
    : bluesteinScalar(full, Direction.INVERSE);

  // Keep real components only
  const out = new Float64Array(n);
  for (let k = 0; k < n; k++) {
    out[k] = inverted[2 * k];
  }

  return out;
}
