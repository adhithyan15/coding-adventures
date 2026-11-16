/**
 * MUS00: Note-to-Frequency Mapping (TypeScript implementation).
 *
 * Converts musical note names (e.g. "A4", "C#5", "Db3") into equal-tempered
 * frequencies in Hertz anchored at A4 = 440 Hz.
 *
 * Hardened against:
 * - CWE-1321 (Prototype Pollution): Uses Map for lookups, rejects prototype properties.
 * - CWE-400 (Uncontrolled Resource Consumption): Caps input length and octave ranges.
 * - CWE-20 (Improper Input Validation): Validates all types, bounds, and finite numbers.
 */

const MAX_NOTE_STRING_LENGTH = 32;
const MIN_OCTAVE = -100;
const MAX_OCTAVE = 100;

const BANNED_PROPERTIES = new Set(["__proto__", "constructor", "prototype", "toString", "valueOf"]);

const NOTE_PATTERN = /^([A-Ga-g])([#b]?)(-?\d+)$/;

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

const REFERENCE_OCTAVE = 4;
const REFERENCE_INDEX = 9; // CHROMATIC_INDEX.get("A")!
const REFERENCE_FREQUENCY_HZ = 440.0;
const SEMITONES_PER_OCTAVE = 12;

export class Note {
  readonly letter: string;
  readonly accidental: string;
  readonly octave: number;

  constructor(letter: string, accidental: string, octave: number) {
    if (typeof letter !== "string" || typeof accidental !== "string" || typeof octave !== "number") {
      throw new TypeError("Note components must be strings and a finite integer octave.");
    }
    if (BANNED_PROPERTIES.has(letter) || BANNED_PROPERTIES.has(accidental)) {
      throw new Error(`Forbidden prototype property name in note: ${letter}${accidental}`);
    }
    if (!Number.isFinite(octave) || !Number.isInteger(octave)) {
      throw new RangeError(`Octave must be a finite integer, got ${octave}`);
    }
    if (octave < MIN_OCTAVE || octave > MAX_OCTAVE) {
      throw new RangeError(`Octave ${octave} out of range [${MIN_OCTAVE}, ${MAX_OCTAVE}]`);
    }

    const canonicalLetter = letter.toUpperCase();
    const spelling = canonicalLetter + accidental;

    if (BANNED_PROPERTIES.has(spelling) || !CHROMATIC_INDEX.has(spelling)) {
      throw new Error(
        `Unsupported note spelling ${JSON.stringify(spelling)}. ` +
          "Only natural notes plus single # or b accidentals are supported."
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
      throw new Error(`Unknown chromatic index for spelling ${this.spelling()}`);
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
    const freq = REFERENCE_FREQUENCY_HZ * 2 ** (this.semitonesFromA4() / SEMITONES_PER_OCTAVE);
    if (!Number.isFinite(freq) || freq <= 0) {
      throw new RangeError(`Computed frequency is invalid: ${freq}`);
    }
    return freq;
  }

  toString(): string {
    return `${this.spelling()}${this.octave}`;
  }
}

export function parseNote(text: string): Note {
  if (typeof text !== "string") {
    throw new TypeError(`Expected string input for note, got ${typeof text}`);
  }
  if (text.length === 0 || text.length > MAX_NOTE_STRING_LENGTH) {
    throw new RangeError(
      `Note string length must be between 1 and ${MAX_NOTE_STRING_LENGTH} characters, got ${text.length}`
    );
  }
  if (BANNED_PROPERTIES.has(text)) {
    throw new Error(`Rejected prototype property name: ${text}`);
  }

  const match = NOTE_PATTERN.exec(text);
  if (match === null) {
    throw new Error(
      `Invalid note ${JSON.stringify(text)}. ` +
        "Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
    );
  }
  const [, letter, accidental, octaveText] = match;
  const parsedOctave = Number.parseInt(octaveText, 10);
  if (!Number.isFinite(parsedOctave)) {
    throw new RangeError(`Parsed octave is not finite: ${octaveText}`);
  }
  return new Note(letter, accidental, parsedOctave);
}

export function noteToFrequency(text: string): number {
  return parseNote(text).frequency();
}
