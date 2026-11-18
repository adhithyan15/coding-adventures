import { MAX_ORDER, MAX_SIGNAL_LEN } from "./constants.js";
import { EmptySignalError, InvalidCoefficientError } from "./errors.js";

type IndexedValues = ArrayLike<unknown>;

function finiteNumber(value: unknown, name: string): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new InvalidCoefficientError(`${name} must be a finite primitive number`);
  }
  return value;
}

function lengthOnce(value: unknown, name: string): number {
  if (value === null || typeof value !== "object") {
    throw new InvalidCoefficientError(`${name} must be indexed`);
  }
  const length = (value as { readonly length: unknown }).length;
  if (typeof length !== "number" || !Number.isSafeInteger(length) || length < 0) {
    throw new InvalidCoefficientError(`${name} length must be a safe nonnegative integer`);
  }
  return length;
}

type Advanced = readonly [output: number, z1: number, z2: number];

/** A strict, finite, failure-atomic binary64 DFII-T filter section. */
export class CheckedBiquad {
  readonly b0: number;
  readonly b1: number;
  readonly b2: number;
  readonly a1: number;
  readonly a2: number;

  private z1 = 0.0;
  private z2 = 0.0;

  constructor(b0: number, b1: number, b2: number, a0: number, a1: number, a2: number) {
    const raw = [b0, b1, b2, a0, a1, a2].map((value, index) =>
      finiteNumber(value, `coefficient[${index}]`),
    );
    if (raw[3] === 0.0) {
      throw new InvalidCoefficientError("a0 must be non-zero");
    }

    const normalized = [raw[0] / raw[3], raw[1] / raw[3], raw[2] / raw[3], raw[4] / raw[3], raw[5] / raw[3]];
    if (!normalized.every(Number.isFinite)) {
      throw new InvalidCoefficientError("normalized coefficients must be finite");
    }
    [this.b0, this.b1, this.b2, this.a1, this.a2] = normalized;
  }

  private advance(sample: number, z1: number, z2: number): Advanced {
    const output = this.b0 * sample + z1;
    const nextZ1 = this.b1 * sample - this.a1 * output + z2;
    const nextZ2 = this.b2 * sample - this.a2 * output;
    if (!Number.isFinite(output)) {
      throw new InvalidCoefficientError("non-finite biquad output");
    }
    if (!Number.isFinite(nextZ1) || !Number.isFinite(nextZ2)) {
      throw new InvalidCoefficientError("non-finite biquad delay");
    }
    return [output, nextZ1, nextZ2];
  }

  processSample(sample: number): number {
    const value = finiteNumber(sample, "sample");
    const [output, nextZ1, nextZ2] = this.advance(value, this.z1, this.z2);
    this.z1 = nextZ1;
    this.z2 = nextZ2;
    return output;
  }

  process(signal: IndexedValues): Float64Array {
    const length = lengthOnce(signal, "signal");
    if (length === 0) {
      throw new EmptySignalError("biquad input signal must be non-empty");
    }
    if (length > MAX_SIGNAL_LEN) {
      throw new InvalidCoefficientError("signal exceeds MAX_SAMPLES");
    }

    let z1 = this.z1;
    let z2 = this.z2;
    const output = new Float64Array(length);
    for (let index = 0; index < length; index += 1) {
      const sample = finiteNumber(signal[index], `signal[${index}]`);
      const advanced = this.advance(sample, z1, z2);
      output[index] = advanced[0];
      z1 = advanced[1];
      z2 = advanced[2];
    }

    this.z1 = z1;
    this.z2 = z2;
    return output;
  }

  reset(): void {
    this.z1 = 0.0;
    this.z2 = 0.0;
  }
}

/** Process one complete finite signal through checked SOS sections. */
export function checkedSosfilt(
  sections: IndexedValues,
  signal: IndexedValues,
): Float64Array {
  const signalLength = lengthOnce(signal, "signal");
  if (signalLength === 0) {
    throw new EmptySignalError("signal must not be empty");
  }
  if (signalLength > MAX_SIGNAL_LEN) {
    throw new InvalidCoefficientError("signal exceeds MAX_SAMPLES");
  }

  const sectionCount = lengthOnce(sections, "sections");
  if (sectionCount > MAX_ORDER) {
    throw new InvalidCoefficientError("section count exceeds MAX_ORDER");
  }

  const checkedSections: CheckedBiquad[] = [];
  for (let sectionIndex = 0; sectionIndex < sectionCount; sectionIndex += 1) {
    const row = sections[sectionIndex];
    if (row === null || typeof row !== "object") {
      throw new InvalidCoefficientError(`section[${sectionIndex}] must be indexed`);
    }
    const rowLength = lengthOnce(row, `section[${sectionIndex}]`);
    if (rowLength !== 6) {
      throw new InvalidCoefficientError("SOS section must have six coefficients");
    }
    const values: number[] = [];
    for (let index = 0; index < 6; index += 1) {
      values.push(finiteNumber((row as IndexedValues)[index], `section[${sectionIndex}][${index}]`));
    }
    checkedSections.push(new CheckedBiquad(values[0], values[1], values[2], values[3], values[4], values[5]));
  }

  let current: Float64Array = new Float64Array(signalLength);
  for (let index = 0; index < signalLength; index += 1) {
    current[index] = finiteNumber(signal[index], `signal[${index}]`);
  }
  for (const section of checkedSections) {
    current = section.process(current);
  }
  return current;
}
