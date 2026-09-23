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

it("pins Telugu's pre-A1 writing ladder", () => {
  const track = languageWritingStages("telugu");

  // The track had NO stage evidence at all -- 49 script lessons and not one
  // writing-stage directive -- so every one of its writing-stage debts read as
  // outstanding while the lessons that could discharge them sat unmarked.
  //
  // The ORDER is asserted rather than the set. `missing-stage-prerequisite`
  // makes a delayed copy invalid unless the tracing and the guided copy come
  // earlier IN SEQUENCE, so a set-equality assertion would pass on a ladder
  // whose rungs are in the wrong order and therefore prove nothing.
  //
  // The fifth rung is a SECOND dictation, roughly 1,150 sequence steps after the
  // first. It is here because the ladder proves the stages are reachable, not
  // that each is practised once: `TE-S170` asks the hand to turn *gau* and
  // *gnya* into shapes with nothing on the page to copy, which is the same
  // stage exercised on a harder pair. A rung may repeat; the ORDER assertion
  // below still forbids one arriving before its prerequisite stages.
  expect(track.validEvidence.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["TE-S01-letter-ta", "observe-trace"],
    ["TE-S01-copy-in-a-word", "guided-copy"],
    ["TE-S01-delayed-copy", "delayed-copy"],
    ["TE-S01-dictation", "dictation-transcription"],
    ["TE-S170-script-dictation-courtesy-letters", "dictation-transcription"],
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
