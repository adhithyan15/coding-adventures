import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  MAX_OCTAVE,
  MIN_OCTAVE,
  Note,
  noteToFrequency,
  parseNote,
} from "../src/index.js";

const VALID_SPELLINGS = [
  "C", "C#", "Db", "D", "D#", "Eb", "E", "F", "F#", "Gb", "G", "G#", "Ab", "A", "A#", "Bb", "B",
] as const;

const ENHARMONIC_PAIRS: readonly [string, string][] = [
  ["C#", "Db"],
  ["D#", "Eb"],
  ["F#", "Gb"],
  ["G#", "Ab"],
  ["A#", "Bb"],
];

test("PBT (SplitMix64): parseNote(note.toString()) roundtrip invariance", () => {
  const prng = new SplitMix64(0x1234_5678_9abc_def0n);
  for (let trial = 0; trial < 200; trial += 1) {
    const spelling = VALID_SPELLINGS[prng.bounded(VALID_SPELLINGS.length)];
    const letter = spelling[0];
    const accidental = spelling.slice(1);
    const octave = MIN_OCTAVE + prng.bounded(MAX_OCTAVE - MIN_OCTAVE + 1);

    const note = new Note(letter, accidental, octave);
    const reparsed = parseNote(note.toString());

    assert.equal(reparsed.letter, note.letter);
    assert.equal(reparsed.accidental, note.accidental);
    assert.equal(reparsed.octave, note.octave);
    assert.equal(reparsed.semitonesFromA4(), note.semitonesFromA4());
    assert.equal(reparsed.frequency(), note.frequency());
  }
});

test("PBT (SplitMix64): octave doubling and semitone monotonicity", () => {
  const prng = new SplitMix64(0xcafe_babe_0000_0001n);
  for (let trial = 0; trial < 150; trial += 1) {
    const spelling = VALID_SPELLINGS[prng.bounded(VALID_SPELLINGS.length)];
    const letter = spelling[0];
    const accidental = spelling.slice(1);
    const octave = -40 + prng.bounded(80); // [-40, 39] so octave + 1 stays well in bounds

    const lower = new Note(letter, accidental, octave);
    const upper = new Note(letter, accidental, octave + 1);

    assert.equal(upper.semitonesFromA4() - lower.semitonesFromA4(), 12);
    const ratio = upper.frequency() / lower.frequency();
    assert.ok(Math.abs(ratio - 2.0) < 1e-12, `Expected ratio 2.0, got ${ratio}`);
  }
});

test("PBT (SplitMix64): enharmonic equivalence and lowercase normalization", () => {
  const prng = new SplitMix64(0x9e37_79b9_7f4a_7c15n);
  for (let trial = 0; trial < 100; trial += 1) {
    const [sharp, flat] = ENHARMONIC_PAIRS[prng.bounded(ENHARMONIC_PAIRS.length)];
    const octave = -20 + prng.bounded(41);

    const fSharp = noteToFrequency(`${sharp}${octave}`);
    const fFlat = noteToFrequency(`${flat}${octave}`);
    const fLower = noteToFrequency(`${sharp.toLowerCase()}${octave}`);

    assert.ok(Math.abs(fSharp - fFlat) <= 1e-12 * Math.max(1, fSharp));
    assert.equal(fSharp, fLower);
  }
});

test("Warrant conformance: semitonesFromA4 and noteToFrequency anchor at A4 = 440 Hz", () => {
  assert.equal(parseNote("A4").semitonesFromA4(), 0);
  assert.equal(noteToFrequency("A4"), 440);
  assert.equal(parseNote("A-0").octave, 0);
});
