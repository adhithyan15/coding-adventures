import { describe, expect, it } from "vitest";
import {
  DERIVED_FILMSTRIP_SCRIPTS,
  filmstripCandidates,
  writingLetterOf,
} from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

describe("which lessons are filmstrip candidates", () => {
  it("takes a one-letter writing lesson with a Writing block", () => {
    expect(writingLetterOf(lesson("TA-S1"))).toBe("அ");
  });

  it("counts a letter as one grapheme, so a consonant with its vowel sign is still one", () => {
    expect(writingLetterOf(lesson("TA-S2", { headword: "கா" }))).toBe("கா");
  });

  it("leaves several letters, a word, a non-writing lesson or a missing Writing block alone", () => {
    expect(writingLetterOf(lesson("TA-S3", { headword: "வ, க" }))).toBeUndefined();
    expect(writingLetterOf(lesson("TA-S4", { headword: "வணக்கம்" }))).toBeUndefined();
    expect(writingLetterOf(lesson("TA-C5", { type: "word" }))).toBeUndefined();
    expect(writingLetterOf(lesson("TA-S6", { blocks: ["Warm-up", "Guided Practice"] }))).toBeUndefined();
  });

  it("only draws candidates from switched-on tracks, into that track's own book", () => {
    const candidates = filmstripCandidates([
      lesson("TA-S1"),
      // Punjabi stands in for a track that is still switched off. (This was
      // Bengali until Bengali's first cited letters switched it on.)
      lesson("PA-S1", { language: "punjabi", headword: "ਅ" }),
    ]);
    expect(candidates).toEqual([
      {
        kind: "script-filmstrip",
        lessonId: "TA-S1",
        script: "tamil",
        glyph: "அ",
        output: "tamil/book/figures/TA-S1-filmstrip.svg",
      },
    ]);
    expect(Object.keys(DERIVED_FILMSTRIP_SCRIPTS)).toContain("tamil");
  });
});
