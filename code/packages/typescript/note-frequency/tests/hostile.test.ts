import assert from "node:assert/strict";
import test from "node:test";
import { SplitMix64 } from "@coding-adventures/test-bootstrap";

import {
  MAX_NOTE_STRING_LENGTH,
  MAX_OCTAVE,
  MIN_OCTAVE,
  Note,
  noteToFrequency,
  parseNote,
} from "../src/index.js";

test("CWE-1333 / CWE-400: linear-time scanner rejects pathological and oversized strings", () => {
  // Pathological inputs that would trigger catastrophic backtracking in naive regexes
  assert.throws(() => parseNote("A" + "#".repeat(10_000)), RangeError);
  assert.throws(() => parseNote("A" + "0".repeat(10_000)), RangeError);
  assert.throws(() => parseNote("A" + " ".repeat(31)), Error);
  assert.throws(() => parseNote("A" + "0".repeat(MAX_NOTE_STRING_LENGTH)), RangeError);
  assert.equal(parseNote("A" + "0".repeat(MAX_NOTE_STRING_LENGTH - 1)).toString(), "A0");
  assert.throws(() => parseNote(""), RangeError);

  assert.throws(
    () => new Note("A".repeat(200_000), "#".repeat(200_000), 4),
    (err: unknown) => err instanceof RangeError && err.message.length < 256
  );
  assert.throws(() => new Note("AA", "", 4), Error);
  assert.throws(() => new Note("A", "##", 4), Error);
});

test("CWE-20 / CWE-190: rejects unsafe integers, non-ASCII digits, signs, and invalid components", () => {
  assert.throws(() => new Note("A", "", Number.MAX_SAFE_INTEGER + 1), RangeError);
  assert.throws(() => new Note("A", "", Number.MIN_SAFE_INTEGER - 1), RangeError);
  assert.throws(() => new Note("A", "", 1e20), RangeError);
  assert.throws(() => new Note("A", "", MIN_OCTAVE - 1), RangeError);
  assert.throws(() => new Note("A", "", MAX_OCTAVE + 1), RangeError);

  // @ts-expect-error testing non-string letter
  assert.throws(() => new Note(42, "", 4), TypeError);
  // @ts-expect-error testing non-string accidental
  assert.throws(() => new Note("A", null, 4), TypeError);
  // @ts-expect-error testing non-number octave
  assert.throws(() => new Note("A", "", "4"), TypeError);

  const malformed = [
    "A+4",
    "A-",
    "A--4",
    "A4 ",
    " A4",
    "A4\n",
    "A4\x00",
    "A\u0664", // Arabic-Indic digit
    "A\uFF14", // Fullwidth digit
    "C##4",
    "Cbb4",
    "Cb4",
    "E#4",
    "B#4",
    "Fb4",
    "G-101",
    "G101",
    "A99999999999999999999",
  ];
  for (const bad of malformed) {
    assert.throws(() => parseNote(bad), Error);
  }
});

test("CWE-1321: prototype pollution keys and Object.prototype remain safe", () => {
  for (const key of ["__proto__", "constructor", "prototype", "toString", "valueOf"]) {
    assert.throws(() => parseNote(key), Error);
    assert.throws(() => new Note(key, "", 4), Error);
    assert.throws(() => new Note("C", key, 4), Error);
  }
  assert.equal(({} as Record<string, unknown>)["C"], undefined);
});

test("Fuzzing hostile inputs with SplitMix64 never panics or returns non-finite frequency", () => {
  const prng = new SplitMix64(0xdead_beef_cafe_1234n);
  const alphabet = "ABCDEFGHabcdefg#b-0123456789+\n\x00 _";
  for (let i = 0; i < 500; i += 1) {
    const len = prng.bounded(48);
    let candidate = "";
    for (let j = 0; j < len; j += 1) {
      candidate += alphabet[prng.bounded(alphabet.length)];
    }
    try {
      const freq = noteToFrequency(candidate);
      assert.ok(Number.isFinite(freq) && freq > 0);
    } catch (err) {
      assert.ok(err instanceof Error);
    }
  }
});
