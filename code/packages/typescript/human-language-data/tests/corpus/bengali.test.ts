import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Bengali continuity", () => expectLanguageContinuity("bengali"));
it("pins Bengali modality", () => expectLanguageModality("bengali"));

it("pins Bengali's pre-A1 writing ladder", () => {
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
  ]);
  expect(track.defects).toEqual([]);
  expect(track.levels[0]).toMatchObject({
    level: "pre-A1",
    missingStages: [],
    complete: true,
  });
});
