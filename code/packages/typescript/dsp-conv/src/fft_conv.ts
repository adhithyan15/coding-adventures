import { irfftScalar, MAX_FFT_LEN, rfftScalar } from "@coding-adventures/dsp-fft";
import { MAX_KERNEL_LEN, MAX_SAMPLES } from "./constants.js";
import {
  EmptyKernelError,
  EmptySignalError,
  InvalidInputError,
  KernelTooLargeError,
  safeFormatValue,
  validateFloats,
  validateSequence,
} from "./errors.js";
import { ConvMode, parseConvMode } from "./types.js";

/**
 * Return smallest power of 2 >= n.
 */
export function nextPow2(n: number): number {
  if (!Number.isSafeInteger(n)) {
    throw new InvalidInputError(`n must be a safe integer; got ${safeFormatValue(n)}`);
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
  const n = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError);
  const k = validateSequence(kernel, "kernel", MAX_KERNEL_LEN, EmptyKernelError);

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

  let sigSpec: Float64Array;
  let kerSpec: Float64Array;
  try {
    sigSpec = rfftScalar(sigPadded);
    kerSpec = rfftScalar(kerPadded);
  } catch (err) {
    throw new InvalidInputError(
      `FFT failed during fftConvolve: ${err instanceof Error ? err.message : safeFormatValue(err)}`
    );
  }

  const numBins = (fftLen >> 1) + 1;
  const prodSpec = new Float64Array(numBins * 2);

  for (let b = 0; b < numBins; b++) {
    const reS = sigSpec[2 * b];
    const imS = sigSpec[2 * b + 1];
    const reK = kerSpec[2 * b];
    const imK = kerSpec[2 * b + 1];
    const re = reS * reK - imS * imK;
    const im = reS * imK + imS * reK;
    if (!Number.isFinite(re) || !Number.isFinite(im)) {
      throw new InvalidInputError(
        `FFT convolution product overflowed to non-finite value at bin ${b}`
      );
    }
    prodSpec[2 * b] = re;
    prodSpec[2 * b + 1] = im;
  }

  let reconstructed: Float64Array;
  try {
    reconstructed = irfftScalar(prodSpec, fftLen);
  } catch (err) {
    throw new InvalidInputError(
      `IFFT failed during fftConvolve: ${err instanceof Error ? err.message : safeFormatValue(err)}`
    );
  }
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
