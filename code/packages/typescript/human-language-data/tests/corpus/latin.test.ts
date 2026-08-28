import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Latin continuity", () => expectLanguageContinuity("latin"));
it("pins Latin modality", () => expectLanguageModality("latin"));
it("pins Latin lesson-content budgets", () =>
  expectLanguageLessonBudgets("latin", {
    lessons: 112,
    idioms: 16,
    senses: 6,
    cultureClaims: 17,
    unitPrefix: "LA",
  }));

it("pins Latin's current writing ramp", () => {
  const latin = languageWritingStages("latin");
  expect(latin.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
    "delayed-copy",
    "dictation-transcription",
  ]);
});
