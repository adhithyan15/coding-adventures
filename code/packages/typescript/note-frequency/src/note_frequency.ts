const NOTE_PATTERN = /^([A-Ga-g])([#b]?)(-?\d+)$/;
const CHROMATIC_INDEX: Record<string, number> = {
  C: 0,
  "C#": 1,
  Db: 1,
  D: 2,
  "D#": 3,
  Eb: 3,
  E: 4,
  F: 5,
  "F#": 6,
  Gb: 6,
  G: 7,
  "G#": 8,
  Ab: 8,
  A: 9,
  "A#": 10,
  Bb: 10,
  B: 11,
};
const REFERENCE_OCTAVE = 4;
const REFERENCE_INDEX = CHROMATIC_INDEX.A;
const REFERENCE_FREQUENCY_HZ = 440.0;
const SEMITONES_PER_OCTAVE = 12;

export class Note {
  readonly letter: string;
  readonly accidental: string;
  readonly octave: number;

  constructor(letter: string, accidental: string, octave: number) {
    const canonicalLetter = letter.toUpperCase();
    const spelling = canonicalLetter + accidental;
    if (!(spelling in CHROMATIC_INDEX)) {
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
    return CHROMATIC_INDEX[this.spelling()];
  }

  semitonesFromA4(): number {
    const octaveOffset = (this.octave - REFERENCE_OCTAVE) * SEMITONES_PER_OCTAVE;
    const pitchOffset = this.chromaticIndex() - REFERENCE_INDEX;
    return octaveOffset + pitchOffset;
  }

  frequency(): number {
    return REFERENCE_FREQUENCY_HZ * 2 ** (this.semitonesFromA4() / SEMITONES_PER_OCTAVE);
  }

  toString(): string {
    return `${this.spelling()}${this.octave}`;
  }
}

export function parseNote(text: string): Note {
  const match = NOTE_PATTERN.exec(text);
  if (match === null) {
    throw new Error(
      `Invalid note ${JSON.stringify(text)}. ` +
        "Expected <letter><optional # or b><octave>, for example 'A4', 'C#5', or 'Db3'."
    );
  }
  const [, letter, accidental, octaveText] = match;
  return new Note(letter, accidental, Number.parseInt(octaveText, 10));
}

export function noteToFrequency(text: string): number {
  return parseNote(text).frequency();
}
