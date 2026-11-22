import { InvalidParamError, safeFormatValue } from "./errors.js";

/**
 * Analysis window function types.
 */
export enum WindowType {
  HANN = "hann",
  HAMMING = "hamming",
  BLACKMAN = "blackman",
  RECTANGULAR = "rectangular",
}

export const Hann = WindowType.HANN;
export const Hamming = WindowType.HAMMING;
export const Blackman = WindowType.BLACKMAN;
export const Rectangular = WindowType.RECTANGULAR;

const MAX_WINDOW_STR_LEN = 64;

/**
 * Parse and normalize window type from enum or case-insensitive string.
 */
export function parseWindowType(win: WindowType | string): WindowType {
  if (typeof win !== "string" || win.length > MAX_WINDOW_STR_LEN) {
    throw new InvalidParamError(`unsupported window type: ${safeFormatValue(win)}`);
  }
  const norm = win.trim().toLowerCase();
  switch (norm) {
    case "hann":
    case "hanning":
      return WindowType.HANN;
    case "hamming":
      return WindowType.HAMMING;
    case "blackman":
      return WindowType.BLACKMAN;
    case "rectangular":
    case "rect":
    case "boxcar":
      return WindowType.RECTANGULAR;
    default:
      throw new InvalidParamError(`unsupported window type: ${safeFormatValue(win)}`);
  }
}
