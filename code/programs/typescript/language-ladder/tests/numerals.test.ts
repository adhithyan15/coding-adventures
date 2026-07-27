import { describe, it, expect } from "vitest";
import { SCRIPTS } from "../src/data";
import { isSyllabary } from "../src/syllabary";
import { buildSyllableMatrix } from "../src/matrix";

const DRAVIDIAN = ["telugu", "kannada", "malayalam"] as const;

describe("script numerals (digits)", () => {
  it("Telugu carries its 10 digits, grounded glyph → value 0–9", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    const d = telugu.digits!;
    expect(d.map((x) => x.glyph)).toEqual(
      ["౦", "౧", "౨", "౩", "౪", "౫", "౬", "౭", "౮", "౯"],
    );
    // Romanization is simply the digit's value, in order 0..9.
    expect(d.map((x) => x.sound)).toEqual(
      ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
    );
    expect(d.every((x) => x.role === "digit")).toBe(true);
    expect(d.every((x) => x.strokeOrder.length === 0)).toBe(true);
  });

  it("all three Dravidian scripts carry their digits", () => {
    DRAVIDIAN.forEach((id) => {
      const s = SCRIPTS.find((x) => x.script === id)!;
      expect(s.digits?.length).toBe(10);
      expect(s.digits!.map((x) => x.sound)).toEqual(
        ["0", "1", "2", "3", "4", "5", "6", "7", "8", "9"],
      );
    });
  });

  it("CONTROL: digits are SEPARATE from the syllabary — letters, isSyllabary and the matrix are untouched", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    const syllableGlyphs = new Set(telugu.letters.map((l) => l.glyph));
    // No digit glyph leaks into the consonant syllable list…
    expect(telugu.digits!.some((x) => syllableGlyphs.has(x.glyph))).toBe(false);
    // …so the script still reads as an all-syllable syllabary, matrix intact.
    expect(isSyllabary(telugu.letters)).toBe(true);
    const m = buildSyllableMatrix(telugu.letters as never)!;
    expect(m.rows.length).toBe(35);
  });

  it("an alphabet (Cyrillic) has no digits list", () => {
    const cyr = SCRIPTS.find((s) => s.script === "cyrillic")!;
    expect(cyr.digits).toBeUndefined();
  });
});
