import { InvalidInputError, safeFormatValue } from "./errors.js";

const MAX_ENUM_STRING_LENGTH = 64;

export enum DctType {
  I = 1,
  II = 2,
  III = 3,
  IV = 4,
}

export enum DctNorm {
  NONE = 0,
  ORTHO = 1,
}

const NORM_MAP = new Map<string, DctNorm>([
  ["none", DctNorm.NONE],
  ["raw", DctNorm.NONE],
  ["unnorm", DctNorm.NONE],
  ["0", DctNorm.NONE],
  ["ortho", DctNorm.ORTHO],
  ["unitary", DctNorm.ORTHO],
  ["1", DctNorm.ORTHO],
]);

export function parseNorm(norm?: DctNorm | string | number): DctNorm {
  if (norm === undefined) {
    return DctNorm.ORTHO;
  }
  if (typeof norm === "number") {
    if (norm === 0) return DctNorm.NONE;
    if (norm === 1) return DctNorm.ORTHO;
  }
  if (typeof norm === "string") {
    if (norm.length > MAX_ENUM_STRING_LENGTH) {
      throw new InvalidInputError(`unknown DCT normalization: ${safeFormatValue(norm)}`);
    }
    const found = NORM_MAP.get(norm.trim().toLowerCase());
    if (found !== undefined) return found;
  }
  throw new InvalidInputError(`unknown DCT normalization: ${safeFormatValue(norm)}`);
}

const TYPE_MAP = new Map<string, DctType>([
  ["1", DctType.I],
  ["i", DctType.I],
  ["2", DctType.II],
  ["ii", DctType.II],
  ["3", DctType.III],
  ["iii", DctType.III],
  ["4", DctType.IV],
  ["iv", DctType.IV],
]);

export function parseType(dctType?: DctType | string | number): DctType {
  if (dctType === undefined) {
    return DctType.II;
  }
  if (typeof dctType === "number") {
    if (Number.isSafeInteger(dctType) && dctType >= 1 && dctType <= 4) return dctType as DctType;
  }
  if (typeof dctType === "string") {
    if (dctType.length > MAX_ENUM_STRING_LENGTH) {
      throw new InvalidInputError(`unknown DCT type: ${safeFormatValue(dctType)}`);
    }
    const found = TYPE_MAP.get(dctType.trim().toLowerCase());
    if (found !== undefined) return found;
  }
  throw new InvalidInputError(`unknown DCT type: ${safeFormatValue(dctType)}`);
}
