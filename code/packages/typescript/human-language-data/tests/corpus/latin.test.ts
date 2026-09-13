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
    //
    // 160 -> 163: chapter 58, the reading rung -- six words, six lines and a
    // 42-word passage. No new word: every token was checked to occur in a
    // lesson with a lower sequence number, which for Latin means the exact
    // INFLECTED form, not the lemma.
    lessons: 163,
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
