/**
 * Types, constants, errors, and pre-flight validation for music analysis (VER15).
 */

/** Maximum number of audio samples supported in a single buffer (2^24 = 16,777,216). */
export const MAX_SAMPLES = 16_777_216;

/** Maximum FFT length supported (2^20 = 1,048,576). */
export const MAX_FFT_LEN = 1_048_576;

/** Maximum number of time frames in spectrogram / feature tracking (2^20 = 1,048,576). */
export const MAX_FRAMES = 1_048_576;

/** Maximum number of harmonic partials (up to 64). */
export const MAX_PARTIALS = 64;

/** Maximum number of detectable onsets (up to 65,536). */
export const MAX_ONSETS = 65_536;

/** Maximum musical tempo in beats per minute. */
export const MAX_BPM = 1000.0;

/** Minimum musical tempo in beats per minute. */
export const MIN_BPM = 1.0;

/** Maximum sampling rate supported in Hertz (192 kHz). */
export const MAX_SAMPLE_RATE_HZ = 192_000.0;

/** Minimum sampling rate supported in Hertz (1 kHz). */
export const MIN_SAMPLE_RATE_HZ = 1000.0;

/** Standard tuning reference frequency for A4 in Hertz (440 Hz). */
export const DEFAULT_A4_HZ = 440.0;

/**
 * Base error class for all music analysis errors.
 */
