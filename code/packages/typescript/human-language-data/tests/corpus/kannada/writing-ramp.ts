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
