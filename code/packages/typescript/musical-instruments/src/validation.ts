/**
 * Defensive parameter validation and resource bounding helpers (VER15 / CWE-20 / CWE-190 / CWE-400).
 */

import {
  MAX_NOTE_DURATION_SECONDS,
  MAX_PARTIALS,
  MAX_SAMPLE_COUNT,
  MAX_SAMPLE_RATE_HZ,
} from "./constants.js";

export function validateFinite(name: string, value: number): number {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new TypeError(`${name} must be a finite number, got ${String(value)}`);
  }
  return value;
}

export function validateNonNegative(name: string, value: number): number {
  const v = validateFinite(name, value);
  if (v < 0.0) {
    throw new RangeError(`${name} must be >= 0.0, got ${String(v)}`);
  }
  return v;
}

export function validatePositive(name: string, value: number): number {
  const v = validateFinite(name, value);
  if (v <= 0.0) {
    throw new RangeError(`${name} must be > 0.0, got ${String(v)}`);
  }
  return v;
}

export function validateUnit(name: string, value: number): number {
  const v = validateFinite(name, value);
  if (v < 0.0 || v > 1.0) {
    throw new RangeError(`${name} must be in [0.0, 1.0], got ${String(v)}`);
  }
  return v;
}

export function validateDuration(name: string, value: number): number {
  const v = validateNonNegative(name, value);
  if (v > MAX_NOTE_DURATION_SECONDS) {
    throw new RangeError(
      `${name} ${String(v)} exceeds MAX_NOTE_DURATION_SECONDS=${String(MAX_NOTE_DURATION_SECONDS)}`,
    );
  }
  return v;
}

export function validateSampleRate(name: string, value: number): number {
  const v = validatePositive(name, value);
  if (v > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError(
      `${name} ${String(v)} exceeds MAX_SAMPLE_RATE_HZ=${String(MAX_SAMPLE_RATE_HZ)}`,
    );
  }
  return v;
}

export function validateSafeInteger(name: string, value: number): number {
  if (typeof value !== "number" || !Number.isSafeInteger(value)) {
    throw new TypeError(`${name} must be a safe integer, got ${String(value)}`);
  }
  return value;
}

export function validatePositiveInt(name: string, value: number): number {
  const v = validateSafeInteger(name, value);
  if (v <= 0) {
    throw new RangeError(`${name} must be > 0, got ${String(v)}`);
  }
  return v;
}

export function validateNonNegativeInt(name: string, value: number): number {
  const v = validateSafeInteger(name, value);
  if (v < 0) {
    throw new RangeError(`${name} must be >= 0, got ${String(v)}`);
  }
  return v;
}

export function validateSampleCountCeiling(name: string, value: number, max = MAX_SAMPLE_COUNT): number {
  const v = validatePositiveInt(name, value);
  if (v > max) {
    throw new RangeError(`${name} ${String(v)} exceeds maximum allowable ${String(max)}`);
  }
  return v;
}

export function validatePartialCountCeiling(name: string, value: number, max = MAX_PARTIALS): number {
  const v = validatePositiveInt(name, value);
  if (v > max) {
    throw new RangeError(`${name} ${String(v)} exceeds maximum allowable ${String(max)}`);
  }
  return v;
}

export function safeRoundSample(timeVal: number, rate: number): number {
  const t = validateFinite("time", timeVal);
  const r = validateFinite("rate", rate);
  const prod = t * r;
  if (!Number.isFinite(prod)) {
    throw new RangeError("time * rate product must be finite");
  }
  const rounded = Math.round(prod);
  if (!Number.isSafeInteger(rounded)) {
    throw new RangeError(`sample index ${String(rounded)} exceeds safe integer range`);
  }
  return rounded;
}
