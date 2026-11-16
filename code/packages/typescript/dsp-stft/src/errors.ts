/**
 * Base exception for all STFT errors.
 */
export class StftError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "StftError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when the input signal is empty.
 */
export class EmptySignalError extends StftError {
  constructor(message: string = "STFT signal must be non-empty") {
    super(message);
    this.name = "EmptySignalError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised on invalid parameters (e.g. nFft <= 0, non-finite values, exceeding caps).
 */
export class InvalidParamError extends StftError {
  constructor(message: string = "invalid parameter") {
    super(message);
    this.name = "InvalidParamError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when strict-mode framing fails (signal.length < nFft).
 */
export class SignalTooShortError extends StftError {
  constructor(message: string = "signal too short") {
    super(message);
    this.name = "SignalTooShortError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when a spectrogram buffer is malformed or invalid for ISTFT.
 */
export class InvalidSpectrogramError extends StftError {
  constructor(message: string = "invalid spectrogram") {
    super(message);
    this.name = "InvalidSpectrogramError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Wraps FFT errors from the underlying transform engine.
 */
export class FftError extends StftError {
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
  ExcType: new (msg: string) => StftError = InvalidParamError
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
