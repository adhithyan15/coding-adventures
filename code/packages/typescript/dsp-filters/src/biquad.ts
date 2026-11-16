import { MAX_ORDER, MAX_SIGNAL_LEN } from "./constants.js";
import { EmptySignalError, InvalidCoefficientError } from "./errors.js";

/**
 * Second-order section (biquad) filter using Direct-Form-II Transposed structure.
 *
 * Conforms to DSP03 and VER15 security requirements:
 * - CWE-400 bounds checks before allocating.
 * - CWE-20 fail-closed finite float validation.
 * - CWE-674 strictly iterative execution (zero recursion).
 */
export class Biquad {
  readonly b0: number;
  readonly b1: number;
  readonly b2: number;
  readonly a1: number;
  readonly a2: number;

  private z1: number = 0.0;
  private z2: number = 0.0;

  constructor(
    b0: number,
    b1: number,
    b2: number,
    a0: number,
    a1: number,
    a2: number
  ) {
    for (const val of [b0, b1, b2, a0, a1, a2]) {
      if (!Number.isFinite(val)) {
        throw new InvalidCoefficientError("biquad coefficients must be finite numbers");
      }
    }
    if (a0 === 0.0) {
      throw new InvalidCoefficientError("a0 must be non-zero");
    }

    const invA0 = 1.0 / a0;
    this.b0 = b0 * invA0;
    this.b1 = b1 * invA0;
    this.b2 = b2 * invA0;
    this.a1 = a1 * invA0;
    this.a2 = a2 * invA0;
  }

  reset(): void {
    this.z1 = 0.0;
    this.z2 = 0.0;
  }

  processSample(x: number): number {
    if (!Number.isFinite(x)) {
      throw new InvalidCoefficientError("non-finite sample in biquad input");
    }
    const y = this.b0 * x + this.z1;
    this.z1 = this.b1 * x - this.a1 * y + this.z2;
    this.z2 = this.b2 * x - this.a2 * y;
    return y;
  }

