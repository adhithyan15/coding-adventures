/**
 * Domain validation helpers (VER15 / CWE-20 / CWE-190 / CWE-400 / CWE-1321 / CWE-1333).
 * Strictly iterative, zero regex, prototype pollution guarded.
 */

import {
  getInstrument,
  instrumentForGmProgram,
  parseNote,
} from "@coding-adventures/musical-instruments";

import {
  BANNED_PROPERTIES,
  CANONICAL_SHARP_SPELLINGS,
  DEFAULT_INSTRUMENT_ID,
  DURATION_BEATS,
  MAX_SAMPLE_RATE_HZ,
  MIN_SAMPLE_RATE_HZ,
  SEMITONES_PER_OCTAVE,
} from "./constants.js";

/**
 * Guard against prototype pollution property injection (CWE-1321).
 */
export function assertSafeProperty(name: string): void {
  if (typeof name !== "string") {
    throw new TypeError("property name must be a string");
  }
  if (BANNED_PROPERTIES.has(name)) {
    throw new Error(`Forbidden property name: ${name}`);
  }
}

/**
 * Linear-time identifier scanner matching ^[A-Za-z_][A-Za-z0-9_-]*$ (zero regex, CWE-1333).
 */
export function isIdentifier(text: string): boolean {
  if (typeof text !== "string" || text.length === 0) {
    return false;
  }
  if (BANNED_PROPERTIES.has(text)) {
    return false;
  }
  const first = text.charCodeAt(0);
  const isAlpha =
    (first >= 65 && first <= 90) || // A-Z
    (first >= 97 && first <= 122) || // a-z
    first === 95; // _
  if (!isAlpha) {
    return false;
  }
  for (let i = 1; i < text.length; i += 1) {
    const ch = text.charCodeAt(i);
    const validChar =
      (ch >= 65 && ch <= 90) ||
      (ch >= 97 && ch <= 122) ||
      (ch >= 48 && ch <= 57) || // 0-9
      ch === 95 || // _
      ch === 45; // -
    if (!validChar) {
      return false;
    }
  }
  return true;
}

/**
 * Validate an identifier string.
 */
export function validateIdentifier(label: string, text: string): string {
  assertSafeProperty(text);
  if (!isIdentifier(text)) {
    throw new RangeError(
      `${label} must be an identifier matching [A-Za-z_][A-Za-z0-9_-]*, got '${text}'`,
    );
  }
  return text;
}

/**
 * Linear-time meter scanner matching ^([1-9][0-9]*)/([1-9][0-9]*)$ (zero regex, CWE-1333).
 */
export function isValidMeter(text: string): boolean {
  if (typeof text !== "string") {
    return false;
  }
  const slashIdx = text.indexOf("/");
  if (slashIdx <= 0 || slashIdx !== text.lastIndexOf("/")) {
    return false;
  }
  const numStr = text.slice(0, slashIdx);
  const denStr = text.slice(slashIdx + 1);
  if (numStr.length === 0 || denStr.length === 0) {
    return false;
  }
  if (numStr[0] === "0" || denStr[0] === "0") {
    return false;
  }
  for (let i = 0; i < numStr.length; i += 1) {
    const code = numStr.charCodeAt(i);
    if (code < 48 || code > 57) {
      return false;
    }
  }
  for (let i = 0; i < denStr.length; i += 1) {
    const code = denStr.charCodeAt(i);
    if (code < 48 || code > 57) {
      return false;
    }
  }
  return true;
}

/**
 * Validate meter text.
 */
export function validateMeter(text: string): string {
  if (!isValidMeter(text)) {
    throw new RangeError(
      `meter must look like positive beats/beat-unit, got '${text}'`,
    );
  }
  return text;
}

/**
 * Parse meter string into [numerator, denominator].
 */
export function meterParts(meter: string): [number, number] {
  validateMeter(meter);
  const slashIdx = meter.indexOf("/");
  const num = Number.parseInt(meter.slice(0, slashIdx), 10);
  const den = Number.parseInt(meter.slice(slashIdx + 1), 10);
  return [num, den];
}

/**
 * Validate a finite number.
 */
export function validateFinite(label: string, val: number): number {
  if (typeof val !== "number" || !Number.isFinite(val) || Number.isNaN(val)) {
    throw new TypeError(`${label} must be a finite number, got ${String(val)}`);
  }
  return val;
}

/**
 * Validate a strictly positive finite number.
 */
export function validatePositive(label: string, val: number): number {
  validateFinite(label, val);
  if (val <= 0.0) {
    throw new RangeError(`${label} must be > 0.0, got ${String(val)}`);
  }
  return val;
}

/**
 * Validate a non-negative finite number.
 */
export function validateNonNegative(label: string, val: number): number {
  validateFinite(label, val);
  if (val < 0.0) {
    throw new RangeError(`${label} must be >= 0.0, got ${String(val)}`);
  }
  return val;
}

/**
 * Validate a unit float in [0.0, 1.0].
 */
export function validateUnitFloat(label: string, val: number): number {
  validateFinite(label, val);
  if (val < 0.0 || val > 1.0) {
    throw new RangeError(`${label} must be in [0.0, 1.0], got ${String(val)}`);
  }
  return val;
}

