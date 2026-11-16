import { irfftScalar, rfftScalar } from "@coding-adventures/dsp-fft";
import { MAX_FILTER_TAPS, MAX_SIGNAL_LEN } from "./constants.js";
import {
  EmptyKernelError,
  EmptySignalError,
  InvalidCoefficientError,
} from "./errors.js";

function validateFirInputs(
  signal: Float64Array | ArrayLike<number>,
  kernel: Float64Array | ArrayLike<number>
): [number, number, number] {
  const n = signal.length;
  if (n === 0) {
    throw new EmptySignalError("filter signal must be non-empty");
  }
  const k = kernel.length;
  if (k === 0) {
    throw new EmptyKernelError("filter kernel must be non-empty");
  }

  if (n > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `signal length ${n} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }
  if (k > MAX_FILTER_TAPS) {
    throw new InvalidCoefficientError(
      `kernel length ${k} exceeds MAX_FILTER_TAPS (${MAX_FILTER_TAPS})`
    );
  }

  const outLen = n + k - 1;
  if (outLen > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `convolution output length ${outLen} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }

  for (let i = 0; i < n; i++) {
    const val = Number(signal[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidCoefficientError("non-finite float in signal");
    }
  }
  for (let i = 0; i < k; i++) {
    const val = Number(kernel[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidCoefficientError("non-finite float in kernel");
    }
  }

  return [n, k, outLen];
}

/**
 * FIR filtering via direct linear convolution (mode='full').
 * Output length is len(signal) + len(kernel) - 1.
 */
export function fir(
  signal: Float64Array | ArrayLike<number>,
  kernel: Float64Array | ArrayLike<number>
): Float64Array {
  const [n, k, outLen] = validateFirInputs(signal, kernel);

  const out = new Float64Array(outLen);
  for (let i = 0; i < outLen; i++) {
    const jLo = Math.max(0, i + 1 - n);
    const jHi = Math.min(k, i + 1);
    let acc = 0.0;
    for (let j = jLo; j < jHi; j++) {
      acc += Number(kernel[j]) * Number(signal[i - j]);
    }
    out[i] = acc;
  }

  return out;
}

/**
 * FIR filtering via FFT-accelerated fast convolution.
 * Returns the exact linear convolution matching fir(signal, kernel).
 */
export function firFft(
  signal: Float64Array | ArrayLike<number>,
  kernel: Float64Array | ArrayLike<number>
): Float64Array {
  const [n, k, outLen] = validateFirInputs(signal, kernel);

  let fftLen = 2;
  while (fftLen < outLen) {
    fftLen <<= 1;
  }
  if (fftLen > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `FFT length ${fftLen} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }

  const sigPad = new Float64Array(fftLen);
  for (let i = 0; i < n; i++) {
    sigPad[i] = Number(signal[i]);
  }
  const kerPad = new Float64Array(fftLen);
  for (let i = 0; i < k; i++) {
    kerPad[i] = Number(kernel[i]);
  }

  const sigSpec = rfftScalar(sigPad);
  const kerSpec = rfftScalar(kerPad);

  const bins = (fftLen >> 1) + 1;
  const prod = new Float64Array(2 * bins);
  for (let b = 0; b < bins; b++) {
    const r1 = sigSpec[2 * b];
    const i1 = sigSpec[2 * b + 1];
    const r2 = kerSpec[2 * b];
    const i2 = kerSpec[2 * b + 1];
    prod[2 * b] = r1 * r2 - i1 * i2;
    prod[2 * b + 1] = r1 * i2 + i1 * r2;
  }

  const conv = irfftScalar(prod, fftLen);
  return conv.slice(0, outLen);
}

export const fir_fft = firFft;
