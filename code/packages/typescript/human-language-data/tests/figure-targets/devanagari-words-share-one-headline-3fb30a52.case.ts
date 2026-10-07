// A Devanagari WORD hangs from one headline, so it is not drawn as its
// letters' strips side by side (that would draw a headline per letter). It
// is one candidate, `composition: "shared-headline"`, which script-ductus
// composes from the cited letters and the font: each letter's body in
// reading order, then ONE headline over the whole word, drawn last because
// most native writers draw it last (HP Labs India's LipiTk 4.0 Devanagari
// recognizer: 82% of 2,706 consonant prototypes; about 5% draw it first).
//
// One sign may join: ā (ा), straight after a consonant, its place cited on
// its mark record (the cited आ draws the same bar after the body and before
// the headline). A phrase of such words, separated by single spaces and
// nothing else, is drawn word by word, each word with its own headline.
//
// This module only picks the headwords worth composing, from the text. These
// cases pin what it accepts and every reason it refuses.
import { readFileSync } from "node:fs";
import { join } from "node:path";
import { describe, expect, it } from "vitest";
import { assertKnownFigureTarget } from "../../src/figure-cli.js";
import {
  filmstripCandidates,
  filmstripImageMarkdown,
  headlinePhraseOf,
  headlineWordOf,
  HEADLINE_WORD_SCRIPTS,
  HEADLINE_WORD_SIGNS,
  MAX_PHRASE_WORDS,
  MAX_SEQUENCE_PIECES,
  HEADLINE_WORD_SPLIT_LETTER_SOURCES,
  withDerivedFilmstrips,
  writingSequenceOf,
} from "../../src/figure-targets.js";
import type { ScriptFilmstripTarget } from "../../src/figure.js";
import { defaultCurriculumRoot } from "../../src/loader.js";
import { lesson } from "./fixture.js";

const sanskrit = (headword: string, options: { type?: string; blocks?: string[] } = {}) =>
  lesson("SA-W1", { language: "sanskrit", headword, ...options });

