import { InvalidParamError } from "./errors.js";

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

/**
 * Parse and normalize window type from enum or case-insensitive string.
 */
export function parseWindowType(win: WindowType | string): WindowType {
  if (typeof win !== "string") {
    throw new InvalidParamError(`unsupported window type: ${String(win)}`);
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
      throw new InvalidParamError(`unsupported window type: ${win}`);
  }
}