/**
 * Validate a safe integer.
 */
export function validateSafeInteger(label: string, val: number): number {
  if (
    typeof val !== "number" ||
    !Number.isFinite(val) ||
    !Number.isSafeInteger(val)
  ) {
    throw new TypeError(`${label} must be a safe integer, got ${String(val)}`);
  }
  return val;
}

/**
 * Validate a strictly positive safe integer.
 */
export function validatePositiveInt(label: string, val: number): number {
  validateSafeInteger(label, val);
  if (val <= 0) {
    throw new RangeError(`${label} must be a positive integer, got ${String(val)}`);
  }
  return val;
}

/**
 * Validate a non-negative safe integer.
 */
export function validateNonNegativeInt(label: string, val: number): number {
  validateSafeInteger(label, val);
  if (val < 0) {
    throw new RangeError(
      `${label} must be an integer >= 0, got ${String(val)}`,
    );
  }
  return val;
}

/**
 * Validate sample rate in [MIN_SAMPLE_RATE_HZ, MAX_SAMPLE_RATE_HZ].
 */
export function validateSampleRate(sampleRateHz: number): number {
  validatePositiveInt("sample_rate_hz", sampleRateHz);
  if (sampleRateHz < MIN_SAMPLE_RATE_HZ || sampleRateHz > MAX_SAMPLE_RATE_HZ) {
    throw new RangeError(
      `sample_rate_hz must be in [${String(MIN_SAMPLE_RATE_HZ)}, ${String(MAX_SAMPLE_RATE_HZ)}], got ${String(sampleRateHz)}`,
    );
  }
  return sampleRateHz;
}

/**
 * Return quarter-note beat count for a duration symbol.
 */
export function beatsForDurationSymbol(symbol: string): number {
  if (typeof symbol !== "string" || symbol.length === 0) {
    throw new RangeError("duration symbol must be non-empty text");
  }
  let dotCount = 0;
  for (let i = 0; i < symbol.length; i += 1) {
    if (symbol[i] === ".") {
      dotCount += 1;
    }
  }
  if (dotCount > 1) {
    throw new RangeError(
      `duration symbol '${symbol}' may have at most one dot`,
    );
  }
  const isDotted = symbol.endsWith(".");
  const baseSymbol = isDotted ? symbol.slice(0, -1) : symbol;
  const baseBeats = Object.prototype.hasOwnProperty.call(DURATION_BEATS, baseSymbol)
    ? DURATION_BEATS[baseSymbol]
    : undefined;
  if (baseBeats === undefined) {
    throw new RangeError(`unknown duration symbol '${symbol}'`);
  }
  return isDotted ? baseBeats * 1.5 : baseBeats;
}

/**
 * Safe rounding of seconds * sample rate without overflow (CWE-190).
 */
export function safeRoundSample(seconds: number, rate: number): number {
  validateNonNegative("seconds", seconds);
  validatePositive("rate", rate);
  const exact = seconds * rate;
  if (!Number.isFinite(exact)) {
    throw new RangeError("seconds * rate produced non-finite value");
  }
  const rounded = Math.round(exact);
  if (!Number.isSafeInteger(rounded)) {
    throw new RangeError("sample index exceeds safe integer range");
  }
  return rounded;
}

/**
 * Transpose a musical note text by a semitone offset.
 */
export function transposeNoteText(text: string, semitoneOffset: number): string {
  if (typeof text !== "string") {
    throw new TypeError("note text must be a string");
  }
  validateSafeInteger("semitoneOffset", semitoneOffset);
  if (semitoneOffset === 0) {
    return text;
  }
  const parsed = parseNote(text);
  const absoluteIndex =
    (parsed.octave + 1) * SEMITONES_PER_OCTAVE + parsed.chromaticIndex();
  const transposedIndex = absoluteIndex + semitoneOffset;
  const chromaticIndex =
    ((transposedIndex % SEMITONES_PER_OCTAVE) + SEMITONES_PER_OCTAVE) %
    SEMITONES_PER_OCTAVE;
  const octave = Math.floor(transposedIndex / SEMITONES_PER_OCTAVE);
  return `${CANONICAL_SHARP_SPELLINGS[chromaticIndex]}${String(octave - 1)}`;
}

/**
 * Resolve score instrument settings to a concrete profile ID.
 */
export function resolveInstrumentId(
  instrumentId?: string,
  gmProgram?: number,
): string {
  if (instrumentId !== undefined && gmProgram !== undefined) {
    throw new RangeError("score may specify instrument or program, not both");
  }
  if (gmProgram !== undefined) {
    validatePositiveInt("program", gmProgram);
    if (gmProgram < 1 || gmProgram > 128) {
      throw new RangeError(
        `program must be in [1, 128], got ${String(gmProgram)}`,
      );
    }
    const p = instrumentForGmProgram(gmProgram);
    return p.id;
  }
  if (instrumentId !== undefined) {
    if (
      instrumentId === "drumkit" ||
      instrumentId === "standard_kit" ||
      instrumentId === "standard"
    ) {
      return "drumkit";
    }
    const p = getInstrument(instrumentId);
    return p.id;
  }
  return DEFAULT_INSTRUMENT_ID;
}
