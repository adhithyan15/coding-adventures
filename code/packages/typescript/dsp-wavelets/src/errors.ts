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
 * Validate that all items in seq are finite numbers (CWE-20).
 */
export function validateFloats(
  seq: ArrayLike<number>,
  name: string,
  ExcType: new (msg: string) => WaveletError = InvalidParamError
): void {
  const len = seq.length;
  for (let idx = 0; idx < len; idx++) {
    const val = Number(seq[idx]);
    if (!Number.isFinite(val)) {
      throw new ExcType(
        `non-finite float in ${name} at index ${idx}: ${val}`
      );
    }
  }
}
