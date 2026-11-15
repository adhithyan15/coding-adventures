/**
 * Error classes for DCT operations.
 *
 * Conforms to VER15 fail-closed validation contracts and dsp-dct.warrant.
 */

export class DctError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "DctError";
  }
}

export class EmptyInputError extends DctError {
  constructor(message: string = "input signal must not be empty") {
    super(message);
    this.name = "EmptyInputError";
  }
}

export class InvalidInputError extends DctError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidInputError";
  }
}
