import { MAX_KERNEL_LEN } from "./constants.js";
import { InvalidInputError, safeFormatValue } from "./errors.js";

/**
 * 1-D Gaussian blur kernel with standard deviation sigma and length size.
 * Normalised so the sum is 1.0. Size must be odd and positive.
 */
export function gaussianBlurKernel(sigma: number, size: number): Float64Array {
  if (!Number.isSafeInteger(size) || size <= 0 || size % 2 === 0) {
    throw new InvalidInputError(`size must be odd and > 0; got ${safeFormatValue(size)}`);
  }
  if (size > MAX_KERNEL_LEN) {
    throw new InvalidInputError(
      `size ${size} exceeds MAX_KERNEL_LEN ${MAX_KERNEL_LEN}`
    );
  }
  if (!Number.isFinite(sigma) || sigma <= 0.0) {
    throw new InvalidInputError(
      `sigma must be positive and finite; got ${safeFormatValue(sigma)}`
    );
  }

  const centre = (size - 1) / 2.0;
  const twoSigmaSq = 2.0 * sigma * sigma;
  if (twoSigmaSq <= 0.0 || !Number.isFinite(twoSigmaSq)) {
    throw new InvalidInputError(
      `sigma ${sigma} is too small or underflows precision (2*sigma^2 <= 0)`
    );
  }
  const normPdf = 1.0 / (sigma * Math.sqrt(2.0 * Math.PI));
  if (!Number.isFinite(normPdf)) {
    throw new InvalidInputError(
      `sigma ${sigma} results in non-finite normalization factor`
    );
  }

  const kernel = new Float64Array(size);
  let total = 0.0;
  for (let i = 0; i < size; i++) {
    const x = i - centre;
    const v = normPdf * Math.exp(-(x * x) / twoSigmaSq);
    kernel[i] = v;
    total += v;
  }

  if (total > 0.0) {
    for (let i = 0; i < size; i++) {
      kernel[i] /= total;
    }
  }

  return kernel;
}

/**
 * 1-D box blur (uniform) kernel of length size summing to 1.0.
 */
export function boxBlurKernel(size: number): Float64Array {
  if (!Number.isSafeInteger(size) || size <= 0) {
    throw new InvalidInputError(`size must be > 0; got ${safeFormatValue(size)}`);
  }
  if (size > MAX_KERNEL_LEN) {
    throw new InvalidInputError(
      `size ${size} exceeds MAX_KERNEL_LEN ${MAX_KERNEL_LEN}`
    );
  }
  const val = 1.0 / size;
  const kernel = new Float64Array(size);
  kernel.fill(val);
  return kernel;
}

/**
 * 3x3 Sobel kernel for horizontal edge detection (gradient in X).
 */
export function sobelXKernel(): Float64Array {
  return new Float64Array([
    -1.0, 0.0, 1.0,
    -2.0, 0.0, 2.0,
    -1.0, 0.0, 1.0,
  ]);
}

/**
 * 3x3 Sobel kernel for vertical edge detection (gradient in Y).
 */
export function sobelYKernel(): Float64Array {
  return new Float64Array([
    -1.0, -2.0, -1.0,
     0.0,  0.0,  0.0,
     1.0,  2.0,  1.0,
  ]);
}

/**
 * 3x3 discrete Laplacian kernel (second derivative).
 */
export function laplacianKernel(): Float64Array {
  return new Float64Array([
    0.0,  1.0, 0.0,
    1.0, -4.0, 1.0,
    0.0,  1.0, 0.0,
  ]);
}

/**
 * 3x3 sharpen kernel: identity + scaled negative Laplacian.
 */
export function sharpenKernel(amount: number = 1.0): Float64Array {
  if (typeof amount !== "number" || !Number.isFinite(amount)) {
    throw new InvalidInputError(`amount must be finite; got ${safeFormatValue(amount)}`);
  }
  const center = 1.0 + 4.0 * amount;
  if (!Number.isFinite(center)) {
    throw new InvalidInputError(`amount ${amount} overflows center coefficient`);
  }
  return new Float64Array([
    0.0, -amount, 0.0,
    -amount, center, -amount,
    0.0, -amount, 0.0,
  ]);
}

/** Snake-case aliases */
export const gaussian_blur_kernel = gaussianBlurKernel;
export const box_blur_kernel = boxBlurKernel;
export const sobel_x_kernel = sobelXKernel;
export const sobel_y_kernel = sobelYKernel;
export const laplacian_kernel = laplacianKernel;
export const sharpen_kernel = sharpenKernel;
