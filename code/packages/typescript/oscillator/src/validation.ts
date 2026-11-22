/**
 * Input validation and math helpers for oscillators and samplers (OSC00, CWE-20, CWE-400).
 */

import {
  INTEGER_TOLERANCE,
  MAX_DURATION_SECONDS,
  MAX_FREQ_HZ,
  MAX_SAMPLES,
  MAX_SAMPLE_RATE_HZ,
} from "./constants.js";

const MAX_SAFE_ERROR_VALUE_LENGTH = 128;

/**
 * Safely format an untrusted value for error messages without throwing on
 * Symbol, null-prototype objects (`Object.create(null)`), or hostile `toString()`.
 */
export function safeFormatValue(value: unknown): string {
  if (value === null) return "null";
  if (value === undefined) return "undefined";
  if (typeof value === "symbol") {
    try {
      return value.toString().slice(0, MAX_SAFE_ERROR_VALUE_LENGTH);
    } catch {
      return "[symbol]";
    }
  }
  if (typeof value === "number" || typeof value === "boolean" || typeof value === "bigint") {
    return String(value).slice(0, MAX_SAFE_ERROR_VALUE_LENGTH);
  }
  if (typeof value === "string") {
    return value.length > MAX_SAFE_ERROR_VALUE_LENGTH
      ? `${value.slice(0, MAX_SAFE_ERROR_VALUE_LENGTH)}...`
      : value;
  }
  try {
    const text = String(value);
    return text.length > MAX_SAFE_ERROR_VALUE_LENGTH
      ? `${text.slice(0, MAX_SAFE_ERROR_VALUE_LENGTH)}...`
      : text;
  } catch {
    try {
      return Object.prototype.toString.call(value);
    } catch {
      return "[object unknown]";
    }
  }
}

export function validateFinite(name: string, value: number): number {
  if (typeof value !== "number" || Number.isNaN(value) || !Number.isFinite(value)) {
    throw new TypeError(
      `parameter '${name}' must be a finite number, got ${safeFormatValue(value)}`,
    );
  }
  return value;
}

export function validateNonNegative(name: string, value: number): number {
  const v = validateFinite(name, value);
  if (v < 0.0) {
    throw new RangeError(
      `parameter '${name}' must be non-negative (>= 0.0), got ${safeFormatValue(v)}`,
    );
  }
  return v;
}

export function validatePositive(name: string, value: number): number {
  const v = validateFinite(name, value);
  if (v <= 0.0) {
    throw new RangeError(
      `parameter '${name}' must be positive (> 0.0), got ${safeFormatValue(v)}`,
    );
  }
  return v;
}

export function validateFrequency(name: string, value: number): number {
  const v = validateNonNegative(name, value);
  if (v > MAX_FREQ_HZ) {
    throw new RangeError(
      `frequency '${name}' ${safeFormatValue(v)} Hz exceeds maximum allowable ${safeFormatValue(MAX_FREQ_HZ)} Hz`,
    );
  }
  return v;
}

export function validateSampleRate(name: string, value: number): number {
  const v = validatePositive(name, value);
  if (v > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError(
      `sample rate '${name}' ${safeFormatValue(v)} Hz exceeds maximum allowable ${safeFormatValue(MAX_SAMPLE_RATE_HZ)} Hz`,
    );
  }
  return v;
}

export function validateDuration(name: string, value: number): number {
  const v = validateNonNegative(name, value);
  if (v > MAX_DURATION_SECONDS) {
    throw new RangeError(
      `duration '${name}' ${safeFormatValue(v)} s exceeds maximum allowable ${safeFormatValue(MAX_DURATION_SECONDS)} s`,
    );
  }
  return v;
}

export function validateDutyCycle(name: string, value: number): number {
  const v = validateFinite(name, value);
  if (v <= 0.0 || v >= 1.0) {
    throw new RangeError(
      `parameter '${name}' must satisfy 0.0 < duty_cycle < 1.0, got ${safeFormatValue(v)}`,
    );
  }
  return v;
}

