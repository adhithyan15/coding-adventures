import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Japanese continuity", () => expectLanguageContinuity("japanese"));
it("pins Japanese modality", () => expectLanguageModality("japanese"));

it("pins Japanese's complete pre-A1 writing ramp", () => {
  const japanese = languageWritingStages("japanese");
  expect(japanese.defects).toEqual([]);
  expect(japanese.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(japanese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});
