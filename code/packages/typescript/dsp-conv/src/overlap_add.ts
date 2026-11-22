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
import { fftConvolve, nextPow2 } from "./fft_conv.js";
import { ConvMode, parseConvMode } from "./types.js";

/**
 * Compute convolution using the overlap-add block method.
 * Purely iterative with bounded blocks (CWE-674).
 */
export function overlapAdd(
  signal: ArrayLike<number>,
  kernel: ArrayLike<number>,
  blockSize?: number | null,
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

  let blockLen: number;
  if (blockSize === null || blockSize === undefined) {
    blockLen = Math.max(64, nextPow2(k));
  } else {
    if (!Number.isSafeInteger(blockSize) || blockSize <= 0) {
      throw new InvalidInputError(
        `blockSize must be a positive integer; got ${safeFormatValue(blockSize)}`
      );
    }
    if (blockSize > MAX_SAMPLES) {
      throw new InvalidInputError(
        `blockSize ${blockSize} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
      );
    }
    blockLen = blockSize;
  }

  const fullLen = n + k - 1;
  if (fullLen > MAX_SAMPLES) {
    throw new InvalidInputError(
      `convolution output length ${fullLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }

  const out = new Float64Array(fullLen);

  let start = 0;
  while (start < n) {
    const end = Math.min(start + blockLen, n);
    const blockLenActual = end - start;
    const block = new Float64Array(blockLenActual);
    for (let b = 0; b < blockLenActual; b++) {
      block[b] = Number(signal[start + b]);
    }
    const convBlock = fftConvolve(block, kernel, ConvMode.FULL);
    for (let idx = 0; idx < convBlock.length; idx++) {
      const outPos = start + idx;
      if (outPos < fullLen) {
        out[outPos] += convBlock[idx];
      }
    }
    start += blockLen;
  }

  if (cMode === ConvMode.FULL) {
    return out;
  }
  if (cMode === ConvMode.SAME) {
    const offset = Math.floor(k / 2);
    return out.slice(offset, offset + n);
  }
  // ConvMode.VALID
  const offset = k - 1;
  return out.slice(offset, offset + (n - k + 1));
}

/** Snake-case alias */
export const overlap_add = overlapAdd;
