import { ComplexArray } from "@coding-adventures/dsp-complex";
import { bluesteinScalar } from "./bluestein.js";
import { MAX_FFT_LEN, MAX_SAMPLES } from "./constants.js";
import { Direction } from "./direction.js";
import {
  InvalidInputError,
  readFiniteNumber,
  validateArrayLikeLength,
} from "./errors.js";
import { fftScalar, ifftScalar } from "./radix2.js";
import { irfftScalar, rfftScalar } from "./rfft.js";

export interface FftOptions {
  complex?: boolean;
}

/**
 * Compute forward 1-D FFT returning a ComplexArray.
 * If signal is already a ComplexArray, performs complex-to-complex FFT.
 * If signal is a number array/Float64Array and options.complex is true, interprets as interleaved [re, im].
 * If signal is a number array/Float64Array and options.complex is false, interprets as real signal with imag=0.
 */
export function fft(
  signal: ComplexArray | Float64Array | ArrayLike<number>,
  options?: FftOptions
): ComplexArray {
  if (signal instanceof ComplexArray) {
    const n = signal.length;
    if (n < 1) {
      throw new InvalidInputError("FFT length must be at least 1");
    }
    if (n > MAX_FFT_LEN) {
      throw new InvalidInputError(
        `FFT length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
      );
    }
    const interleaved = signal.asInterleaved();
    const isPow2 = (n & (n - 1)) === 0 && n >= 1;
    const out = isPow2
      ? fftScalar(interleaved)
      : bluesteinScalar(interleaved, Direction.FORWARD);
    return ComplexArray.fromInterleaved(out);
  }

  const isComplex = options?.complex === true;
  if (isComplex) {
    const rawLen = validateArrayLikeLength(signal, "signal");
    if (rawLen % 2 !== 0) {
      throw new InvalidInputError(
        `interleaved complex buffer must have even length; got ${rawLen}`
      );
    }
    const n = rawLen / 2;
    if (n < 1) {
      throw new InvalidInputError("FFT length must be at least 1");
    }
    if (n > MAX_FFT_LEN || rawLen > MAX_SAMPLES) {
      throw new InvalidInputError(
        `signal length ${rawLen} exceeds maximum allowed`
      );
    }
    for (let i = 0; i < rawLen; i++) {
      readFiniteNumber(signal, i);
    }
    const isPow2 = (n & (n - 1)) === 0 && n >= 1;
    const out = isPow2
      ? fftScalar(signal)
      : bluesteinScalar(signal, Direction.FORWARD);
    return ComplexArray.fromInterleaved(out);
  }

  // Real signal input
  const n = validateArrayLikeLength(signal, "signal");
  if (n < 1) {
    throw new InvalidInputError("FFT length must be at least 1");
  }
  if (n > MAX_FFT_LEN || n > MAX_SAMPLES) {
    throw new InvalidInputError(`signal length ${n} exceeds maximum allowed`);
  }
  const interleaved = new Float64Array(2 * n);
  for (let i = 0; i < n; i++) {
    interleaved[2 * i] = readFiniteNumber(signal, i);
    interleaved[2 * i + 1] = 0.0;
  }
  const isPow2 = (n & (n - 1)) === 0 && n >= 1;
  const out = isPow2
    ? fftScalar(interleaved)
    : bluesteinScalar(interleaved, Direction.FORWARD);
  return ComplexArray.fromInterleaved(out);
}

/**
 * Compute inverse 1-D FFT from a ComplexArray spectrum.
 */
export function ifft(spectrum: ComplexArray): ComplexArray {
  if (!(spectrum instanceof ComplexArray)) {
    throw new InvalidInputError("spectrum must be a ComplexArray");
  }
  const n = spectrum.length;
  if (n < 1) {
    throw new InvalidInputError("FFT length must be at least 1");
  }
  if (n > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `spectrum length ${n} exceeds MAX_FFT_LEN (${MAX_FFT_LEN})`
    );
  }
  const interleaved = spectrum.asInterleaved();
  const isPow2 = (n & (n - 1)) === 0 && n >= 1;
  const out = isPow2
    ? ifftScalar(interleaved)
    : bluesteinScalar(interleaved, Direction.INVERSE);
  return ComplexArray.fromInterleaved(out);
}

/**
 * Compute real-input FFT returning a ComplexArray half-spectrum of length floor(N/2) + 1.
 */
export function rfft(
  signal: Float64Array | ArrayLike<number>
): ComplexArray {
  const halfInterleaved = rfftScalar(signal);
  return ComplexArray.fromInterleaved(halfInterleaved);
}

/**
 * Compute inverse real-input FFT from a ComplexArray half-spectrum back to real signal.
 */
export function irfft(
  halfSpectrum: ComplexArray | Float64Array | ArrayLike<number>,
  outputLength: number
): Float64Array {
  if (halfSpectrum instanceof ComplexArray) {
    return irfftScalar(halfSpectrum.asInterleaved(), outputLength);
  }
  return irfftScalar(halfSpectrum, outputLength);
}
