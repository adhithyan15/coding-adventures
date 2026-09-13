import { expect, it } from "vitest";
import {
  expectLanguageContinuity,
  expectLanguageLessonBudgets,
  expectLanguageModality,
  languageWritingStages,
} from "./assert-language-corpus.js";
it("pins Portuguese continuity", () => expectLanguageContinuity("portuguese"));
it("pins Portuguese modality", () => expectLanguageModality("portuguese"));
it("pins Portuguese lesson-content budgets", () =>
  expectLanguageLessonBudgets("portuguese", {
    //
    // 113 -> 116: chapter 29, the reading rung -- six words, six lines and a
    // 42-word passage. No new word: every token was checked to occur in a
    // lesson with a lower sequence number.
    lessons: 116,
    idioms: 7,
    senses: 7,
    cultureClaims: 11,
    unitPrefix: "PT",
  }));

it("pins Portuguese's current pre-A1 writing foothold", () => {
  const portuguese = languageWritingStages("portuguese");
  expect(portuguese.validEvidence.map((entry) => entry.stage)).toEqual([
    "observe-trace",
    "guided-copy",
  ]);
});
