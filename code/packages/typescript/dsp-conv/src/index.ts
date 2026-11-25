/**
 * @coding-adventures/dsp-conv
 *
 * Discrete convolution and image filtering algorithms for the DSP layer.
 */

export { MAX_KERNEL_LEN, MAX_SAMPLES, MAX_SIGNAL_LEN } from "./constants.js";
export {
  conv1d,
  convolve,
  correlate,
  extendIndex,
  sample,
} from "./conv1d.js";
export {
  ConvError,
  EmptyKernelError,
  EmptySignalError,
  ImageSizeMismatchError,
  InvalidInputError,
  KernelTooLargeError,
  safeFormatValue,
  validateFloats,
  validateSequence,
} from "./errors.js";
export {
  fftConvolve,
  fft_convolve,
  nextPow2,
} from "./fft_conv.js";
export {
  boxBlurKernel,
  box_blur_kernel,
  gaussianBlurKernel,
  gaussian_blur_kernel,
  laplacianKernel,
  laplacian_kernel,
  sharpenKernel,
  sharpen_kernel,
  sobelXKernel,
  sobel_x_kernel,
  sobelYKernel,
  sobel_y_kernel,
} from "./kernels.js";
export {
  overlapAdd,
  overlap_add,
} from "./overlap_add.js";
export {
  conv2d,
  convolve2d,
  convolve_2d,
  sepConv2d,
  sep_conv2d,
} from "./two_d.js";
export {
  BoundaryMode,
  ConvMode,
  Full,
  Replicate,
  Reflect,
  Same,
  Valid,
  Wrap,
  Zero,
  parseBoundaryMode,
  parseConvMode,
} from "./types.js";
