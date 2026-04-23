import { Note, noteToFrequency, parseNote } from "../src/note_frequency";
import { describe, expect, test } from "vitest";

describe("parseNote", () => {
  test("parses a sharp note", () => {
    const note = parseNote("C#5");
    expect(note.letter).toBe("C");
    expect(note.accidental).toBe("#");
    expect(note.octave).toBe(5);
  });

  test("normalizes lowercase letters", () => {
    expect(parseNote("g4").toString()).toBe("G4");
  });

  test.each(["", "A", "H4", "#4", "4A", "A##4", "Bb"])("rejects malformed note %s", (value) => {
    expect(() => parseNote(value)).toThrow("Invalid note");
  });

  test("rejects unsupported spellings", () => {
    expect(() => new Note("E", "#", 4)).toThrow("Unsupported note spelling");
  });
});

describe("semitonesFromA4", () => {
  test("matches the reference examples", () => {
    expect(parseNote("A4").semitonesFromA4()).toBe(0);
    expect(parseNote("A5").semitonesFromA4()).toBe(12);
    expect(parseNote("A3").semitonesFromA4()).toBe(-12);
    expect(parseNote("C4").semitonesFromA4()).toBe(-9);
  });
});

describe("frequency mapping", () => {
  test("maps the reference As correctly", () => {
    expect(parseNote("A4").frequency()).toBeCloseTo(440.0, 12);
    expect(parseNote("A5").frequency()).toBeCloseTo(880.0, 12);
    expect(parseNote("A3").frequency()).toBeCloseTo(220.0, 12);
  });

  test("maps middle C", () => {
    expect(noteToFrequency("C4")).toBeCloseTo(261.6255653005986, 12);
  });

  test("treats enharmonic spellings as the same pitch", () => {
    expect(noteToFrequency("C#4")).toBeCloseTo(noteToFrequency("Db4"), 12);
  });
});
