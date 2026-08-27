import { expect } from "vitest";
import type { GlyphEvidence } from "./types";

export default [
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 10,
    name: "Telugu carries the 13 independent vowels, grounded glyph + ISO-15919 roman",
    verify: ({ SCRIPTS }) => {
      const telugu = SCRIPTS.find((s) => s.script === "telugu")!;
      const iv = telugu.independentVowels!;
      expect(iv.map((v) => v.glyph)).toEqual(
        ["అ", "ఆ", "ఇ", "ఈ", "ఉ", "ఊ", "ఎ", "ఏ", "ఒ", "ఓ", "ఐ", "ఔ", "ఋ"],
      );
      expect(iv.map((v) => v.sound)).toEqual(
        ["a", "ā", "i", "ī", "u", "ū", "e", "ē", "o", "ō", "ai", "au", "r̥"],
      );
      // Independent vowels are vowels, not syllables. అ, ఆ, ఇ, ఉ, and ఎ have crossed
      // the source-and-font gate; every other row remains free of fabricated ductus.
      expect(iv.every((v) => v.role === "vowel")).toBe(true);
      expect(iv[0]!.strokeOrder).toHaveLength(4);
      expect(iv[0]!.penLifts).toBe(1);
      expect(iv[0]!.strokeOrderSource?.url).toBe(
        "https://write-telugu-alphabets.en.aptoide.com/app",
      );
      expect(iv[1]!.strokeOrder).toHaveLength(2);
      expect(iv[1]!.penLifts).toBe(1);
      expect(iv[1]!.strokeOrderSource?.url).toBe(
        "https://www.youtube.com/watch?v=vXdrj1pP6q0",
      );
      expect(iv[2]!.strokeOrder).toHaveLength(3);
      expect(iv[2]!.penLifts).toBe(2);
      expect(iv[2]!.strokeOrderSource?.url).toBe(
        "https://www.youtube.com/watch?v=MKvmq1hFVIE",
      );
      expect(iv[4]!.strokeOrder).toHaveLength(5);
      expect(iv[4]!.penLifts).toBe(2);
      expect(iv[4]!.strokeOrderSource?.citation).toMatch(/dot_stroke_v_5_u\.png.*movements 1–5.*version 2\.6/i);
      expect(iv[6]!.strokeOrder).toHaveLength(3);
      expect(iv[6]!.penLifts).toBe(1);
      expect(iv[6]!.strokeOrderSource?.url).toBe(
        "https://write-telugu-alphabets.en.aptoide.com/app",
      );
      expect(iv[7]!.strokeOrder).toHaveLength(4);
      expect(iv[7]!.penLifts).toBe(2);
      expect(iv[7]!.strokeOrderSource?.citation).toMatch(/dot_stroke_v_10_ae\.png.*movements 1–4.*version 2\.6/i);
      expect(iv.filter((_, index) => ![0, 1, 2, 4, 6, 7].includes(index)).every((v) => v.strokeOrder.length === 0)).toBe(true);
      // The vocalic-R vowel is ISO-15919 r̥ = r + U+0325 (ring below), not IAST ṛ.
      expect([...iv[12]!.sound].map((c) => c.codePointAt(0))).toEqual([0x72, 0x325]);
    },
  },
  {
    suite: "independent (word-initial) vowels",
    suiteOrder: 10,
    caseOrder: 50,
    name: "CONTROL: they are SEPARATE from the syllabary — letters, isSyllabary and the matrix are untouched",
    verify: ({ SCRIPTS, isSyllabary, buildSyllableMatrix }) => {
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
    },
  },
] satisfies readonly GlyphEvidence[];


