/**
 * Base exception for all convolution errors.
 */
export class ConvError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "ConvError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when the input signal or image is empty.
 */
export class EmptySignalError extends ConvError {
  constructor(message: string = "convolution signal must be non-empty") {
    super(message);
    this.name = "EmptySignalError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when the convolution kernel is empty.
 */
export class EmptyKernelError extends ConvError {
  constructor(message: string = "convolution kernel must be non-empty") {
    super(message);
    this.name = "EmptyKernelError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when image or kernel dimensions do not match the buffer length.
 */
export class ImageSizeMismatchError extends ConvError {
  constructor(message: string = "image size mismatch") {
    super(message);
    this.name = "ImageSizeMismatchError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when the kernel is larger than the signal or image.
 */
export class KernelTooLargeError extends ConvError {
  constructor(message: string = "kernel too large") {
    super(message);
    this.name = "KernelTooLargeError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised on non-finite floats, defensive cap violations, or invalid arguments.
 */
export class InvalidInputError extends ConvError {
  constructor(message: string = "invalid input") {
    super(message);
    this.name = "InvalidInputError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Safely format an arbitrary untrusted value for error messages without
 * throwing on Symbol or Object.create(null) inputs (VER15).
 */
export function safeFormatValue(value: unknown): string {
  if (typeof value === "symbol") {
    return value.toString();
  }
  if (value === null || (typeof value !== "object" && typeof value !== "function")) {
    return String(value);
  }
  try {
    return String(value);
  } catch {
    try {
      return Object.prototype.toString.call(value);
    } catch {
      return "[unformattable]";
    }
  }
}

/**
 * Validate that an ArrayLike<number> input is a non-null numeric sequence
 * (rejecting DataView, BigInt64Array, BigUint64Array, primitives, and oversized
 * lengths before any allocation). Returns the validated length.
 */
export function validateSequence(
  seq: unknown,
  name: string,
  maxLen: number,
  EmptyErr: (new (msg: string) => ConvError) | null = null,
  InvalidErr: new (msg: string) => ConvError = InvalidInputError
): number {
  if (seq === null || seq === undefined || typeof seq !== "object") {
    throw new InvalidErr(`${name} must be an array-like object; got ${safeFormatValue(seq)}`);
  }
  if (seq instanceof DataView) {
    throw new InvalidErr(`${name} must be a numeric ArrayLike, not DataView`);
  }
  if (seq instanceof BigInt64Array) {
    throw new InvalidErr(`${name} must be a numeric ArrayLike, not BigInt64Array`);
  }
  if (seq instanceof BigUint64Array) {
    throw new InvalidErr(`${name} must be a numeric ArrayLike, not BigUint64Array`);
  }
  const rawLen = (seq as { length?: unknown }).length;
  if (typeof rawLen !== "number" || !Number.isSafeInteger(rawLen) || rawLen < 0) {
    throw new InvalidErr(
      `${name} length must be a non-negative safe integer; got ${safeFormatValue(rawLen)}`
    );
  }
  if (rawLen === 0 && EmptyErr !== null) {
    throw new EmptyErr(`${name} must be non-empty`);
  }
  if (rawLen > maxLen) {
    throw new InvalidErr(`${name} length ${rawLen} exceeds maximum ${maxLen}`);
  }
  return rawLen;
}

/**
 * Validate that all items in seq are finite numbers (CWE-20).
 */
export function validateFloats(seq: ArrayLike<number>, name: string): void {
  const len = validateSequence(seq, name, Number.MAX_SAFE_INTEGER);
  for (let idx = 0; idx < len; idx++) {
    const raw: unknown = seq[idx];
    if (typeof raw !== "number" || !Number.isFinite(raw)) {
      throw new InvalidInputError(
        `non-finite float in ${name} at index ${idx}: ${safeFormatValue(raw)}`
      );
    }
  }
}
