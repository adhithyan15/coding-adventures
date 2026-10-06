import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "../assert-language-corpus.js";

it("pins Bengali's writing ladder through its first A1 controlled composition", () => {
  const track = languageWritingStages("bengali");

  // The track had NO stage evidence at all, so every writing-stage debt read as
  // outstanding while the lessons that could discharge them sat unmarked.
  //
  // The ORDER is asserted rather than the set: `missing-stage-prerequisite`
  // makes a delayed copy invalid unless the tracing and the guided copy come
  // earlier IN SEQUENCE, so a set-equality assertion would pass on a ladder
  // whose rungs are in the wrong order and therefore prove nothing.
  expect(track.validEvidence.map((entry) => [entry.lessonId, entry.stage])).toEqual([
    ["BN-W01-na-trace", "observe-trace"],
    ["BN-W01-na-guided-copy", "guided-copy"],
    ["BN-W01-na-delayed-copy", "delayed-copy"],
    ["BN-W01-na-dictation", "dictation-transcription"],
    ["BN-W26-kothay-controlled-question", "controlled-composition"],
    ["BN-W40-a1-timed-production", "timed-assessment-production"],
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
