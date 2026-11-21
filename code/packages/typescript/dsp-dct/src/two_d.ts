import { MAX_DCT_LEN, MAX_SAMPLES } from "./constants.js";
import { dct, idct } from "./dct.js";
import {
  EmptyInputError,
  InvalidInputError,
  readFiniteNumber,
  validateArrayLikeLength,
} from "./errors.js";
import { DctNorm, DctType } from "./types.js";

function validate2d(
  image: Float64Array | ArrayLike<number>,
  height: number,
  width: number
): void {
  const imgLen = validateArrayLikeLength(image, "image");
  if (imgLen === 0) {
    throw new EmptyInputError("input image must not be empty");
  }
  if (typeof height !== "number" || !Number.isSafeInteger(height) || height < 1) {
    throw new InvalidInputError("height must be a positive integer");
  }
  if (typeof width !== "number" || !Number.isSafeInteger(width) || width < 1) {
    throw new InvalidInputError("width must be a positive integer");
  }
  if (height > MAX_DCT_LEN) {
    throw new InvalidInputError(
      `height ${height} exceeds MAX_DCT_LEN (${MAX_DCT_LEN})`
    );
  }
  if (width > MAX_DCT_LEN) {
    throw new InvalidInputError(
      `width ${width} exceeds MAX_DCT_LEN (${MAX_DCT_LEN})`
    );
  }
  const total = height * width;
  if (imgLen !== total) {
    throw new InvalidInputError(
      `image length ${imgLen} does not match height * width (${height} * ${width} = ${total})`
    );
  }
  if (total > MAX_SAMPLES) {
    throw new InvalidInputError(
      `total samples ${total} exceeds MAX_SAMPLES (${MAX_SAMPLES})`
    );
  }
  for (let i = 0; i < total; i++) {
    readFiniteNumber(image, i);
  }
}

/**
 * 2-D Discrete Cosine Transform (separable row-column application).
 */
export function dct2d(
  image: Float64Array | ArrayLike<number>,
  height: number,
  width: number,
  dctType?: DctType | number | string,
  norm?: DctNorm | string | number
): Float64Array {
  validate2d(image, height, width);

  const rowTransformed = new Float64Array(height * width);
  const rowBuf = new Float64Array(width);

  // Transform each row
  for (let r = 0; r < height; r++) {
    const rowOffset = r * width;
    for (let c = 0; c < width; c++) {
      rowBuf[c] = Number(image[rowOffset + c]);
    }
    const rowRes = dct(rowBuf, dctType, norm);
    for (let c = 0; c < width; c++) {
      rowTransformed[rowOffset + c] = rowRes[c];
    }
  }

  // Transform each column
  const out = new Float64Array(height * width);
  const colBuf = new Float64Array(height);
  for (let c = 0; c < width; c++) {
    for (let r = 0; r < height; r++) {
      colBuf[r] = rowTransformed[r * width + c];
    }
    const colRes = dct(colBuf, dctType, norm);
    for (let r = 0; r < height; r++) {
      out[r * width + c] = colRes[r];
    }
  }

  return out;
}

/**
 * 2-D Inverse Discrete Cosine Transform.
 */
export function idct2d(
  coeffs: Float64Array | ArrayLike<number>,
  height: number,
  width: number,
  dctType?: DctType | number | string,
  norm?: DctNorm | string | number
): Float64Array {
  validate2d(coeffs, height, width);

  // Invert columns first
  const colTransformed = new Float64Array(height * width);
  const colBuf = new Float64Array(height);
  for (let c = 0; c < width; c++) {
    for (let r = 0; r < height; r++) {
      colBuf[r] = Number(coeffs[r * width + c]);
    }
    const colRes = idct(colBuf, dctType, norm);
    for (let r = 0; r < height; r++) {
      colTransformed[r * width + c] = colRes[r];
    }
  }

  // Invert rows
  const out = new Float64Array(height * width);
  const rowBuf = new Float64Array(width);
  for (let r = 0; r < height; r++) {
    const rowOffset = r * width;
    for (let c = 0; c < width; c++) {
      rowBuf[c] = colTransformed[rowOffset + c];
    }
    const rowRes = idct(rowBuf, dctType, norm);
    for (let c = 0; c < width; c++) {
      out[rowOffset + c] = rowRes[c];
    }
  }

  return out;
}

export const dct_2d = dct2d;
export const idct_2d = idct2d;
