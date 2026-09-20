import { expect, it } from "vitest";
import { measureContinuity } from "../../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Kannada A1 coverage, and the ordinal point the tranche closed", () => {
  const { lessons } = loadEverything();
  const coverage = measureExamCoverage(loadExamInventory("kannada", "A1"), lessons);
  expect(coverage.enumerated).toBe(258);
  // 194 -> 197: KA-A1-L-12 (the full stop and the comma), KA-A1-L-13 (the
  // question mark) and KA-A1-L-14 (colon, brackets, quotes, dash). THE CORPUS
  // HAD BEEN PRINTING THESE MARKS SINCE CHAPTER ONE while no lesson named any
  // of them -- every Kannada sentence in every reading passage ends in a Latin
  // full stop -- so chapter 77 opens by pointing at the end of a line in the
  // previous chapter and saying that something is sitting there nothing has
  // named. Kannada borrows the whole Latin set, shape and job together, which
  // is why three points cost one short chapter and nothing in it looks
  // unfamiliar.
  // L-13 CARRIES THE ONE LOAD-BEARING CONTRAST: the Spanish demand it derives
  // from opens a question with a second inverted mark and Kannada does not, so
  // a Kannada reader meets the mark at the end or not at all and the WORDS have
  // to carry the question until then.
  // L-14 IS PROBED AS A RECOGNITION POINT, not a production one: at A1 the
  // demand is knowing what a colon or a bracket signals on a notice, and the
  // recall lesson sorts the set into the three a reader writes and the rest
  // they read. KA-A1-L-15 (abbreviations and symbols) stays open.
  // FOUR OF KANNADA'S UNMAPPED POINTS ARE STRUCTURALLY UNCOVERABLE and are
  // marked untransferable in the inventory: capital letters, written
  // accentuation and superscript abbreviation letters have no Kannada
  // counterpart at all, and neither does Spanish's mid-distance demonstrative.
  // The real ceiling for this track is 254/258, not 258/258.
  // 197 -> 198: KA-A1-L-09, THE SCRIPT CLOSED. Its label used to be a status
  // rather than a demand -- "THE SCRIPT IS NOT CLOSED: N characters are used but
  // never taught" -- with N going 27, 19, 13 as the tranches landed. Chapters 79
  // and 80 take it to zero and the label is now the thing the point asks for.
  // ITS PROBE IS THE TWENTY-SEVEN IT WAS OPENED FOR: the eight chapters 67-73
  // taught, the six chapter 78 taught as writing, and the thirteen chapters 79
  // and 80 taught as recognition. The point reopens if any one loses its lesson.
  // THE SPLIT BETWEEN WRITING AND RECOGNITION IS ABOUT SOURCES, NOT DIFFICULTY.
  // Chapter 78's six carry cited Wikimedia Commons stroke-order animations.
  // None of these thirteen has a sourced ductus anywhere in this project, so
  // every one keeps the standing refusal to state a pen path without a source.
  // THIRTEEN ATOMS IN ONE CHAPTER WAS THE FIRST DRAFT AND IT WAS WRONG: the
  // per-chapter budget in core/chapter-policy.json is twelve, and the snapshot
  // diff showed atomChapterSpikes 3 -> 4. Splitting into chapter 79 (the three
  // vowel signs plus nna, sha and ssa) and chapter 80 (the seven breathy
  // consonants) removed the spike and put one idea in each chapter.
  expect(coverage.covered).toBe(198);
  expect(coverage.unmapped).toBe(60);
  expect(coverage.partial).toBe(0);
  // KA-A1-NUM-07 was one of the thirteen ordinal points HL-C354 left open, and
  // the one it priced cheapest: Kannada's -aneya has no exceptions, so ten
  // ordinals follow from one ending on cardinals chapter 7 already taught. The
  // eighth point in this category is KA-A1-NUM-08 (measures), which is a
  // vocabulary absence rather than an ordinal one.
  expect(coverage.byCategory["Sankhye (numerals and quantity)"]!).toEqual({
    enumerated: 8,
    covered: 7,
  });
  expect(formatExamCoverage(coverage)).toContain(
    "kannada A1 (partial inventory): 198/258 points covered (77%)",
  );
}, 60_000);
