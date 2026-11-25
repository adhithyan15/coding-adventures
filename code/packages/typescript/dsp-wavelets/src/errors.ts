/**
 * Base exception for all wavelet errors.
 */
export class WaveletError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "WaveletError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when the input signal is empty.
 */
export class EmptySignalError extends WaveletError {
  constructor(message: string = "wavelet signal must be non-empty") {
    super(message);
    this.name = "EmptySignalError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised on invalid parameters (e.g. levels <= 0, non-finite values, exceeding caps).
 */
export class InvalidParamError extends WaveletError {
  constructor(message: string = "invalid parameter") {
    super(message);
    this.name = "InvalidParamError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when the signal is too short to support the requested decomposition levels.
 */
export class SignalTooShortError extends WaveletError {
  constructor(message: string = "signal too short") {
    super(message);
    this.name = "SignalTooShortError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when coefficient shapes or values are invalid.
 */
export class InvalidCoefficientsError extends WaveletError {
  constructor(message: string = "invalid coefficients") {
    super(message);
    this.name = "InvalidCoefficientsError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Wraps FFT errors (for CWT).
 */
export class FftError extends WaveletError {
  constructor(message: string = "FFT failure") {
    super(message);
    this.name = "FftError";
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
  EmptyErr: (new (msg: string) => WaveletError) | null = null,
  InvalidErr: new (msg: string) => WaveletError = InvalidParamError
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
export function validateFloats(
  seq: ArrayLike<number>,
  name: string,
  ExcType: new (msg: string) => WaveletError = InvalidParamError
): void {
  const len = validateSequence(seq, name, Number.MAX_SAFE_INTEGER, null, ExcType);
  for (let idx = 0; idx < len; idx++) {
    const raw: unknown = seq[idx];
    if (typeof raw !== "number" || !Number.isFinite(raw)) {
      throw new ExcType(
        `non-finite float in ${name} at index ${idx}: ${safeFormatValue(raw)}`
      );
    }
  }
}
