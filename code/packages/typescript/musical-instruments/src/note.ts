/**
 * Musical note representation and equal-tempered frequency mapping (MUS00 / MUS05).
 * Strictly iterative, zero regex (CWE-1333), protected against prototype pollution (CWE-1321).
 */

import { MAX_OCTAVE, MIN_OCTAVE } from "./constants.js";
import type { NoteInput } from "./types.js";

const REFERENCE_OCTAVE = 4;
const REFERENCE_INDEX = 9; // A
const REFERENCE_FREQUENCY_HZ = 440.0;
const SEMITONES_PER_OCTAVE = 12;
const MAX_NOTE_STRING_LENGTH = 32;

const BANNED_NAMES = new Set([
  "__proto__",
  "constructor",
  "prototype",
  "toString",
  "valueOf",
]);

const CHROMATIC_INDEX: ReadonlyMap<string, number> = new Map<string, number>([
  ["C", 0],
  ["C#", 1],
  ["Db", 1],
  ["D", 2],
  ["D#", 3],
  ["Eb", 3],
  ["E", 4],
  ["F", 5],
  ["F#", 6],
  ["Gb", 6],
  ["G", 7],
  ["G#", 8],
  ["Ab", 8],
  ["A", 9],
  ["A#", 10],
  ["Bb", 10],
  ["B", 11],
]);

export class Note {
  readonly letter: string;
  readonly accidental: string;
  readonly octave: number;

  constructor(letter: string, accidental: string, octave: number) {
    if (
      typeof letter !== "string" ||
      typeof accidental !== "string" ||
      typeof octave !== "number"
    ) {
      throw new TypeError("Note components must be strings and a finite integer octave");
    }
    if (BANNED_NAMES.has(letter) || BANNED_NAMES.has(accidental)) {
      throw new Error(`Forbidden property name in note: ${letter}${accidental}`);
    }
    if (!Number.isFinite(octave) || !Number.isInteger(octave)) {
      throw new RangeError(`Octave must be a finite integer, got ${String(octave)}`);
    }
    if (octave < MIN_OCTAVE || octave > MAX_OCTAVE) {
      throw new RangeError(`Octave ${String(octave)} out of range [${String(MIN_OCTAVE)}, ${String(MAX_OCTAVE)}]`);
    }

    const canonicalLetter = letter.toUpperCase();
    const spelling = canonicalLetter + accidental;

    if (!CHROMATIC_INDEX.has(spelling)) {
      throw new Error(
        `Unsupported note spelling "${spelling}". Only natural notes plus single # or b accidentals are supported.`,
      );
    }

    this.letter = canonicalLetter;
    this.accidental = accidental;
    this.octave = octave;
  }

  spelling(): string {
    return `${this.letter}${this.accidental}`;
  }

  chromaticIndex(): number {
    const idx = CHROMATIC_INDEX.get(this.spelling());
    if (idx === undefined) {
      throw new Error(`Unknown chromatic index for spelling "${this.spelling()}"`);
    }
    return idx;
  }

  semitonesFromA4(): number {
    const octaveOffset = (this.octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE;
    const pitchOffset = this.chromaticIndex() - REFERENCE_INDEX;
    const semitones = octaveOffset + pitchOffset;
    if (!Number.isFinite(semitones)) {
      throw new RangeError("Computed semitones is not a finite number");
    }
    return semitones;
  }

  frequency(): number {
    const semitones = this.semitonesFromA4();
    const freq = REFERENCE_FREQUENCY_HZ * 2.0 ** (semitones / SEMITONES_PER_OCTAVE);
    if (!Number.isFinite(freq) || freq <= 0.0) {
      throw new RangeError(`Computed frequency is invalid: ${String(freq)}`);
    }
    return freq;
  }

  toString(): string {
    return `${this.spelling()}${String(this.octave)}`;
  }
}

/**
 * Parse a note string such as "A4", "C#5", or "Db3" without regular expressions (CWE-1333).
 */
export function parseNote(text: string): Note {
  if (typeof text !== "string") {
    throw new TypeError(`Expected string input for note, got ${typeof text}`);
  }
  const len = text.length;
  if (len < 2 || len > MAX_NOTE_STRING_LENGTH) {
    throw new RangeError(
      `Note string length must be between 2 and ${String(MAX_NOTE_STRING_LENGTH)} characters, got ${String(len)}`,
    );
  }
  if (BANNED_NAMES.has(text)) {
    throw new Error(`Rejected prototype property name: ${text}`);
  }

  const firstChar = text[0].toUpperCase();
  if (firstChar < "A" || firstChar > "G") {
    throw new Error(
      `Invalid note "${text}". Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.`,
    );
  }

  let accidental = "";
  let octaveStartIndex = 1;

  if (text[1] === "#" || text[1] === "b") {
    accidental = text[1];
    octaveStartIndex = 2;
  }

  if (octaveStartIndex >= len) {
    throw new Error(
      `Invalid note "${text}". Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'.`,
    );
  }

  let isNegative = false;
  let digitStartIndex = octaveStartIndex;
  if (text[digitStartIndex] === "-") {
    isNegative = true;
    digitStartIndex += 1;
    if (digitStartIndex >= len) {
      throw new Error(
        `Invalid note "${text}". Negative sign must be followed by digits.`,
      );
    }
  }

  let parsedMagnitude = 0;
  for (let i = digitStartIndex; i < len; i += 1) {
    const code = text.charCodeAt(i);
    if (code < 48 || code > 57) {
      // Not '0'-'9'
      throw new Error(
        `Invalid character in octave in note "${text}": character "${text[i]}" at index ${String(i)}`,
      );
    }
    parsedMagnitude = parsedMagnitude * 10 + (code - 48);
    if (!Number.isSafeInteger(parsedMagnitude)) {
      throw new RangeError(`Octave in note "${text}" exceeds safe integer limit`);
    }
  }

  const octave = isNegative ? -parsedMagnitude : parsedMagnitude;
  return new Note(firstChar, accidental, octave);
}

export function noteToFrequency(note: NoteInput): number {
  return resolveNote(note).frequency();
}

export function resolveNote(note: NoteInput): Note {
  if (note instanceof Note) {
    return note;
  }
  if (typeof note === "string") {
    return parseNote(note);
  }
  if (
    note !== null &&
    typeof note === "object" &&
    typeof (note as Note).letter === "string" &&
    typeof (note as Note).accidental === "string" &&
    typeof (note as Note).octave === "number" &&
    typeof (note as Note).frequency === "function"
  ) {
    return note as Note;
  }
  throw new TypeError(`note must be a Note instance or note string, got ${String(note)}`);
}