export function validateSampleCount(name: string, count: number): number {
  if (typeof count !== "number" || !Number.isSafeInteger(count)) {
    throw new TypeError(
      `parameter '${name}' must be a safe integer, got ${safeFormatValue(count)}`,
    );
  }
  if (count < 0) {
    throw new RangeError(
      `parameter '${name}' must be non-negative, got ${safeFormatValue(count)}`,
    );
  }
  if (count > MAX_SAMPLES) {
    throw new RangeError(
      `sample count '${name}' ${safeFormatValue(count)} exceeds maximum allowable ${safeFormatValue(MAX_SAMPLES)}`,
    );
  }
  return count;
}

export function validateIntegerIndex(name: string, index: number, count: number): number {
  if (typeof index !== "number" || !Number.isSafeInteger(index)) {
    throw new TypeError(
      `parameter '${name}' must be a safe integer, got ${safeFormatValue(index)}`,
    );
  }
  if (index < 0 || index >= count) {
    throw new RangeError(
      `index '${name}' ${safeFormatValue(index)} out of bounds for buffer of size ${safeFormatValue(count)}`,
    );
  }
  return index;
}

/**
 * Return the fractional part in the half-open interval [0.0, 1.0).
 * Handles negative numbers consistently via x - floor(x).
 */
export function fractionalPart(value: number): number {
  return value - Math.floor(value);
}

/**
 * Return half the sample rate: the Nyquist frequency limit.
 */
export function nyquistFrequency(sampleRateHz: number): number {
  const rate = validateSampleRate("sampleRateHz", sampleRateHz);
  return rate / 2.0;
}

/**
 * Calculate the number of samples for a given duration and sample rate.
 * Uses integer tolerance to prevent floating-point off-by-one errors.
 */
export function sampleCountForDuration(durationSeconds: number, sampleRateHz: number): number {
  const duration = validateDuration("durationSeconds", durationSeconds);
  const sampleRate = validateSampleRate("sampleRateHz", sampleRateHz);

  const rawCount = duration * sampleRate;
  const nearest = Math.round(rawCount);
  const count = Math.abs(rawCount - nearest) <= INTEGER_TOLERANCE ? nearest : Math.floor(rawCount);

  return validateSampleCount("sampleCount", count);
}

/**
 * Return the timestamp in seconds corresponding to sample index on a uniform grid.
 */
export function timeAtSample(
  index: number,
  sampleRateHz: number,
  startTimeSeconds = 0.0,
): number {
  const validIndex = validateNonNegative("index", index);
  if (!Number.isSafeInteger(validIndex)) {
    throw new TypeError(`index must be a safe integer, got ${safeFormatValue(index)}`);
  }
  if (validIndex > MAX_SAMPLES) {
    throw new RangeError(
      `index ${safeFormatValue(validIndex)} exceeds MAX_SAMPLES ${safeFormatValue(MAX_SAMPLES)}`,
    );
  }
  const sampleRate = validateSampleRate("sampleRateHz", sampleRateHz);
  const startTime = validateFinite("startTimeSeconds", startTimeSeconds);
  return startTime + validIndex / sampleRate;
}

/**
 * Warrant operation: calc_nyquist(sample_rate: Int) -> Option<Int>
 */
export function calcNyquist(sampleRate: number): number | null {
  if (typeof sampleRate !== "number" || !Number.isSafeInteger(sampleRate) || sampleRate <= 0) {
    return null;
  }
  if (sampleRate > MAX_SAMPLE_RATE_HZ) {
    return null;
  }
  return Math.floor(sampleRate / 2);
}

/**
 * Warrant operation: eval_square_duty(pos_percent: Int, duty_percent: Int) -> Bool
 */
export function evalSquareDuty(posPercent: number, dutyPercent: number): boolean {
  if (
    typeof posPercent !== "number" ||
    !Number.isFinite(posPercent) ||
    typeof dutyPercent !== "number" ||
    !Number.isFinite(dutyPercent)
  ) {
    return false;
  }
  return posPercent < dutyPercent;
}
