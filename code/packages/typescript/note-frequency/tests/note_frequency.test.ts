import assert from "node:assert/strict";
import { describe, it, expect, assertProperty, Gen } from "@coding-adventures/test-bootstrap";
import { Note, noteToFrequency, parseNote } from "../src/index.js";

describe("parseNote core functionality", () => {
  it("parses a sharp note", () => {
    const note = parseNote("C#5");
    expect(note.letter).toBe("C");
    expect(note.accidental).toBe("#");
    expect(note.octave).toBe(5);
  });

  it("normalizes lowercase letters", () => {
    expect(parseNote("g4").toString()).toBe("G4");
  });

  it("rejects malformed note strings", () => {
    const malformed = ["", "A", "H4", "#4", "4A", "A##4", "Bb", "C 4", "D#", "X9"];
    for (const val of malformed) {
      expect(() => parseNote(val)).toThrow();
    }
  });

  it("rejects unsupported spellings", () => {
    expect(() => new Note("E", "#", 4)).toThrow("Unsupported note spelling");
    expect(() => new Note("B", "#", 4)).toThrow("Unsupported note spelling");
    expect(() => new Note("C", "b", 4)).toThrow("Unsupported note spelling");
    expect(() => new Note("F", "b", 4)).toThrow("Unsupported note spelling");
  });
});

describe("semitonesFromA4", () => {
  it("matches the reference examples", () => {
    expect(parseNote("A4").semitonesFromA4()).toBe(0);
    expect(parseNote("A5").semitonesFromA4()).toBe(12);
    expect(parseNote("A3").semitonesFromA4()).toBe(-12);
    expect(parseNote("C4").semitonesFromA4()).toBe(-9);
  });
});

describe("frequency mapping", () => {
  it("maps the reference As correctly", () => {
    expect(parseNote("A4").frequency()).toBeCloseTo(440.0, 5);
    expect(parseNote("A5").frequency()).toBeCloseTo(880.0, 5);
    expect(parseNote("A3").frequency()).toBeCloseTo(220.0, 5);
  });

  it("maps middle C", () => {
    expect(noteToFrequency("C4")).toBeCloseTo(261.6255653005986, 5);
  });

  it("treats enharmonic spellings as the same pitch", () => {
    expect(noteToFrequency("C#4")).toBeCloseTo(noteToFrequency("Db4"), 5);
    expect(noteToFrequency("D#4")).toBeCloseTo(noteToFrequency("Eb4"), 5);
    expect(noteToFrequency("F#4")).toBeCloseTo(noteToFrequency("Gb4"), 5);
    expect(noteToFrequency("G#4")).toBeCloseTo(noteToFrequency("Ab4"), 5);
    expect(noteToFrequency("A#4")).toBeCloseTo(noteToFrequency("Bb4"), 5);
  });
});

describe("CWE-1321: Prototype Pollution & Object Key Hardening", () => {
  it("rejects prototype property names in parseNote", () => {
    const malicious = ["__proto__", "constructor", "prototype", "toString", "valueOf"];
    for (const key of malicious) {
      expect(() => parseNote(key)).toThrow();
      expect(() => parseNote(`${key}4`)).toThrow();
    }
  });

  it("rejects prototype property names in Note constructor", () => {
    expect(() => new Note("__proto__", "", 4)).toThrow();
    expect(() => new Note("C", "__proto__", 4)).toThrow();
    expect(() => new Note("constructor", "", 4)).toThrow();
    expect(() => new Note("prototype", "", 4)).toThrow();
  });

  it("verifies Object.prototype was not polluted", () => {
    const freshObj: Record<string, unknown> = {};
    expect(freshObj["polluted"]).toBeUndefined();
    expect(freshObj["chromaticIndex"]).toBeUndefined();
  });
});

describe("CWE-400: Resource Consumption & Input Length Caps", () => {
  it("rejects oversized strings exceeding 32 characters", () => {
    const hugeInput = "A" + "4".repeat(100);
    assert.throws(() => parseNote(hugeInput), RangeError);
  });

  it("rejects out-of-range octaves to prevent numeric overflow", () => {
    assert.throws(() => parseNote("A101"), RangeError);
    assert.throws(() => parseNote("A-101"), RangeError);
    assert.throws(() => new Note("A", "", 9999), RangeError);
    assert.throws(() => new Note("A", "", -9999), RangeError);
  });
});

describe("CWE-20: Improper Input Validation & Type Safety", () => {
  it("rejects non-string types for parseNote", () => {
    for (const bad of [null, undefined, 440, {}, [], true]) {
      // @ts-expect-error Testing runtime validation for hostile inputs
      assert.throws(() => parseNote(bad), TypeError);
    }
  });

  it("rejects non-finite and float octaves", () => {
    for (const badOctave of [NaN, Infinity, -Infinity, 4.5, -3.2]) {
      assert.throws(() => new Note("A", "", badOctave), RangeError);
    }
  });
});

describe("PBT & Invariant Testing (VER05a)", () => {
  it("proves doubling invariant for octave shifts across all valid notes", () => {
    const validLetters = ["C", "D", "E", "F", "G", "A", "B"];
    for (const letter of validLetters) {
      for (let oct = -20; oct <= 20; oct++) {
        const n1 = new Note(letter, "", oct);
        const n2 = new Note(letter, "", oct + 1);
        expect(n2.semitonesFromA4() - n1.semitonesFromA4()).toBe(12);
        expect(n2.frequency() / n1.frequency()).toBeCloseTo(2.0, 5);
      }
    }
  });

  it("executes generative property test for valid pitch bounds", () => {
    const noteGen = new Gen((rng) => {
      const letters = ["C", "D", "E", "F", "G", "A", "B"];
      const letter = letters[rng.bounded(letters.length)];
      const oct = rng.bounded(101) - 50; // -50 to 50
      return { letter, oct };
    });

    assertProperty(noteGen, ({ letter, oct }) => {
      const note = new Note(letter, "", oct);
      const freq = note.frequency();
      return Number.isFinite(freq) && freq > 0;
    }, { numTests: 100 });
  });

  it("executes hostile fuzzing inputs without uncaught exceptions", () => {
    const hostileInputs = [
      "",
      "__proto__",
      "constructor",
      "prototype",
      "A-99999999999999",
      "A+99999999999999",
      "\x00\x00\x00",
      "NaN",
      "Infinity",
      "C#5",
      "Db3",
      "A4",
      "g4",
      "H99",
      "A".repeat(100),
    ];
    const hostileGen = new Gen((rng) => hostileInputs[rng.bounded(hostileInputs.length)]);

    assertProperty(hostileGen, (input) => {
      try {
        const n = parseNote(input);
        return Number.isFinite(n.frequency());
      } catch (err) {
        // Must fail closed with standard error types
        return err instanceof Error;
      }
    }, { numTests: 100 });
  });
});
