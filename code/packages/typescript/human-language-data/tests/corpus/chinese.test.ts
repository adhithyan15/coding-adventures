import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Chinese continuity", () => expectLanguageContinuity("chinese"));
it("pins Chinese modality", () => expectLanguageModality("chinese"));

it("pins Chinese's complete pre-A1 writing ramp", () => {
  const chinese = languageWritingStages("chinese");
  expect(chinese.defects).toEqual([]);
  expect(chinese.levels[0]).toMatchObject({ level: "pre-A1", complete: true, missingStages: [] });
  expect(chinese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});
