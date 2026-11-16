import { irfftScalar, MAX_FFT_LEN, rfftScalar } from "@coding-adventures/dsp-fft";
import { MAX_KERNEL_LEN, MAX_SAMPLES } from "./constants.js";
import {
  EmptyKernelError,
  EmptySignalError,
  InvalidInputError,
  KernelTooLargeError,
  validateFloats,
} from "./errors.js";
import { ConvMode, parseConvMode } from "./types.js";

/**
 * Return smallest power of 2 >= n.
 */
export function nextPow2(n: number): number {
  if (!Number.isSafeInteger(n)) {
    throw new InvalidInputError(`n must be a safe integer; got ${n}`);
  }
  if (n <= 1) {
    return 1;
  }
  let p = 1;
  while (p < n) {
    p <<= 1;
    if (p <= 0) {
      throw new InvalidInputError("nextPow2 integer overflow");
    }
  }
  return p;
}

/**
 * Compute 1-D convolution via FFT with O(N log N) complexity.
 */
export function fftConvolve(
  signal: ArrayLike<number>,
  kernel: ArrayLike<number>,
  mode: ConvMode | string = ConvMode.FULL
): Float64Array {
  const n = signal.length;
  const k = kernel.length;

  if (n === 0) {
    throw new EmptySignalError("signal must be non-empty");
  }
  if (k === 0) {
    throw new EmptyKernelError("kernel must be non-empty");
  }
  if (n > MAX_SAMPLES) {
    throw new InvalidInputError(
      `signal length ${n} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }
  if (k > MAX_KERNEL_LEN) {
    throw new InvalidInputError(
      `kernel length ${k} exceeds MAX_KERNEL_LEN ${MAX_KERNEL_LEN}`
    );
  }

  const cMode = parseConvMode(mode);
  validateFloats(signal, "signal");
  validateFloats(kernel, "kernel");

  if (cMode === ConvMode.VALID && n < k) {
    throw new KernelTooLargeError(
      `kernel length ${k} exceeds signal length ${n} in valid mode`
    );
  }

  const fullLen = n + k - 1;
  if (fullLen > MAX_SAMPLES) {
    throw new InvalidInputError(
      `convolution output length ${fullLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }

  const fftLen = nextPow2(fullLen);
  if (fftLen > MAX_FFT_LEN) {
    throw new InvalidInputError(
      `required FFT length ${fftLen} exceeds MAX_FFT_LEN ${MAX_FFT_LEN}`
    );
  }

  const sigPadded = new Float64Array(fftLen);
  for (let i = 0; i < n; i++) {
    sigPadded[i] = Number(signal[i]);
  }

  const kerPadded = new Float64Array(fftLen);
  for (let j = 0; j < k; j++) {
    kerPadded[j] = Number(kernel[j]);
  }

  const sigSpec = rfftScalar(sigPadded);
  const kerSpec = rfftScalar(kerPadded);

  const numBins = (fftLen >> 1) + 1;
  const prodSpec = new Float64Array(numBins * 2);

  for (let b = 0; b < numBins; b++) {
    const reS = sigSpec[2 * b];
    const imS = sigSpec[2 * b + 1];
    const reK = kerSpec[2 * b];
    const imK = kerSpec[2 * b + 1];
    prodSpec[2 * b] = reS * reK - imS * imK;
    prodSpec[2 * b + 1] = reS * imK + imS * reK;
  }

  const reconstructed = irfftScalar(prodSpec, fftLen);
  const fullResult = reconstructed.slice(0, fullLen);

  if (cMode === ConvMode.FULL) {
    return fullResult;
  }
  if (cMode === ConvMode.SAME) {
    const start = Math.floor(k / 2);
    return fullResult.slice(start, start + n);
  }
  // ConvMode.VALID
  const start = k - 1;
  return fullResult.slice(start, start + (n - k + 1));
}

/** Snake-case alias matching Python and Rust contracts */
export const fft_convolve = fftConvolve;
