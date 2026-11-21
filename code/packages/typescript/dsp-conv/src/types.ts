import { InvalidInputError, safeFormatValue } from "./errors.js";

/**
 * How to extend the signal outside [0, N) support.
 */
export enum BoundaryMode {
  ZERO = "zero",
  REPLICATE = "replicate",
  REFLECT = "reflect",
  WRAP = "wrap",
}

export const Zero = BoundaryMode.ZERO;
export const Replicate = BoundaryMode.REPLICATE;
export const Reflect = BoundaryMode.REFLECT;
export const Wrap = BoundaryMode.WRAP;

/**
 * Shape of convolution output.
 */
export enum ConvMode {
  FULL = "full",
  SAME = "same",
  VALID = "valid",
}

export const Full = ConvMode.FULL;
export const Same = ConvMode.SAME;
export const Valid = ConvMode.VALID;

const MAX_MODE_STR_LEN = 64;

/**
 * Parse and normalize boundary mode from enum or case-insensitive string.
 */
export function parseBoundaryMode(mode: BoundaryMode | string): BoundaryMode {
  if (typeof mode !== "string" || mode.length > MAX_MODE_STR_LEN) {
    throw new InvalidInputError(`unsupported boundary mode: ${safeFormatValue(mode)}`);
  }
  const norm = mode.trim().toLowerCase();
  switch (norm) {
    case "zero":
      return BoundaryMode.ZERO;
    case "replicate":
      return BoundaryMode.REPLICATE;
    case "reflect":
      return BoundaryMode.REFLECT;
    case "wrap":
      return BoundaryMode.WRAP;
    default:
      throw new InvalidInputError(`unsupported boundary mode: ${safeFormatValue(mode)}`);
  }
}

/**
 * Parse and normalize convolution mode from enum or case-insensitive string.
 */
export function parseConvMode(mode: ConvMode | string): ConvMode {
  if (typeof mode !== "string" || mode.length > MAX_MODE_STR_LEN) {
    throw new InvalidInputError(`unsupported conv mode: ${safeFormatValue(mode)}`);
  }
  const norm = mode.trim().toLowerCase();
  switch (norm) {
    case "full":
      return ConvMode.FULL;
    case "same":
      return ConvMode.SAME;
    case "valid":
      return ConvMode.VALID;
    default:
      throw new InvalidInputError(`unsupported conv mode: ${safeFormatValue(mode)}`);
  }
}
