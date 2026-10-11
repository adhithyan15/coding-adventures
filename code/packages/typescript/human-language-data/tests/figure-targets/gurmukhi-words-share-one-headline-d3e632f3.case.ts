// A Gurmukhi WORD hangs from one headline (ਸਿਰੋਰੇਖਾ), like a Devanagari one,
// so it is not its letters' strips side by side: each cited letter (GNPS's
// tracing lesson) draws its own headline FIRST, and a word drawn letter by
// letter would show one headline per letter, each before its body.
//
// It is one candidate, `composition: "shared-headline"`, which script-ductus
// composes from the cited letters and the font: each letter's body in reading
// order, then ONE headline, last, left to right. That order is this book's
// convention for words (the order fluent writers are described as using, and
// the one it draws Devanagari words in), not something a Gurmukhi source
// records; a single letter's own strip keeps GNPS's headline-first order.
//
// The rule is Devanagari's with nothing added and no Gurmukhi sign: one word
// of two or more bare base letters. These cases pin what it accepts and what
// it refuses; the real-corpus case pins the lessons it unlocks.
import { describe, expect, it } from "vitest";
import {
  filmstripCandidates,
  filmstripImageMarkdown,
  headlinePhraseOf,
  headlineWordOf,
  HEADLINE_WORD_SCRIPTS,
  HEADLINE_WORD_SIGNS,
  HEADLINE_WORD_SPLIT_LETTER_SOURCES,
  SEPARATE_LETTER_SCRIPTS,
  withDerivedFilmstrips,
  writingSequenceOf,
} from "../../src/figure-targets.js";
import { lesson } from "./fixture.js";

const punjabi = (headword: string, options: { type?: string; blocks?: string[] } = {}) =>
  lesson("PA-W1", { language: "punjabi", headword, ...options });

describe("Gurmukhi words share one headline", () => {
  it("is a headline-word script, not a separate-letter one, with no sign and no split letter", () => {
    expect(HEADLINE_WORD_SCRIPTS.gurmukhi!.test("ਮ")).toBe(true);
    expect(HEADLINE_WORD_SCRIPTS.gurmukhi!.test("म")).toBe(false);
    expect(SEPARATE_LETTER_SCRIPTS.has("gurmukhi")).toBe(false);
    // No Gurmukhi sign has a cited ductus yet, so a word holds none.
    expect(HEADLINE_WORD_SIGNS.gurmukhi).toBeUndefined();
    // Noto Sans Gurmukhi changes no run of bare letters while shaping.
    expect(HEADLINE_WORD_SPLIT_LETTER_SOURCES.gurmukhi).toBeUndefined();
  });

  it("takes a word of bare letters as ONE word, never as a sequence of letters", () => {
    for (const word of ["ਪਰ", "ਅਮਨ", "ਮਨਨ", "ਨਪਨ", "ਉਹ"]) {
      expect(headlineWordOf(punjabi(word), "gurmukhi"), word).toBe(word);
      expect(writingSequenceOf(punjabi(word), "gurmukhi"), word).toBeUndefined();
    }
  });

  it("refuses every vowel sign, bindi, tippi, addak, halant and nukta", () => {
    for (const word of [
      "ਕਿ", // ਿ
      "ਨਾ", // ਾ
      "ਨਹੀਂ", // ੀ, ਂ
      "ਅਤੇ", // ੇ
      "ਕੰਮ", // ੰ
      "ਪੱਕਾ", // ੱ, ਾ
      "ਸ੍ਰੀ", // ੍, ੀ
      "ਸ਼", // ਸ + ਼ typed apart
      "ਸ਼ਰ", // ਸ਼ precomposed: NFD is ਸ + ਼
    ]) {
      expect(headlineWordOf(punjabi(word), "gurmukhi"), word).toBeUndefined();
    }
  });

  it("refuses what is not one word of two or more Gurmukhi letters", () => {
    for (const word of ["ਮ", "ਪਰ, ਮਨ", "ਪਰ।", "ਪਰ:", "੧੫", "ਪA", ""]) {
      expect(headlineWordOf(punjabi(word), "gurmukhi"), word).toBeUndefined();
    }
    expect(headlineWordOf(punjabi("ਪਰ", { type: "vocabulary" }), "gurmukhi")).toBeUndefined();
    expect(headlineWordOf(punjabi("ਪਰ", { blocks: ["Warm-up", "Wrap-up Recall"] }), "gurmukhi")).toBeUndefined();
  });

  it("takes a phrase of such words word by word, as in Devanagari", () => {
    expect(headlinePhraseOf(punjabi("ਅਮਨ ਮਨਨ"), "gurmukhi")).toEqual(["ਅਮਨ", "ਮਨਨ"]);
    expect(headlinePhraseOf(punjabi("ਨਮਸਤੇ ਮਨਨ"), "gurmukhi")).toBeUndefined(); // ੇ
    expect(headlinePhraseOf(punjabi("ਨਮਸਤੇ ਮਨਨ।"), "gurmukhi")).toBeUndefined();
  });

  it("makes the word one candidate, drawn only when the ledger holds the composed word", () => {
    const candidates = filmstripCandidates([
      lesson("PA-W1", { language: "punjabi", headword: "ਪਰ" }),
      lesson("PA-W2", { language: "punjabi", headword: "ਨਪਨ" }),
      lesson("PA-W3", { language: "punjabi", headword: "ਨ, ਮ" }),
      lesson("PA-W4", { language: "punjabi", headword: "ਕਿ" }),
    ]);
    expect(candidates.map((target) => [target.lessonId, target.glyph, target.letters, target.composition])).toEqual([
      ["PA-W1", "ਪਰ", undefined, "shared-headline"],
      ["PA-W2", "ਨਪਨ", undefined, "shared-headline"],
      ["PA-W3", "ਨ, ਮ", ["ਨ", "ਮ"], undefined],
      // ਕਿ is one grapheme: a letter candidate, drawn once ਿ is cited.
      ["PA-W4", "ਕਿ", undefined, undefined],
    ]);
    // The ledger is the record of what script-ductus composed: ਪਰ, and not
    // ਨਪਨ (one straight headline would cross ਪ's split bar).
    const ledger = new Set(["gurmukhi:ਪਰ", "gurmukhi:ਨ", "gurmukhi:ਮ"]);
    const drawn = withDerivedFilmstrips([], candidates, (script, glyph) => ledger.has(`${script}:${glyph}`));
    expect(drawn.map((target) => target.lessonId)).toEqual(["PA-W1", "PA-W3"]);
    expect(filmstripImageMarkdown(candidates[0]!)).toBe(
      "![How ਪਰ is written, letter by letter, then one headline, stroke by stroke](figures/PA-W1-filmstrip.svg)",
    );
  });
});
