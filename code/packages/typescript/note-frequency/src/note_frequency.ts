/**
 * MUS00: Note-to-Frequency Mapping (TypeScript implementation).
 *
 * Converts musical note names (e.g. "A4", "C#5", "Db3") into equal-tempered
 * frequencies in Hertz anchored at A4 = 440 Hz.
 *
 * Hardened against:
 * - CWE-1321 (Prototype Pollution): Uses Map and Set for lookups, rejects prototype properties.
 * - CWE-1333 (ReDoS): Uses a bounded linear-time ASCII scanner instead of regexes.
 * - CWE-400 (Uncontrolled Resource Consumption): Caps input length and octave ranges.
 * - CWE-20 / CWE-190 (Improper Input Validation / Integer Overflow): Validates all types,
 *   safe integer bounds, and finite numbers.
 */

export const MAX_NOTE_STRING_LENGTH = 32;
export const MIN_OCTAVE = -100;
export const MAX_OCTAVE = 100;

const BANNED_PROPERTIES = new Set(["__proto__", "constructor", "prototype", "toString", "valueOf"]);

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

function invalidNoteError(text: string): Error {
  return new Error(
    `Invalid note ${JSON.stringify(text)}. ` +
      "Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
  );
}

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
    if (letter.length > MAX_NOTE_STRING_LENGTH || accidental.length > MAX_NOTE_STRING_LENGTH) {
      throw new RangeError(
        `Note component lengths (${letter.length}, ${accidental.length}) exceed maximum ${MAX_NOTE_STRING_LENGTH}.`
      );
    }
    if (letter.length !== 1 || accidental.length > 1) {
      throw new Error(
        `Unsupported note spelling ${JSON.stringify(letter + accidental)}. ` +
          "Only natural notes plus single # or b accidentals are supported."
      );
    }
    if (!Number.isSafeInteger(octave)) {
      throw new RangeError(`Octave must be a safe finite integer, got ${octave}`);
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
    if (!Number.isSafeInteger(semitones)) {
      throw new RangeError("Computed semitones is not a safe integer");
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

/**
 * Parse a note label with a bounded linear-time ASCII scanner (CWE-1333 / CWE-400).
 *
 * Accepted grammar:
 *   <letter> ::= 'A'..'G' | 'a'..'g'
 *   <accidental> ::= '' | '#' | 'b'
 *   <octave> ::= ['-'] '0'..'9'+
 */
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

  const firstCode = text.charCodeAt(0);
  let letter: string;
  if (firstCode >= 65 && firstCode <= 71) {
    letter = text[0];
  } else if (firstCode >= 97 && firstCode <= 103) {
    letter = String.fromCharCode(firstCode - 32);
  } else {
    throw invalidNoteError(text);
  }

  let pos = 1;
  let accidental = "";
  if (pos < text.length) {
    const accCode = text.charCodeAt(pos);
    if (accCode === 35 || accCode === 98) {
      accidental = text[pos];
      pos += 1;
    }
  }

  if (pos >= text.length) {
    throw invalidNoteError(text);
  }

  let negative = false;
  if (text.charCodeAt(pos) === 45) {
    negative = true;
    pos += 1;
  }

  if (pos >= text.length) {
    throw invalidNoteError(text);
  }

  let octaveAbs = 0;
  for (let i = pos; i < text.length; i += 1) {
    const digitCode = text.charCodeAt(i);
    if (digitCode < 48 || digitCode > 57) {
      throw invalidNoteError(text);
    }
    octaveAbs = octaveAbs * 10 + (digitCode - 48);
    if (octaveAbs > MAX_OCTAVE) {
      throw new RangeError(
        `Octave in ${JSON.stringify(text)} out of range [${MIN_OCTAVE}, ${MAX_OCTAVE}]`
      );
    }
  }

  const parsedOctave = negative ? (octaveAbs === 0 ? 0 : -octaveAbs) : octaveAbs;
  return new Note(letter, accidental, parsedOctave);
}

export function noteToFrequency(text: string): number {
  return parseNote(text).frequency();
}