  process(signal: Float64Array | ArrayLike<number>): Float64Array {
    const n = signal.length;
    if (n === 0) {
      throw new EmptySignalError("biquad input signal must be non-empty");
    }
    if (n > MAX_SIGNAL_LEN) {
      throw new InvalidCoefficientError(
        `signal length ${n} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
      );
    }

    const out = new Float64Array(n);
    for (let i = 0; i < n; i++) {
      out[i] = this.processSample(Number(signal[i]));
    }
    return out;
  }

  // ─────────────────────────── Audio EQ Cookbook Designs ───────────────────────────

  static lowpass(cutoffNorm: number, q: number = 1.0 / Math.SQRT2): Biquad {
    if (!Number.isFinite(cutoffNorm) || cutoffNorm <= 0.0 || cutoffNorm >= 0.5) {
      throw new InvalidCoefficientError("cutoffNorm must be in (0.0, 0.5)");
    }
    if (!Number.isFinite(q) || q <= 0.0) {
      throw new InvalidCoefficientError("Q must be finite and > 0");
    }

    const omega = 2.0 * Math.PI * cutoffNorm;
    const sinW = Math.sin(omega);
    const cosW = Math.cos(omega);
    const alpha = sinW / (2.0 * q);

    const b0 = (1.0 - cosW) * 0.5;
    const b1 = 1.0 - cosW;
    const b2 = b0;
    const a0 = 1.0 + alpha;
    const a1 = -2.0 * cosW;
    const a2 = 1.0 - alpha;
    return new Biquad(b0, b1, b2, a0, a1, a2);
  }

  static highpass(cutoffNorm: number, q: number = 1.0 / Math.SQRT2): Biquad {
    if (!Number.isFinite(cutoffNorm) || cutoffNorm <= 0.0 || cutoffNorm >= 0.5) {
      throw new InvalidCoefficientError("cutoffNorm must be in (0.0, 0.5)");
    }
    if (!Number.isFinite(q) || q <= 0.0) {
      throw new InvalidCoefficientError("Q must be finite and > 0");
    }

    const omega = 2.0 * Math.PI * cutoffNorm;
    const sinW = Math.sin(omega);
    const cosW = Math.cos(omega);
    const alpha = sinW / (2.0 * q);

    const b0 = (1.0 + cosW) * 0.5;
    const b1 = -(1.0 + cosW);
    const b2 = b0;
    const a0 = 1.0 + alpha;
    const a1 = -2.0 * cosW;
    const a2 = 1.0 - alpha;
    return new Biquad(b0, b1, b2, a0, a1, a2);
  }

  static bandpass(centerNorm: number, q: number = 1.0): Biquad {
    if (!Number.isFinite(centerNorm) || centerNorm <= 0.0 || centerNorm >= 0.5) {
      throw new InvalidCoefficientError("centerNorm must be in (0.0, 0.5)");
    }
    if (!Number.isFinite(q) || q <= 0.0) {
      throw new InvalidCoefficientError("Q must be finite and > 0");
    }

    const omega = 2.0 * Math.PI * centerNorm;
    const sinW = Math.sin(omega);
    const cosW = Math.cos(omega);
    const alpha = sinW / (2.0 * q);

    const b0 = sinW * 0.5;
    const b1 = 0.0;
    const b2 = -b0;
    const a0 = 1.0 + alpha;
    const a1 = -2.0 * cosW;
    const a2 = 1.0 - alpha;
    return new Biquad(b0, b1, b2, a0, a1, a2);
  }

  static notch(centerNorm: number, q: number = 1.0): Biquad {
    if (!Number.isFinite(centerNorm) || centerNorm <= 0.0 || centerNorm >= 0.5) {
      throw new InvalidCoefficientError("centerNorm must be in (0.0, 0.5)");
    }
    if (!Number.isFinite(q) || q <= 0.0) {
      throw new InvalidCoefficientError("Q must be finite and > 0");
    }

    const omega = 2.0 * Math.PI * centerNorm;
    const sinW = Math.sin(omega);
    const cosW = Math.cos(omega);
    const alpha = sinW / (2.0 * q);

    const b0 = 1.0;
    const b1 = -2.0 * cosW;
    const b2 = 1.0;
    const a0 = 1.0 + alpha;
    const a1 = -2.0 * cosW;
    const a2 = 1.0 - alpha;
    return new Biquad(b0, b1, b2, a0, a1, a2);
  }

  static allpass(centerNorm: number, q: number = 1.0): Biquad {
    if (!Number.isFinite(centerNorm) || centerNorm <= 0.0 || centerNorm >= 0.5) {
      throw new InvalidCoefficientError("centerNorm must be in (0.0, 0.5)");
    }
    if (!Number.isFinite(q) || q <= 0.0) {
      throw new InvalidCoefficientError("Q must be finite and > 0");
    }

    const omega = 2.0 * Math.PI * centerNorm;
    const sinW = Math.sin(omega);
    const cosW = Math.cos(omega);
    const alpha = sinW / (2.0 * q);

    const b0 = 1.0 - alpha;
    const b1 = -2.0 * cosW;
    const b2 = 1.0 + alpha;
    const a0 = 1.0 + alpha;
    const a1 = -2.0 * cosW;
    const a2 = 1.0 - alpha;
    return new Biquad(b0, b1, b2, a0, a1, a2);
  }

  static peak(centerNorm: number, gainDb: number, q: number = 1.0): Biquad {
    if (!Number.isFinite(centerNorm) || centerNorm <= 0.0 || centerNorm >= 0.5) {
      throw new InvalidCoefficientError("centerNorm must be in (0.0, 0.5)");
    }
    if (!Number.isFinite(gainDb)) {
      throw new InvalidCoefficientError("gainDb must be finite");
    }
    if (!Number.isFinite(q) || q <= 0.0) {
      throw new InvalidCoefficientError("Q must be finite and > 0");
    }

    const aGain = Math.pow(10.0, gainDb / 40.0);
    const omega = 2.0 * Math.PI * centerNorm;
    const sinW = Math.sin(omega);
    const cosW = Math.cos(omega);
    const alpha = sinW / (2.0 * q);

    const b0 = 1.0 + alpha * aGain;
    const b1 = -2.0 * cosW;
    const b2 = 1.0 - alpha * aGain;
    const a0 = 1.0 + alpha / aGain;
    const a1 = -2.0 * cosW;
    const a2 = 1.0 - alpha / aGain;
    return new Biquad(b0, b1, b2, a0, a1, a2);
  }
}

/**
 * Filter signal using a cascade of Second-Order Sections (SOS).
 * Matches scipy.signal.sosfilt.
 */
export function sosfilt(
  sos: (Float64Array | ArrayLike<number>)[],
  signal: Float64Array | ArrayLike<number>
): Float64Array {
  const n = signal.length;
  if (n === 0) {
    throw new EmptySignalError("signal must not be empty");
  }
  if (n > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError(
      `signal length ${n} exceeds MAX_SIGNAL_LEN (${MAX_SIGNAL_LEN})`
    );
  }

  const numSections = sos.length;
  if (numSections > MAX_ORDER) {
    throw new InvalidCoefficientError(
      `section count ${numSections} exceeds MAX_ORDER (${MAX_ORDER})`
    );
  }

  // Pre-validate all floats in signal
  let current: Float64Array = new Float64Array(n);
  for (let i = 0; i < n; i++) {
    const val = Number(signal[i]);
    if (!Number.isFinite(val)) {
      throw new InvalidCoefficientError("non-finite sample in signal");
    }
    current[i] = val;
  }

  for (let s = 0; s < numSections; s++) {
    const sec = sos[s];
    if (sec.length !== 6) {
      throw new InvalidCoefficientError(
        `section ${s} must have 6 coefficients; got ${sec.length}`
      );
    }
    const b0 = Number(sec[0]);
    const b1 = Number(sec[1]);
    const b2 = Number(sec[2]);
    const a0 = Number(sec[3]);
    const a1 = Number(sec[4]);
    const a2 = Number(sec[5]);
    const bq = new Biquad(b0, b1, b2, a0, a1, a2);
    current = bq.process(current);
  }

  return current;
}

export const sosCascade = sosfilt;
export const sos_cascade = sosfilt;
