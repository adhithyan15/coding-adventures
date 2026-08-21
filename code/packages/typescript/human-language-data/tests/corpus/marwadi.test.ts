import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Marwadi continuity", () => expectLanguageContinuity("marwadi"));
it("pins Marwadi modality", () => expectLanguageModality("marwadi"));
it("pins Marwadi's complete pre-A1 writing ramp", () => {
  const marwadi = languageWritingStages("marwadi");
  expect(marwadi.defects).toEqual([]);
  expect(marwadi.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(marwadi.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
    "observe-trace",
    "observe-trace",
    "delayed-copy",
    "dictation-transcription",
  ]);
});
