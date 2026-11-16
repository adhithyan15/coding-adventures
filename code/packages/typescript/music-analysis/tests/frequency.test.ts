import assert from "node:assert/strict";
import test from "node:test";

import {
  frequencyToNote,
  frequencyToNoteA440,
  frequencyToNoteOpt,
  frequency_to_note,
  frequency_to_note_a440,
  frequency_to_note_opt,
  InvalidInputError,
  BudgetExceededError,
  PITCH_NAMES,
} from "../src/index.js";

function noteToFrequency(noteStr: string): number {
  const pitchClasses: Record<string, number> = {
    C: 0, "C#": 1, D: 2, "D#": 3, E: 4, F: 5, "F#": 6, G: 7, "G#": 8, A: 9, "A#": 10, B: 11,
  };
  let pc = noteStr.slice(0, 1);
  let rest = noteStr.slice(1);
  if (rest.startsWith("#")) {
    pc = noteStr.slice(0, 2);
    rest = noteStr.slice(2);
  }
  const octave = parseInt(rest, 10);
  const midi = (octave + 1) * 12 + pitchClasses[pc];
  return 440.0 * Math.pow(2.0, (midi - 69) / 12.0);
}

test("frequencyToNote: standard A440 tuning", () => {
  const res = frequencyToNoteA440(440.0);
  assert.equal(res.note, "A4");
  assert.equal(res.midi, 69);
  assert.ok(Math.abs(res.cents - 0.0) < 0.1);
});

test("frequencyToNote: C4 middle C", () => {
  const c4Hz = 440.0 * Math.pow(2.0, -9.0 / 12.0);
  const res = frequencyToNoteA440(c4Hz);
  assert.equal(res.note, "C4");
  assert.equal(res.midi, 60);
  assert.ok(Math.abs(res.cents - 0.0) < 0.1);
});

test("frequencyToNote: octave extremes A0 and C8", () => {
  // A0: 27.5 Hz
  const resA0 = frequencyToNoteA440(27.5);
  assert.equal(resA0.note, "A0");
  assert.equal(resA0.midi, 21);

  // C8: ~4186 Hz
  const c8Hz = 440.0 * Math.pow(2.0, (108.0 - 69.0) / 12.0);
  const resC8 = frequencyToNoteA440(c8Hz);
  assert.equal(resC8.note, "C8");
  assert.equal(resC8.midi, 108);
});

test("frequencyToNote: alternate tuning reference A4 = 442 Hz", () => {
  const res = frequencyToNote(442.0, 442.0);
  assert.equal(res.note, "A4");
  assert.equal(res.midi, 69);
  assert.ok(Math.abs(res.cents - 0.0) < 0.1);

  const resOpt = frequencyToNoteOpt(440.0, undefined);
  assert.equal(resOpt.note, "A4");
});

test("frequencyToNote: cents deviation", () => {
  // 445 Hz is sharp relative to 440 Hz (~ +19.56 cents)
  const resSharp = frequencyToNoteA440(445.0);
  assert.equal(resSharp.note, "A4");
  assert.equal(resSharp.midi, 69);
  assert.ok(resSharp.cents > 15.0 && resSharp.cents < 25.0);

  // 435 Hz is flat relative to 440 Hz (~ -19.78 cents)
  const resFlat = frequencyToNoteA440(435.0);
  assert.equal(resFlat.note, "A4");
  assert.equal(resFlat.midi, 69);
  assert.ok(resFlat.cents < -15.0 && resFlat.cents > -25.0);
});

test("frequencyToNote: round-trip consistency with note-frequency", () => {
  const testNotes = ["C3", "D#4", "F#5", "A4", "B2", "G#3"];
  for (const noteStr of testNotes) {
    const hz = noteToFrequency(noteStr);
    const matched = frequencyToNoteA440(hz);
    assert.equal(matched.note, noteStr);
    assert.ok(Math.abs(matched.cents - 0.0) < 0.2);
  }
});

test("frequencyToNote: aliases match camelCase", () => {
  assert.deepEqual(frequency_to_note(440.0, 440.0), frequencyToNote(440.0, 440.0));
  assert.deepEqual(frequency_to_note_a440(440.0), frequencyToNoteA440(440.0));
  assert.deepEqual(frequency_to_note_opt(440.0, 440.0), frequencyToNoteOpt(440.0, 440.0));
  assert.equal(PITCH_NAMES.length, 12);
});

test("frequencyToNote: invalid and out-of-bounds inputs", () => {
  assert.throws(() => frequencyToNoteA440(0.0), InvalidInputError);
  assert.throws(() => frequencyToNoteA440(-440.0), InvalidInputError);
  assert.throws(() => frequencyToNoteA440(NaN), InvalidInputError);
  assert.throws(() => frequencyToNoteA440(Infinity), InvalidInputError);
  assert.throws(() => frequencyToNoteA440(-Infinity), InvalidInputError);
  assert.throws(() => frequencyToNote(440.0, 0.0), InvalidInputError);
  assert.throws(() => frequencyToNote(440.0, -440.0), InvalidInputError);
  assert.throws(() => frequencyToNote(440.0, NaN), InvalidInputError);
  assert.throws(() => frequencyToNote(440.0, Infinity), InvalidInputError);

  // Ceiling checks
  assert.throws(() => frequencyToNote(96001.0, 440.0), BudgetExceededError);
  assert.throws(() => frequencyToNote(440.0, 10001.0), BudgetExceededError);
});
