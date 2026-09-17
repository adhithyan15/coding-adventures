import { expect, it } from "vitest";
import { measureContinuity } from "../../src/continuity.js";
import {
  defaultCurriculumRoot,
  loadEverything,
  loadExamInventory,
  loadTrackLessons,
} from "../../src/loader.js";
import {
  formatExamCoverage,
  measureExamCoverage,
  trackIntroducedAtoms,
} from "../../src/exam-inventory.js";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Kannada continuity", () => expectLanguageContinuity("kannada"));
it("pins Kannada modality", () => expectLanguageModality("kannada"));
it("keeps Kannada's opening free of future farewells and pronouns", () => {
  const references = measureContinuity(
    loadTrackLessons("kannada", defaultCurriculumRoot()),
  ).forwardReferences;
  expect(references.length).toBeLessThanOrEqual(15);
  expect(references.filter((reference) => /-C0[12]-/.test(reference.lessonId))).toEqual([]);
});

// ---------------------------------------------------------------------------
// THE KANNADA A1 INVENTORY HAD NO ASSERTION AT ALL -- the same hole HL-C354
// found in Telugu and Hindi, and the one it told the next reader to go looking
// for in the other eighteen. Without these two tests the ordinal tranche could
// land its atoms, wire KA-A1-NUM-07's probe, and leave a coverage number that
// nothing reads. Both halves were falsified before being kept: a fabricated
// atom id in the probe fails the first, and nulling KA-A1-NUM-07's probe fails
// the second.
// ---------------------------------------------------------------------------
it("probes only Kannada atoms that EXIST, so a guessed id cannot under-report", () => {
  const { lessons } = loadEverything();
  const taught = trackIntroducedAtoms(lessons, "kannada");
  const unknown: string[] = [];
  for (const point of loadExamInventory("kannada", "A1").points) {
    for (const atom of point.probe ?? []) if (!taught.has(atom)) unknown.push(`${point.id}:${atom}`);
  }
  expect(unknown).toEqual([]);
}, 60_000);

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

it("pins Kannada's pre-A1 writing ladder", () => {
  const track = languageWritingStages("kannada");

  // The track had NO stage evidence at all -- 50 script lessons and not one
  // writing-stage directive -- so every one of its writing-stage debts read as
  // outstanding while the lessons that could discharge them sat unmarked.
  //
  // The ORDER is asserted rather than the set. `missing-stage-prerequisite`
  // makes a delayed copy invalid unless the tracing and the guided copy come
  // earlier IN SEQUENCE, so a set-equality assertion would pass on a ladder
  // whose rungs are in the wrong order and therefore prove nothing.
  expect(track.validEvidence.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["KA-S01-letter-na", "observe-trace"],
    ["KA-S01-copy-in-a-word", "guided-copy"],
    ["KA-S01-delayed-copy", "delayed-copy"],
    ["KA-S01-dictation", "dictation-transcription"],
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});

// ---------------------------------------------------------------------------
// ZERO, AND PINNED AS ZERO RATHER THAN RE-PINNED TO A SMALLER COUNT.
//
// This number was 27 when `KA-A1-L-09` was first measured, 19 after chapters
// 67-73, 13 after chapter 78, and chapters 79 and 80 take it to nothing: every
// Kannada character this corpus prints now has a lesson.
//
// An exact zero, not a ceiling. A single new violation is a lesson asking the
// reader to decode something nobody taught, and there is no longer a backlog
// for it to hide inside. That case is not hypothetical — a draft of
// `KA-S163-vowel-sign-au` mentioned the independent vowel au in passing, and
// that ONE printed glyph was the only thing standing between this track and
// zero, because the letter has no sourced stroke order and is taught nowhere.
// The prose names the letter instead of printing it.
//
// `measureScriptClosure` CANNOT BE USED FOR THIS. It credits a glyph to any
// script lesson whose BODY contains it (HL-C383), which for this track reports
// a closure the corpus does not have. The rule applied below is HL-C386's: a
// glyph counts as taught only when its lesson's HEADWORD is a GLYPH INVENTORY —
// every whitespace- or middot-separated token a base plus at most one combining
// mark — so a headword that happens to be a whole word teaches nothing.
// ---------------------------------------------------------------------------
it("pins how many Kannada characters the corpus prints and never teaches", () => {
  const lessons = loadTrackLessons("kannada", defaultCurriculumRoot());
  const isInventory = (headword: string) =>
    headword
      .replace(/◌/g, "")
      .trim()
      .split(/[\s/·]+/)
      .filter(Boolean)
      .every((token) => [...token].length <= 2);

  const taught = new Set<string>();
  for (const lesson of lessons) {
    const headword = lesson.frontmatter.headword ?? "";
    if (!headword || !isInventory(headword)) continue;
    for (const glyph of headword.replace(/◌/g, "")) taught.add(glyph);
  }

  const used = new Set<string>();
  for (const lesson of lessons) {
    for (const glyph of lesson.body) {
      const code = glyph.codePointAt(0)!;
      if (code >= 0x0c80 && code <= 0x0cff) used.add(glyph);
    }
  }

  const untaught = [...used].filter((glyph) => !taught.has(glyph));
  expect(untaught).toEqual([]);
});