describe("Devanagari words share one headline", () => {
  it("takes a word of bare letters as ONE word, never as a sequence of letters", () => {
    for (const word of ["मम", "नमक", "कमल", "थम"]) {
      expect(headlineWordOf(sanskrit(word), "devanagari"), word).toBe(word);
      // The letter-by-letter path still refuses it: that would draw a
      // headline per letter.
      expect(writingSequenceOf(sanskrit(word), "devanagari"), word).toBeUndefined();
    }
  });

  it("takes the ā sign straight after a consonant, and nowhere else", () => {
    expect([...HEADLINE_WORD_SIGNS.devanagari!]).toEqual(["\u093E"]);
    for (const word of ["नाम", "सा", "नामा", "कमला"]) {
      expect(headlineWordOf(sanskrit(word), "devanagari"), word).toBe(word);
    }
    // First, doubled, or after a vowel letter: no consonant before it.
    for (const word of ["\u093E\u092E", "\u0928\u093E\u093E", "\u0905\u093E\u092E", "\u0907\u093E"]) {
      expect(headlineWordOf(sanskrit(word), "devanagari"), word).toBeUndefined();
    }
  });

  it("refuses every other vowel sign, nasal, visarga, nukta and virama", () => {
    // None has a cited place against its consonant AND the shared headline:
    // ि's side is unresolved, े's flag splits writers, ो ौ ी have no ductus
    // or no majority, and the virama makes the conjuncts the font fuses.
    for (const word of [
      "मि", // ि
      "मेरा", // े, ा
      "हो", // ो
      "कुल", // ु
      "संत", // ं
      "हँस", // ँ
      "नमः", // ः
      "नमस्ते", // ्, conjunct
      "क़म", // क़ precomposed: NFD is क + ़
      "क़म", // क + ़ typed apart
    ]) {
      expect(headlineWordOf(sanskrit(word), "devanagari"), word).toBeUndefined();
    }
  });

  it("refuses the letters the bundled font splits while shaping", () => {
    expect(HEADLINE_WORD_SPLIT_LETTER_SOURCES.devanagari!.flatMap((source) => source.letters)).toEqual([
      "ई",
      "ऐ",
    ]);
    for (const word of ["मई", "ऐम"]) {
      expect(headlineWordOf(sanskrit(word), "devanagari"), word).toBeUndefined();
    }
    // The same letters elsewhere (इ, ए) are not split.
    expect(headlineWordOf(sanskrit("इस"), "devanagari")).toBe("इस");
    expect(headlineWordOf(sanskrit("एक"), "devanagari")).toBe("एक");
  });

  it("cites the Devanagari font that is actually bundled", () => {
    // The GSUB lookup was read from this file. If the font is replaced, its
    // version string changes and the citation must be read again.
    const font = readFileSync(join(defaultCurriculumRoot(), "_fonts", "NotoSansDevanagari-Static.ttf"));
    for (const source of HEADLINE_WORD_SPLIT_LETTER_SOURCES.devanagari!) {
      expect(source.citation).toContain(
        "Noto Sans Devanagari Version 2.006, bundled as learning/human-languages/_fonts/NotoSansDevanagari-Static.ttf",
      );
      expect(source.url).toMatch(/^https:\/\//);
    }
    const version = Buffer.from("Version 2.006", "utf16le").swap16();
    expect(font.includes(version)).toBe(true);
  });

  it("refuses what is not one word of two or more Devanagari letters", () => {
    for (const word of [
      "म", // one letter: a letter strip
      "मम नाम", // a phrase: one headline per word (`headlinePhraseOf`)
      "न, म", // a list: letters drawn one by one
      "न—म",
      "म१", // a digit
      "मम।", // a danda
      "मA", // a Latin letter
      "",
    ]) {
      expect(headlineWordOf(sanskrit(word), "devanagari"), word).toBeUndefined();
    }
  });

  it("asks only of writing lessons with a block to land in, on a headline script", () => {
    expect(headlineWordOf(sanskrit("मम", { type: "vocabulary" }), "devanagari")).toBeUndefined();
    expect(headlineWordOf(sanskrit("मम", { blocks: ["Warm-up", "Wrap-up Recall"] }), "devanagari")).toBeUndefined();
    expect(Object.keys(HEADLINE_WORD_SCRIPTS)).toEqual(["devanagari"]);
    // Tamil letters stand apart; Gurmukhi's cited source draws the headline first.
    expect(headlineWordOf(lesson("TA-W1", { headword: "மம" }), "tamil")).toBeUndefined();
    expect(headlineWordOf(lesson("PA-W1", { language: "punjabi", headword: "ਕਮ" }), "gurmukhi")).toBeUndefined();
  });

  it("makes the word one candidate, drawn only when the ledger holds the composed word", () => {
    const candidates = filmstripCandidates([
      lesson("HI-W1", { language: "hindi", headword: "मम" }),
      lesson("HI-W2", { language: "hindi", headword: "मथ" }),
      lesson("HI-W3", { language: "hindi", headword: "न, म" }),
      lesson("HI-W4", { language: "hindi", headword: "नाम" }),
      lesson("HI-W5", { language: "hindi", headword: "मेरा" }),
    ]);
    expect(candidates.map((target) => [target.lessonId, target.glyph, target.letters, target.composition])).toEqual([
      ["HI-W1", "मम", undefined, "shared-headline"],
      ["HI-W2", "मथ", undefined, "shared-headline"],
      ["HI-W3", "न, म", ["न", "म"], undefined],
      ["HI-W4", "नाम", undefined, "shared-headline"],
    ]);
    // The ledger is the record of what script-ductus could compose: it holds
    // मम and नाम, and not मथ (थ's printed headline does not reach its left
    // edge, so one straight headline would cross blank paper).
    const ledger = new Set(["devanagari:मम", "devanagari:नाम", "devanagari:न", "devanagari:म"]);
    const drawn = withDerivedFilmstrips([], candidates, (script, glyph) => ledger.has(`${script}:${glyph}`));
    expect(drawn.map((target) => target.lessonId)).toEqual(["HI-W1", "HI-W3", "HI-W4"]);
  });

  it("takes a phrase of such words, separated by single spaces, word by word", () => {
    expect(headlinePhraseOf(sanskrit("मम नाम"), "devanagari")).toEqual(["मम", "नाम"]);
    // A one-letter word is drawn as that letter's own strip.
    expect(headlinePhraseOf(sanskrit("न मम"), "devanagari")).toEqual(["न", "मम"]);
    // Three words is the cap; ten pieces too.
    expect(MAX_PHRASE_WORDS).toBe(3);
    expect(headlinePhraseOf(sanskrit("मम नाम मम"), "devanagari")).toEqual(["मम", "नाम", "मम"]);
    expect(headlinePhraseOf(sanskrit("मम नाम मम मम"), "devanagari")).toBeUndefined();
    expect(MAX_SEQUENCE_PIECES).toBe(10);
    expect(headlinePhraseOf(sanskrit("कमला नमक कमला"), "devanagari")).toBeUndefined(); // 11 pieces
    expect(headlinePhraseOf(sanskrit("कमल नमक कमला"), "devanagari")).toEqual(["कमल", "नमक", "कमला"]); // 10
  });

  it("refuses a phrase with punctuation, other separators, an unplaced sign or only letters", () => {
    for (const headword of [
      "नाम: मीरा", // a label: no source draws the colon
      "नमस्कार मीरा.", // a sentence, and a conjunct
      "मम नाम।", // a danda
      "मम, नाम", // a list separator
      "मम — नाम",
      "मम · नाम",
      "मम  नाम", // two spaces
      "मम\u00a0नाम", // a no-break space
      "मेरा नाम", // े has no cited place
      "पाछे मिलसू", // े and ू
      "न म", // every item one letter: a list, drawn letter by letter
      "मम",
    ]) {
      expect(headlinePhraseOf(sanskrit(headword), "devanagari"), headword).toBeUndefined();
    }
    expect(headlinePhraseOf(lesson("TA-W1", { headword: "மம நம" }), "tamil")).toBeUndefined();
    expect(headlinePhraseOf(sanskrit("मम नाम", { type: "vocabulary" }), "devanagari")).toBeUndefined();
  });

  it("makes a phrase one candidate whose letters are its words, drawn only when every word is in the ledger", () => {
    const candidates = filmstripCandidates([
      lesson("SA-W1", { language: "sanskrit", headword: "मम नाम" }),
      lesson("SA-W2", { language: "sanskrit", headword: "मम मथ" }),
    ]);
    expect(candidates.map((target) => [target.lessonId, target.glyph, target.letters, target.composition])).toEqual([
      ["SA-W1", "मम नाम", ["मम", "नाम"], "shared-headline"],
      ["SA-W2", "मम मथ", ["मम", "मथ"], "shared-headline"],
    ]);
    const ledger = new Set(["devanagari:मम", "devanagari:नाम"]);
    const drawn = withDerivedFilmstrips([], candidates, (script, glyph) => ledger.has(`${script}:${glyph}`));
    expect(drawn.map((target) => target.lessonId)).toEqual(["SA-W1"]);
    expect(filmstripImageMarkdown(candidates[0]!)).toBe(
      "![How मम नाम is written, word by word, each with its own headline, stroke by stroke](figures/SA-W1-filmstrip.svg)",
    );
    expect(() => assertKnownFigureTarget(candidates[0]!)).not.toThrow();
    // The words must spell the headword back, one space apart.
    expect(() => assertKnownFigureTarget({ ...candidates[0]!, letters: ["नाम", "मम"] })).toThrow(/spell its glyph/);
  });

  it("captions the word by its letters and its one headline", () => {
    const target: ScriptFilmstripTarget = {
      kind: "script-filmstrip",
      lessonId: "SA-W1",
      script: "devanagari",
      glyph: "मम",
      composition: "shared-headline",
      output: "sanskrit/book/figures/SA-W1-filmstrip.svg",
    };
    expect(filmstripImageMarkdown(target)).toBe(
      "![How मम is written, letter by letter, then one headline, stroke by stroke](figures/SA-W1-filmstrip.svg)",
    );
    expect(() => assertKnownFigureTarget(target)).not.toThrow();
    // With letters it is a phrase, whose words must spell the glyph back.
    expect(() => assertKnownFigureTarget({ ...target, letters: ["म", "म"] })).toThrow(/spell its glyph/);
    expect(() =>
      assertKnownFigureTarget({ ...target, composition: "joined" as unknown as "shared-headline" }),
    ).toThrow(/composition/);
  });
});
