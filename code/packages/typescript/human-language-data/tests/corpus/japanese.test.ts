import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Japanese continuity", () => expectLanguageContinuity("japanese"));
it("pins Japanese modality", () => expectLanguageModality("japanese"));

it("pins Japanese's writing ladder IN ORDER, pre-A1 through A1", () => {
  const japanese = languageWritingStages("japanese");
  expect(japanese.defects).toEqual([]);
  expect(japanese.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(japanese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    // The three A1 stages, proved by the appended writing chapters. ORDER is
    // the point: a timed or connected paper before a controlled one is invalid.
    "controlled-composition",
    "timed-assessment-production",
    "connected-composition",
  ]);
});
