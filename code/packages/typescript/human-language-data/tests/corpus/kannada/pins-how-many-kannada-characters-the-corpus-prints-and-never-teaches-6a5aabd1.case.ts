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
