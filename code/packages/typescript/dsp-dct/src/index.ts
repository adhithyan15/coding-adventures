/**
 * @coding-adventures/dsp-dct
 *
 * Discrete Cosine Transform (DCT) and Discrete Sine Transform (DST) for the DSP layer.
 */

export { MAX_DCT_LEN, MAX_SAMPLES } from "./constants.js";
export {
  dct,
  dctI,
  dctIiViaFft,
  dctIiiViaFft,
  dctIv,
  idct,
  validateSignal,
} from "./dct.js";
export {
  dst,
  dstI,
  dstIi,
  dstIii,
  dstIv,
  DstType,
  idst,
  parseDstType,
} from "./dst.js";
export {
  DctError,
  EmptyInputError,
  InvalidInputError,
} from "./errors.js";
export {
  dct2d,
  dct_2d,
  idct2d,
  idct_2d,
} from "./two_d.js";
export {
  DctNorm,
  DctType,
  parseNorm,
  parseType,
} from "./types.js";
