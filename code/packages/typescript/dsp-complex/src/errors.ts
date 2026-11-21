/**
 * Error classes for complex number and array operations.
 *
 * Conforms to VER15 fail-closed validation contracts and dsp-complex.warrant.
 */

export class ComplexError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ComplexError";
  }
}

export class InvalidInputError extends ComplexError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidInputError";
  }
}

export class LengthMismatchError extends ComplexError {
  readonly realLen: number;
  readonly imagLen: number;

  constructor(realLen: number, imagLen: number) {
    super(`real array has ${realLen} elements, imag has ${imagLen}; must match`);
    this.name = "LengthMismatchError";
    this.realLen = realLen;
    this.imagLen = imagLen;
  }
}

export class OddInterleavedLengthError extends ComplexError {
  readonly length: number;

  constructor(length: number) {
    super(`interleaved buffer has odd length ${length}; must be even`);
    this.name = "OddInterleavedLengthError";
    this.length = length;
  }
}

const MAX_SAFE_ERROR_VALUE_LENGTH = 128;

export function safeFormatValue(value: unknown): string {
  if (value === null) return "null";
  if (value === undefined) return "undefined";
  if (typeof value === "symbol") {
    try {
      return value.toString().slice(0, MAX_SAFE_ERROR_VALUE_LENGTH);
    } catch {
      return "[symbol]";
    }
  }
  if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
    return String(value).slice(0, MAX_SAFE_ERROR_VALUE_LENGTH);
  }
  if (typeof value === "string") {
    return value.length > MAX_SAFE_ERROR_VALUE_LENGTH
      ? `${value.slice(0, MAX_SAFE_ERROR_VALUE_LENGTH)}...`
      : value;
  }
  try {
    const text = String(value);
    return text.length > MAX_SAFE_ERROR_VALUE_LENGTH
      ? `${text.slice(0, MAX_SAFE_ERROR_VALUE_LENGTH)}...`
      : text;
  } catch {
    try {
      return Object.prototype.toString.call(value);
    } catch {
      return "[object unknown]";
    }
  }
}

export function validateArrayLikeLength(data: unknown, name: string): number {
  if (
    data === null ||
    data === undefined ||
    typeof data !== "object" ||
    (ArrayBuffer.isView(data) && data instanceof DataView)
  ) {
    throw new InvalidInputError(`${name} must be a typed array or array-like of numbers`);
  }
  const rawLen = (data as { length?: unknown }).length;
  if (typeof rawLen !== "number" || !Number.isSafeInteger(rawLen) || rawLen < 0) {
    throw new InvalidInputError(
      `${name} length must be a non-negative safe integer; got ${safeFormatValue(rawLen)}`
    );
  }
  return rawLen;
}

export function readFiniteNumber(data: ArrayLike<number>, index: number): number {
  const raw = (data as Record<number, unknown>)[index];
  if (typeof raw !== "number" || !Number.isFinite(raw)) {
    throw new InvalidInputError("input contains non-finite float (NaN or Inf)");
  }
  return raw;
}
