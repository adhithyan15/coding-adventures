/**
 * Equal-tempered frequency to note inversion (MUS08, MUS00).
 */

import {
  DEFAULT_A4_HZ,
  MAX_SAMPLE_RATE_HZ,
  NotePitch,
  validatePositiveFloat,
} from "./types.js";

/**
 * Canonical chromatic pitch class names in ascending semitone order.
 */
export const PITCH_NAMES = [
  "C",
  "C#",
  "D",
  "D#",
  "E",
  "F",
  "F#",
  "G",
  "G#",
  "A",
  "A#",
  "B",
] as const;

/**
 * Convert a positive frequency in Hz to nearest note name, MIDI index, and cents deviation.
 *
 * @param hz Fundamental frequency in Hertz (> 0.0, finite, <= 96,000 Hz).
 * @param a4Hz Tuning reference frequency for A4 in Hertz (default 440.0 Hz, <= 10,000 Hz).
 * @returns NotePitch object with canonical name, MIDI index, and cents deviation.
 */
export function frequencyToNote(
  hz: number,
  a4Hz: number = DEFAULT_A4_HZ,
): NotePitch {
  const validHz = validatePositiveFloat(hz, "hz", MAX_SAMPLE_RATE_HZ / 2.0);
  const validA4 = validatePositiveFloat(a4Hz, "a4_hz", 10_000.0);

  // Invert equal-tempered frequency formula: f = a4 * 2^((midi - 69) / 12)
  const semitonesFromA4 = 12.0 * Math.log2(validHz / validA4);
  const exactMidi = 69.0 + semitonesFromA4;
  const nearestMidi = Math.floor(exactMidi + 0.5);
  const cents = (exactMidi - nearestMidi) * 100.0;

  const pitchIndex = ((nearestMidi % 12) + 12) % 12;
  const pitchClass = PITCH_NAMES[pitchIndex];
  const octave = Math.floor(nearestMidi / 12) - 1;
  const noteName = `${pitchClass}${octave}`;

  const roundedCents = Math.round(cents * 100.0) / 100.0;

  return {
    note: noteName,
    midi: nearestMidi,
    cents: roundedCents,
  };
}

/**
 * Convert frequency to note using the standard 440.0 Hz tuning reference.
 */
export function frequencyToNoteA440(hz: number): NotePitch {
  return frequencyToNote(hz, DEFAULT_A4_HZ);
}

/**
 * Convert frequency to note with an optional A4 tuning reference (defaults to 440.0 Hz).
 */
export function frequencyToNoteOpt(
  hz: number,
  a4Hz?: number,
): NotePitch {
  return frequencyToNote(hz, a4Hz ?? DEFAULT_A4_HZ);
}

// Snake-case aliases
export const frequency_to_note = frequencyToNote;
export const frequency_to_note_a440 = frequencyToNoteA440;
export const frequency_to_note_opt = frequencyToNoteOpt;
