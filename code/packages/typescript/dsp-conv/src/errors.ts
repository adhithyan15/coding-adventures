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
 * Validate that all items in seq are finite numbers (CWE-20).
 */
export function validateFloats(seq: ArrayLike<number>, name: string): void {
  const len = seq.length;
  for (let idx = 0; idx < len; idx++) {
    const val = Number(seq[idx]);
    if (!Number.isFinite(val)) {
      throw new InvalidInputError(
        `non-finite float in ${name} at index ${idx}: ${val}`
      );
    }
  }
}
