import { MAX_KERNEL_LEN, MAX_SAMPLES } from "./constants.js";
import { conv1d, extendIndex } from "./conv1d.js";
import {
  EmptyKernelError,
  EmptySignalError,
  ImageSizeMismatchError,
  InvalidInputError,
  KernelTooLargeError,
  validateFloats,
} from "./errors.js";
import {
  BoundaryMode,
  ConvMode,
  parseBoundaryMode,
  parseConvMode,
} from "./types.js";

/**
 * Same-size 2-D convolution on a row-major [H, W] real image.
 * Output is a same-size [H, W] row-major Float64Array.
 * Kernel is centred at (kernelHeight // 2, kernelWidth // 2).
 */
export function conv2d(
  image: ArrayLike<number>,
  kernel: ArrayLike<number>,
  imageHeight: number,
  imageWidth: number,
  kernelHeight: number,
  kernelWidth: number,
  mode: BoundaryMode | string = BoundaryMode.ZERO
): Float64Array {
  if (
    !Number.isSafeInteger(imageHeight) ||
    !Number.isSafeInteger(imageWidth) ||
    imageHeight <= 0 ||
    imageWidth <= 0
  ) {
    throw new ImageSizeMismatchError(
      `image dimensions must be positive safe integers; got ${imageHeight}x${imageWidth}`
    );
  }
  if (
    !Number.isSafeInteger(kernelHeight) ||
    !Number.isSafeInteger(kernelWidth) ||
    kernelHeight <= 0 ||
    kernelWidth <= 0
  ) {
    throw new EmptyKernelError("kernel dimensions must be positive safe integers");
  }

  const h = imageHeight;
  const w = imageWidth;
  const kh = kernelHeight;
  const kw = kernelWidth;

  const expectedImageLen = h * w;
  if (expectedImageLen > MAX_SAMPLES) {
    throw new InvalidInputError(
      `image size ${expectedImageLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }
  if (image.length !== expectedImageLen) {
    throw new ImageSizeMismatchError(
      `image length ${image.length} does not match ${h}x${w} = ${expectedImageLen}`
    );
  }

  const expectedKernelLen = kh * kw;
  if (expectedKernelLen > MAX_KERNEL_LEN) {
    throw new InvalidInputError(
      `kernel size ${expectedKernelLen} exceeds MAX_KERNEL_LEN ${MAX_KERNEL_LEN}`
    );
  }
  if (kernel.length !== expectedKernelLen) {
    throw new ImageSizeMismatchError(
      `kernel length ${kernel.length} does not match ${kh}x${kw} = ${expectedKernelLen}`
    );
  }

  const bMode = parseBoundaryMode(mode);
  validateFloats(image, "image");
  validateFloats(kernel, "kernel");

  if (kh > h || kw > w) {
    throw new KernelTooLargeError(`kernel ${kh}x${kw} exceeds image ${h}x${w}`);
  }

  const ch = Math.floor(kh / 2);
  const cw = Math.floor(kw / 2);

  const out = new Float64Array(expectedImageLen);

  for (let r = 0; r < h; r++) {
    const rOffset = r * w;
    for (let c = 0; c < w; c++) {
      let acc = 0.0;
      for (let kr = 0; kr < kh; kr++) {
        const srcR = r + ch - kr;
        const row = extendIndex(srcR, h, bMode);
        if (row === null) {
          continue;
        }
        const rowOffset = row * w;
        for (let kc = 0; kc < kw; kc++) {
          const srcC = c + cw - kc;
          const col = extendIndex(srcC, w, bMode);
          if (col === null) {
            continue;
          }
          acc +=
            Number(kernel[kr * kw + kc]) * Number(image[rowOffset + col]);
        }
      }
      out[rOffset + c] = acc;
    }
  }

  return out;
}

/**
 * Separable 2-D convolution: row-pass then column-pass in O(H * W * (KH + KW)).
 */
export function sepConv2d(
  image: ArrayLike<number>,
  horizontalKernel: ArrayLike<number>,
  verticalKernel: ArrayLike<number>,
  imageHeight: number,
  imageWidth: number,
  mode: BoundaryMode | string = BoundaryMode.ZERO
): Float64Array {
  if (
    !Number.isSafeInteger(imageHeight) ||
    !Number.isSafeInteger(imageWidth) ||
    imageHeight <= 0 ||
    imageWidth <= 0
  ) {
    throw new ImageSizeMismatchError(
      `image dimensions must be positive safe integers; got ${imageHeight}x${imageWidth}`
    );
  }
  if (horizontalKernel.length === 0 || verticalKernel.length === 0) {
    throw new EmptyKernelError("separable kernels must be non-empty");
  }

  const h = imageHeight;
  const w = imageWidth;
  const expectedImageLen = h * w;
  if (expectedImageLen > MAX_SAMPLES) {
    throw new InvalidInputError(
      `image size ${expectedImageLen} exceeds MAX_SAMPLES ${MAX_SAMPLES}`
    );
  }
  if (image.length !== expectedImageLen) {
    throw new ImageSizeMismatchError(
      `image length ${image.length} does not match ${h}x${w} = ${expectedImageLen}`
    );
  }
  if (
    horizontalKernel.length > MAX_KERNEL_LEN ||
    verticalKernel.length > MAX_KERNEL_LEN
  ) {
    throw new InvalidInputError(
      `separable kernel length exceeds MAX_KERNEL_LEN ${MAX_KERNEL_LEN}`
    );
  }

  const bMode = parseBoundaryMode(mode);
  validateFloats(image, "image");
  validateFloats(horizontalKernel, "horizontalKernel");
  validateFloats(verticalKernel, "verticalKernel");

  // Pass 1: convolve each row horizontally
  const intermediate = new Float64Array(expectedImageLen);
  const rowBuf = new Float64Array(w);
  for (let r = 0; r < h; r++) {
    const rowStart = r * w;
    for (let c = 0; c < w; c++) {
      rowBuf[c] = Number(image[rowStart + c]);
    }
    const convRow = conv1d(rowBuf, horizontalKernel, bMode);
    for (let c = 0; c < w; c++) {
      intermediate[rowStart + c] = convRow[c];
    }
  }

  // Pass 2: convolve each column vertically
  const out = new Float64Array(expectedImageLen);
  const colBuf = new Float64Array(h);
  for (let c = 0; c < w; c++) {
    for (let r = 0; r < h; r++) {
      colBuf[r] = intermediate[r * w + c];
    }
    const convCol = conv1d(colBuf, verticalKernel, bMode);
    for (let r = 0; r < h; r++) {
      out[r * w + c] = convCol[r];
    }
  }

  return out;
}

/**
 * Flexible 2-D discrete convolution supporting flat and 2-D nested arrays.
 */
export function convolve2d(
  image: ArrayLike<number> | ArrayLike<number>[],
  kernel: ArrayLike<number> | ArrayLike<number>[],
  mode: ConvMode | string = ConvMode.FULL,
  boundary: BoundaryMode | string = BoundaryMode.ZERO,
  imageHeight?: number,
  imageWidth?: number,
  kernelHeight?: number,
  kernelWidth?: number
): Float64Array | number[][] {
  if (image.length === 0) {
    throw new EmptySignalError("image must be non-empty");
  }
  if (kernel.length === 0) {
    throw new EmptyKernelError("kernel must be non-empty");
  }

  const firstImgItem = (image as any)[0];
  const is2dNested =
    Array.isArray(firstImgItem) ||
    (firstImgItem && typeof firstImgItem === "object" && "length" in firstImgItem);

  let h: number;
  let w: number;
  if (imageHeight !== undefined || imageWidth !== undefined) {
    if (imageHeight === undefined || imageWidth === undefined) {
      throw new ImageSizeMismatchError(
        "imageHeight and imageWidth required for flat 1-D image"
      );
    }
    h = imageHeight;
    w = imageWidth;
  } else if (is2dNested) {
    h = image.length;
    w = (firstImgItem as ArrayLike<number>).length;
    for (let r = 0; r < h; r++) {
      const row = (image as ArrayLike<number>[])[r];
      if (!row || row.length !== w) {
        throw new ImageSizeMismatchError("jagged 2-D image array");
      }
    }
  } else {
    throw new ImageSizeMismatchError(
      "imageHeight and imageWidth required for flat 1-D image"
    );
  }

  if (h <= 0 || w <= 0 || !Number.isSafeInteger(h) || !Number.isSafeInteger(w)) {
    throw new ImageSizeMismatchError("image dimensions must be positive safe integers");
  }
  const imgLen = h * w;
  if (imgLen > MAX_SAMPLES) {
    throw new InvalidInputError("image size exceeds MAX_SAMPLES");
  }
  if (!is2dNested && image.length !== imgLen) {
    throw new ImageSizeMismatchError(
      `image length ${image.length} != imageHeight * imageWidth (${imgLen})`
    );
  }

  const firstKerItem = (kernel as any)[0];
  const isKer2d =
    Array.isArray(firstKerItem) ||
    (firstKerItem && typeof firstKerItem === "object" && "length" in firstKerItem);

  let kh: number;
  let kw: number;
  if (kernelHeight !== undefined || kernelWidth !== undefined) {
    if (kernelHeight === undefined || kernelWidth === undefined) {
      throw new ImageSizeMismatchError(
        "kernelHeight and kernelWidth required for flat 1-D kernel"
      );
    }
    kh = kernelHeight;
    kw = kernelWidth;
  } else if (isKer2d) {
    kh = kernel.length;
    kw = (firstKerItem as ArrayLike<number>).length;
    for (let r = 0; r < kh; r++) {
      const row = (kernel as ArrayLike<number>[])[r];
      if (!row || row.length !== kw) {
        throw new ImageSizeMismatchError("jagged 2-D kernel array");
      }
    }
  } else {
    throw new ImageSizeMismatchError(
      "kernelHeight and kernelWidth required for flat 1-D kernel"
    );
  }

  if (kh <= 0 || kw <= 0 || !Number.isSafeInteger(kh) || !Number.isSafeInteger(kw)) {
    throw new EmptyKernelError("kernel dimensions must be positive safe integers");
  }
  const kerLen = kh * kw;
  if (kerLen > MAX_KERNEL_LEN) {
    throw new InvalidInputError("kernel size exceeds MAX_KERNEL_LEN");
  }
  if (!isKer2d && kernel.length !== kerLen) {
    throw new ImageSizeMismatchError(
      `kernel length ${kernel.length} != kernelHeight * kernelWidth (${kerLen})`
    );
  }

  const cMode = parseConvMode(mode);
  const bMode = parseBoundaryMode(boundary);

  let outH: number;
  let outW: number;
  if (cMode === ConvMode.SAME) {
    outH = h;
    outW = w;
  } else if (cMode === ConvMode.VALID) {
    if (h < kh || w < kw) {
      throw new KernelTooLargeError(
        `kernel ${kh}x${kw} exceeds image ${h}x${w} in valid mode`
      );
    }
    outH = h - kh + 1;
    outW = w - kw + 1;
  } else {
    // FULL
    outH = h + kh - 1;
    outW = w + kw - 1;
  }

  const outLen = outH * outW;
  if (outLen > MAX_SAMPLES) {
    throw new InvalidInputError("output size exceeds MAX_SAMPLES");
  }

  // Flatten inputs into typed Float64Arrays
  const flatImg = new Float64Array(imgLen);
  if (is2dNested) {
    let idx = 0;
    for (let r = 0; r < h; r++) {
      const row = (image as ArrayLike<number>[])[r];
      for (let c = 0; c < w; c++) {
        flatImg[idx++] = Number(row[c]);
      }
    }
  } else {
    for (let i = 0; i < imgLen; i++) {
      flatImg[i] = Number((image as ArrayLike<number>)[i]);
    }
  }
  validateFloats(flatImg, "image");

  const flatKer = new Float64Array(kerLen);
  if (isKer2d) {
    let idx = 0;
    for (let r = 0; r < kh; r++) {
      const row = (kernel as ArrayLike<number>[])[r];
      for (let c = 0; c < kw; c++) {
        flatKer[idx++] = Number(row[c]);
      }
    }
  } else {
    for (let i = 0; i < kerLen; i++) {
      flatKer[i] = Number((kernel as ArrayLike<number>)[i]);
    }
  }
  validateFloats(flatKer, "kernel");

  let resultFlat: Float64Array;
  if (cMode === ConvMode.SAME) {
    resultFlat = conv2d(flatImg, flatKer, h, w, kh, kw, bMode);
  } else if (cMode === ConvMode.VALID) {
    resultFlat = new Float64Array(outLen);
    for (let r = 0; r < outH; r++) {
      for (let c = 0; c < outW; c++) {
        let acc = 0.0;
        for (let kr = 0; kr < kh; kr++) {
          for (let kc = 0; kc < kw; kc++) {
            acc +=
              flatKer[kr * kw + kc] *
              flatImg[(r + kh - 1 - kr) * w + (c + kw - 1 - kc)];
          }
        }
        resultFlat[r * outW + c] = acc;
      }
    }
  } else {
    // FULL
    resultFlat = new Float64Array(outLen);
    for (let r = 0; r < outH; r++) {
      for (let c = 0; c < outW; c++) {
        let acc = 0.0;
        for (let kr = 0; kr < kh; kr++) {
          const srcR = r - kr;
          const row = extendIndex(srcR, h, bMode);
          if (row === null) {
            continue;
          }
          const rowOffset = row * w;
          for (let kc = 0; kc < kw; kc++) {
            const srcC = c - kc;
            const col = extendIndex(srcC, w, bMode);
            if (col === null) {
              continue;
            }
            acc += flatKer[kr * kw + kc] * flatImg[rowOffset + col];
          }
        }
        resultFlat[r * outW + c] = acc;
      }
    }
  }

  if (is2dNested) {
    const nested: number[][] = [];
    for (let r = 0; r < outH; r++) {
      const rowArr: number[] = [];
      const offset = r * outW;
      for (let c = 0; c < outW; c++) {
        rowArr.push(resultFlat[offset + c]);
      }
      nested.push(rowArr);
    }
    return nested;
  }

  return resultFlat;
}

/** Snake-case aliases */
export const sep_conv2d = sepConv2d;
export const convolve_2d = convolve2d;
