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
import {
  BoundaryMode,
  ConvMode,
  parseBoundaryMode,
  parseConvMode,
} from "./types.js";

/**
 * Map a possibly-out-of-bounds index into [0, n) per boundary mode,
 * or return null for Zero mode when outside support.
 */
export function extendIndex(
  idx: number,
  n: number,
  mode: BoundaryMode
): number | null {
  if (!Number.isSafeInteger(idx)) {
    throw new InvalidInputError(`index must be a safe integer; got ${safeFormatValue(idx)}`);
  }
  if (!Number.isSafeInteger(n) || n <= 0) {
    throw new InvalidInputError(`signal length n must be a positive safe integer; got ${safeFormatValue(n)}`);
  }
  if (
    mode !== BoundaryMode.ZERO &&
    mode !== BoundaryMode.REPLICATE &&
    mode !== BoundaryMode.REFLECT &&
    mode !== BoundaryMode.WRAP
  ) {
    throw new InvalidInputError(`unknown boundary mode: ${safeFormatValue(mode)}`);
  }
  if (idx >= 0 && idx < n) {
    return idx;
  }
  switch (mode) {
    case BoundaryMode.ZERO:
      return null;
    case BoundaryMode.REPLICATE:
      return Math.max(0, Math.min(n - 1, idx));
    case BoundaryMode.REFLECT: {
      if (n === 1) {
        return 0;
      }
      const period = 2 * (n - 1);
      let m = ((idx % period) + period) % period;
      if (m >= n) {
        m = period - m;
      }
      return m;
    }
    case BoundaryMode.WRAP: {
      return ((idx % n) + n) % n;
    }
  }
}

/**
 * Read signal at a possibly out-of-bounds index per the chosen boundary mode.
 */
export function sample(
  signal: ArrayLike<number>,
  idx: number,
  mode: BoundaryMode
): number {
  const n = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError);
  const mapped = extendIndex(idx, n, mode);
  if (mapped === null) {
    return 0.0;
  }
  const val: unknown = signal[mapped];
  if (typeof val !== "number" || !Number.isFinite(val)) {
    throw new InvalidInputError(
      `non-finite float in signal at index ${mapped}: ${safeFormatValue(val)}`
    );
  }
  return val;
}

/**
 * Same-size 1-D convolution.
 *
 * Output length equals signal.length. The kernel is centred at
 * Math.floor(kernel.length / 2) (matching scipy.ndimage.convolve and Rust dsp-conv).
 */
export function conv1d(
  signal: ArrayLike<number>,
  kernel: ArrayLike<number>,
  mode: BoundaryMode | string = BoundaryMode.ZERO
): Float64Array {
  const n = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError);
  const k = validateSequence(kernel, "kernel", MAX_KERNEL_LEN, EmptyKernelError);

  const bMode = parseBoundaryMode(mode);
  validateFloats(signal, "signal");
  validateFloats(kernel, "kernel");

  const centre = Math.floor(k / 2);
  const out = new Float64Array(n);

  for (let i = 0; i < n; i++) {
    let acc = 0.0;
    for (let j = 0; j < k; j++) {
      const srcIdx = i + centre - j;
      acc += Number(kernel[j]) * sample(signal, srcIdx, bMode);
    }
    out[i] = acc;
  }

  return out;
}

/**
 * 1-D discrete convolution supporting Full, Same, and Valid shape modes.
 */
export function convolve(
  signal: ArrayLike<number>,
  kernel: ArrayLike<number>,
  mode: ConvMode | string = ConvMode.FULL,
  boundary: BoundaryMode | string = BoundaryMode.ZERO
): Float64Array {
  const n = validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError);
  const k = validateSequence(kernel, "kernel", MAX_KERNEL_LEN, EmptyKernelError);

  const cMode = parseConvMode(mode);
  const bMode = parseBoundaryMode(boundary);
  validateFloats(signal, "signal");
  validateFloats(kernel, "kernel");

  if (cMode === ConvMode.SAME) {
    return conv1d(signal, kernel, bMode);
  }

  if (cMode === ConvMode.VALID) {
    if (n < k) {
      throw new KernelTooLargeError(
        `kernel length ${k} exceeds signal length ${n} in valid mode`
      );
    }
    const outLen = n - k + 1;
    const out = new Float64Array(outLen);
    for (let i = 0; i < outLen; i++) {
      let acc = 0.0;
      for (let j = 0; j < k; j++) {
        acc += Number(kernel[j]) * Number(signal[i + k - 1 - j]);
      }
      out[i] = acc;
    }
    return out;
  }

  // Full convolution mode
  const outLen = n + k - 1;
  if (outLen > MAX_SAMPLES) {
    throw new InvalidInputError(
      `full conv length ${outLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }

  const out = new Float64Array(outLen);
  for (let i = 0; i < outLen; i++) {
    let acc = 0.0;
    for (let j = 0; j < k; j++) {
      const srcIdx = i - j;
      acc += Number(kernel[j]) * sample(signal, srcIdx, bMode);
    }
    out[i] = acc;
  }
  return out;
}

/**
 * 1-D discrete cross-correlation.
 * (signal * kernel)(n) = sum_k signal[k] * kernel[k - n].
 */
export function correlate(
  signal: ArrayLike<number>,
  kernel: ArrayLike<number>,
  mode: ConvMode | string = ConvMode.FULL
): Float64Array {
  validateSequence(signal, "signal", MAX_SAMPLES, EmptySignalError);
  const k = validateSequence(kernel, "kernel", MAX_KERNEL_LEN, EmptyKernelError);
  validateFloats(kernel, "kernel");
  const reversedKernel = new Float64Array(k);
  for (let i = 0; i < k; i++) {
    reversedKernel[i] = Number(kernel[k - 1 - i]);
  }
  return convolve(signal, reversedKernel, mode, BoundaryMode.ZERO);
}
