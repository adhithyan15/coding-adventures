// A Devanagari WORD hangs from one headline, so it is not drawn as its
// letters' strips side by side (that would draw a headline per letter). It
// is one candidate, `composition: "shared-headline"`, which script-ductus
// composes from the cited letters and the font: each letter's body in
// reading order, then ONE headline over the whole word, drawn last because
// most native writers draw it last (HP Labs India's LipiTk 4.0 Devanagari
// recognizer: 82% of 2,706 consonant prototypes; about 5% draw it first).
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
  headlineWordOf,
  HEADLINE_WORD_SCRIPTS,
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

  it("refuses every vowel sign, nasal, visarga, nukta and virama", () => {
    // No Devanagari sign has a cited place against its consonant or the
    // shared headline: ि's is unresolved, ा has no cited ductus, and the
    // virama makes the conjuncts the font fuses.
    for (const word of [
      "मि", // ि
      "नाम", // ा
      "मेरा", // े, ा
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
      "मम नाम", // a phrase: one headline per word
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
    ]);
    expect(candidates.map((target) => [target.lessonId, target.glyph, target.letters, target.composition])).toEqual([
      ["HI-W1", "मम", undefined, "shared-headline"],
      ["HI-W2", "मथ", undefined, "shared-headline"],
      ["HI-W3", "न, म", ["न", "म"], undefined],
    ]);
    // The ledger is the record of what script-ductus could compose: it holds
    // मम, and not मथ (थ's printed headline does not reach its left edge, so
    // one straight headline would cross blank paper).
    const ledger = new Set(["devanagari:मम", "devanagari:न", "devanagari:म"]);
    const drawn = withDerivedFilmstrips([], candidates, (script, glyph) => ledger.has(`${script}:${glyph}`));
    expect(drawn.map((target) => target.lessonId)).toEqual(["HI-W1", "HI-W3"]);
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
    expect(() => assertKnownFigureTarget({ ...target, letters: ["म", "म"] })).toThrow(/composition/);
    expect(() =>
      assertKnownFigureTarget({ ...target, composition: "joined" as unknown as "shared-headline" }),
    ).toThrow(/composition/);
  });
});
