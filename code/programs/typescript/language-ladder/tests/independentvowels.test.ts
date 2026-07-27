import { describe, it, expect } from "vitest";
import { SCRIPTS } from "../src/data";
import { isSyllabary } from "../src/syllabary";
import { buildSyllableMatrix } from "../src/matrix";

const DRAVIDIAN = ["telugu", "kannada", "malayalam"] as const;

describe("independent (word-initial) vowels", () => {
  it("Telugu carries the 13 independent vowels, grounded glyph + ISO-15919 roman", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    const iv = telugu.independentVowels!;
    expect(iv.map((v) => v.glyph)).toEqual(
      ["అ", "ఆ", "ఇ", "ఈ", "ఉ", "ఊ", "ఎ", "ఏ", "ఒ", "ఓ", "ఐ", "ఔ", "ఋ"],
    );
    expect(iv.map((v) => v.sound)).toEqual(
      ["a", "ā", "i", "ī", "u", "ū", "e", "ē", "o", "ō", "ai", "au", "r̥"],
    );
    // Independent vowels are vowels, not syllables, and carry no fabricated ductus.
    expect(iv.every((v) => v.role === "vowel")).toBe(true);
    expect(iv.every((v) => v.strokeOrder.length === 0)).toBe(true);
    // The vocalic-R vowel is ISO-15919 r̥ = r + U+0325 (ring below), not IAST ṛ.
    expect([...iv[12]!.sound].map((c) => c.codePointAt(0))).toEqual([0x72, 0x325]);
  });

  it("all three Dravidian scripts carry them", () => {
    DRAVIDIAN.forEach((id) => {
      const s = SCRIPTS.find((x) => x.script === id)!;
      expect(s.independentVowels?.length).toBe(13);
    });
  });

  it("CONTROL: they are SEPARATE from the syllabary — letters, isSyllabary and the matrix are untouched", () => {
    const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
    // None of the independent-vowel glyphs leak into the consonant syllable list…
    const syllableGlyphs = new Set(telugu.letters.map((l) => l.glyph));
    expect(telugu.independentVowels!.some((v) => syllableGlyphs.has(v.glyph))).toBe(false);
    // …so the script still reads as an all-syllable syllabary, and the matrix
    // still builds a full 35 × 13 grid (adding the vowels changed neither).
    expect(isSyllabary(telugu.letters)).toBe(true);
    const m = buildSyllableMatrix(telugu.letters as never)!;
    expect(m.rows.length).toBe(35);
    expect(m.rows.every((r) => r.cells.length === 13)).toBe(true);
  });

  it("an alphabet (Cyrillic) has no independent-vowel list", () => {
    const cyr = SCRIPTS.find((s) => s.script === "cyrillic")!;
    expect(cyr.independentVowels).toBeUndefined();
  });
});
