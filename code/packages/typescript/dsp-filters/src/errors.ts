/**
 * Error classes for filter operations.
 *
 * Conforms to VER15 fail-closed validation contracts and dsp-filters.warrant.
 */

export class FilterError extends Error {
  constructor(message: string) {
    super(message);
    this.name = "FilterError";
  }
}

export class EmptySignalError extends FilterError {
  constructor(message: string = "filter signal must be non-empty") {
    super(message);
    this.name = "EmptySignalError";
  }
}

export class EmptyKernelError extends FilterError {
  constructor(message: string = "filter kernel or coefficient list must be non-empty") {
    super(message);
    this.name = "EmptyKernelError";
  }
}

export class InvalidCoefficientError extends FilterError {
  constructor(message: string) {
    super(message);
    this.name = "InvalidCoefficientError";
  }
}

export class FftError extends FilterError {
  constructor(message: string) {
    super(message);
    this.name = "FftError";
  }
}
