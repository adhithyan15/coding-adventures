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
