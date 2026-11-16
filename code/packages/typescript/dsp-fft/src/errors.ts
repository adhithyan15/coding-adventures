/**
 * Error classes for FFT operations.
 *
 * Conforms to VER15 fail-closed validation contracts and dsp-fft.warrant.
 */

export class FftError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FftError";
  }
}

export class InvalidInputError extends FftError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidInputError";
  }
}

export class NotPowerOfTwoError extends FftError {
  readonly n: number;

  constructor(n: number) {
    super(`FFT length ${n} is not a power of two; use Bluestein for arbitrary N`);
    this.name = "NotPowerOfTwoError";
    this.n = n;
  }
}