export class MusicAnalysisError extends Error {
  constructor(message: string) {
    super(message);
    this.name = this.constructor.name;
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when an input parameter is invalid, non-finite, or malformed (CWE-20).
 */
export class InvalidInputError extends MusicAnalysisError {
  constructor(message: string) {
    super(`InvalidInput: ${message}`);
    this.name = "InvalidInputError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Raised when an input exceeds pre-flight resource allocation bounds (CWE-400 / CWE-190).
 */
export class BudgetExceededError extends MusicAnalysisError {
  constructor(message: string) {
    super(`BudgetExceeded: ${message}`);
    this.name = "BudgetExceededError";
    Object.setPrototypeOf(this, new.target.prototype);
  }
}

/**
 * Compatibility alias / namespace mirroring Rust AnalysisError variants.
 */
export const AnalysisError = {
  InvalidInput: (msg: string): InvalidInputError => new InvalidInputError(msg),
  BudgetExceeded: (msg: string): BudgetExceededError => new BudgetExceededError(msg),
};
export type AnalysisError = InvalidInputError | BudgetExceededError;

/**
 * Result of frequency to note conversion.
 */
export interface NotePitch {
  /** Canonical note name with octave (e.g. "A4", "C#5"). */
  readonly note: string;
  /** Nearest MIDI semitone index (e.g. 69 for A4, 60 for C4). */
  readonly midi: number;
  /** Cents deviation from nearest semitone in [-50.0, +50.0]. */
  readonly cents: number;
}

/**
 * Validate that `val` is a finite float (CWE-20 fail-closed).
 */
export function validateFiniteFloat(val: unknown, name: string): number {
  if (typeof val !== "number" || !Number.isFinite(val)) {
    throw new InvalidInputError(`${name} must be finite; got ${String(val)}`);
  }
  return val;
}

/**
 * Validate that `val` is a finite positive float > 0.0, and optionally <= `maxVal`.
 */
export function validatePositiveFloat(
  val: unknown,
  name: string,
  maxVal?: number,
): number {
  const fVal = validateFiniteFloat(val, name);
  if (fVal <= 0.0) {
    throw new InvalidInputError(`${name} must be positive (> 0.0); got ${fVal}`);
  }
  if (maxVal !== undefined) {
    if (typeof maxVal !== "number" || !Number.isFinite(maxVal)) {
      throw new InvalidInputError(`maxVal for ${name} must be finite`);
    }
    if (fVal > maxVal) {
      throw new BudgetExceededError(
        `${name} ${fVal} exceeds maximum allowed ${maxVal}`,
      );
    }
  }
  return fVal;
}

/**
 * Validate that `val` is a positive safe integer > 0, and optionally <= `maxVal`.
 */
export function validatePositiveInt(
  val: unknown,
  name: string,
  maxVal?: number,
): number {
  if (typeof val !== "number" || !Number.isSafeInteger(val)) {
    throw new InvalidInputError(`${name} must be a safe integer; got ${String(val)}`);
  }
  if (val <= 0) {
    throw new InvalidInputError(`${name} must be positive (> 0); got ${val}`);
  }
  if (maxVal !== undefined) {
    if (val > maxVal) {
      throw new BudgetExceededError(
        `${name} ${val} exceeds maximum allowed ${maxVal}`,
      );
    }
  }
  return val;
}

/**
 * Validate that `val` is a non-negative safe integer >= 0, and optionally <= `maxVal`.
 */
export function validateNonNegativeInt(
  val: unknown,
  name: string,
  maxVal?: number,
): number {
  if (typeof val !== "number" || !Number.isSafeInteger(val)) {
    throw new InvalidInputError(`${name} must be a safe integer; got ${String(val)}`);
  }
  if (val < 0) {
    throw new InvalidInputError(`${name} must be non-negative (>= 0); got ${val}`);
  }
  if (maxVal !== undefined) {
    if (val > maxVal) {
      throw new BudgetExceededError(
        `${name} ${val} exceeds maximum allowed ${maxVal}`,
      );
    }
  }
  return val;
}

/**
 * Validate sample buffer before processing (CWE-400, CWE-20).
 * Pre-flight bounds checking prevents unbounded allocations or iterations.
 */
export function validateSamples(
  samples: ArrayLike<number> | Iterable<number> | null | undefined,
  name: string,
): Float64Array {
  if (samples == null) {
    throw new InvalidInputError(`${name} must not be null or undefined`);
  }

  // 1. If input has a .length property (ArrayLike: Array, TypedArray, { length: N })
  if (
    typeof samples === "object" &&
    "length" in (samples as object) &&
    typeof (samples as ArrayLike<number>).length === "number"
  ) {
    const rawLen = (samples as ArrayLike<number>).length;
    if (!Number.isSafeInteger(rawLen) || rawLen < 0) {
      throw new InvalidInputError(
        `${name} length must be a non-negative safe integer; got ${rawLen}`,
      );
    }
    if (rawLen === 0) {
      throw new InvalidInputError(`${name} must be non-empty`);
    }
    if (rawLen > MAX_SAMPLES) {
      throw new BudgetExceededError(
        `${name} length ${rawLen} exceeds MAX_SAMPLES=${MAX_SAMPLES}`,
      );
    }
    const out = new Float64Array(rawLen);
    const arrLike = samples as ArrayLike<number>;
    for (let i = 0; i < rawLen; i++) {
      const s = Number(arrLike[i]);
      if (!Number.isFinite(s)) {
        throw new InvalidInputError(`${name}[${i}] must be finite; got ${s}`);
      }
      out[i] = s;
    }
    return out;
  }

  // 2. If input is an arbitrary Iterable without .length (e.g. Set, Generator)
  if (typeof (samples as any)[Symbol.iterator] === "function") {
    const collected: number[] = [];
    let count = 0;
    for (const item of samples as Iterable<number>) {
      count++;
      if (count > MAX_SAMPLES) {
        throw new BudgetExceededError(
          `${name} exceeds MAX_SAMPLES=${MAX_SAMPLES}`,
        );
      }
      const s = Number(item);
      if (!Number.isFinite(s)) {
        throw new InvalidInputError(
          `${name}[${count - 1}] must be finite; got ${s}`,
        );
      }
      collected.push(s);
    }
    if (count === 0) {
      throw new InvalidInputError(`${name} must be non-empty`);
    }
    return Float64Array.from(collected);
  }

  throw new InvalidInputError(`${name} must be an ArrayLike or Iterable`);
}

/**
 * Validate frequency-domain magnitude or power spectrum (CWE-400, CWE-20).
 * Pre-flight bounds checking prevents unbounded allocations or iterations.
 */
export function validateSpectrum(
  spectrum: ArrayLike<number> | Iterable<number> | null | undefined,
  name: string,
): Float64Array {
  if (spectrum == null) {
    throw new InvalidInputError(`${name} must not be null or undefined`);
  }

  // 1. If input has a .length property (ArrayLike: Array, TypedArray, { length: N })
  if (
    typeof spectrum === "object" &&
    "length" in (spectrum as object) &&
    typeof (spectrum as ArrayLike<number>).length === "number"
  ) {
    const rawLen = (spectrum as ArrayLike<number>).length;
    if (!Number.isSafeInteger(rawLen) || rawLen < 0) {
      throw new InvalidInputError(
        `${name} length must be a non-negative safe integer; got ${rawLen}`,
      );
    }
    if (rawLen === 0) {
      throw new InvalidInputError(`${name} must be non-empty`);
    }
    if (rawLen > MAX_FFT_LEN) {
      throw new BudgetExceededError(
        `${name} length ${rawLen} exceeds MAX_FFT_LEN=${MAX_FFT_LEN}`,
      );
    }
    const out = new Float64Array(rawLen);
    const arrLike = spectrum as ArrayLike<number>;
    for (let i = 0; i < rawLen; i++) {
      const s = Number(arrLike[i]);
      if (!Number.isFinite(s)) {
        throw new InvalidInputError(`${name}[${i}] must be finite; got ${s}`);
      }
      out[i] = s;
    }
    return out;
  }

  // 2. If input is an arbitrary Iterable without .length (e.g. Set, Generator)
  if (typeof (spectrum as any)[Symbol.iterator] === "function") {
    const collected: number[] = [];
    let count = 0;
    for (const item of spectrum as Iterable<number>) {
      count++;
      if (count > MAX_FFT_LEN) {
        throw new BudgetExceededError(
          `${name} exceeds MAX_FFT_LEN=${MAX_FFT_LEN}`,
        );
      }
      const s = Number(item);
      if (!Number.isFinite(s)) {
        throw new InvalidInputError(
          `${name}[${count - 1}] must be finite; got ${s}`,
        );
      }
      collected.push(s);
    }
    if (count === 0) {
      throw new InvalidInputError(`${name} must be non-empty`);
    }
    return Float64Array.from(collected);
  }

  throw new InvalidInputError(`${name} must be an ArrayLike or Iterable`);
}
